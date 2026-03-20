#!/usr/bin/env python3
"""
Import Schwab JSON transaction export directly into theta-vault-rust SQLite database.

Usage:
    python3 scripts/import_schwab_rust.py /path/to/schwab_export.json [trades.db] [--delete-existing]

Arguments:
    json_path           Path to Schwab JSON export (BrokerageTransactions format)
    db_path             SQLite database path (default: trades.db)
    --delete-existing   Delete non-protected trades before importing

The script:
1. Parses Schwab's JSON transaction format
2. Groups individual legs into complete trades (spreads, CSPs, CCs, ICs, calendars)
3. Matches opens with closes/expirations
4. Imports directly into SQLite (no server required)
5. Estimates Greeks using Black-Scholes with Yahoo Finance historical prices
6. Detects and links rolled trades
"""

import json
import math
import re
import sys
import sqlite3
import urllib.request
import urllib.error
from datetime import datetime, timedelta
from collections import defaultdict
from typing import List, Dict, Any, Optional, Tuple


# ---------- Configuration ----------

# Maximum calendar days between a put vertical and call vertical to merge them into an IC.
# Raise this if you intentionally leg into ICs over more than 14 days.
IC_MERGE_MAX_DAYS: int = 14

# ---------- Yahoo Finance Historical Prices ----------

_price_cache: Dict[str, Dict[str, float]] = {}  # ticker -> {date_str -> close_price}


def fetch_historical_prices(ticker: str, start_date: str, end_date: str) -> Dict[str, float]:
    """Fetch historical daily close prices from Yahoo Finance. Returns {YYYY-MM-DD: close}."""
    cache_key = ticker
    if cache_key in _price_cache:
        return _price_cache[cache_key]

    # Yahoo Finance chart API (v8) — no auth needed
    # Convert ^VIX to %5EVIX for URL encoding
    url_ticker = ticker.replace('^', '%5E')
    start_ts = int(datetime.strptime(start_date, '%Y-%m-%d').timestamp())
    end_ts = int((datetime.strptime(end_date, '%Y-%m-%d') + timedelta(days=1)).timestamp())

    prices: Dict[str, float] = {}
    for host in ('query1.finance.yahoo.com', 'query2.finance.yahoo.com'):
        url = f"https://{host}/v8/finance/chart/{url_ticker}?period1={start_ts}&period2={end_ts}&interval=1d"
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode())
                result = data['chart']['result'][0]
                timestamps = result['timestamp']
                closes = result['indicators']['quote'][0]['close']
                for ts, close in zip(timestamps, closes):
                    if close is not None:
                        dt = datetime.utcfromtimestamp(ts).strftime('%Y-%m-%d')
                        prices[dt] = round(close, 2)
            break  # success — stop trying hosts
        except Exception as e:
            if host == 'query2.finance.yahoo.com':
                print(f"  WARNING: Failed to fetch {ticker} prices: {e}")

    _price_cache[cache_key] = prices
    return prices


def get_price_on_date(prices: Dict[str, float], date_str: str) -> Optional[float]:
    """Get price for a date, falling back to previous trading day (weekends/holidays)."""
    dt = datetime.strptime(date_str, '%Y-%m-%d')
    for offset in range(5):  # look back up to 5 days for holidays/weekends
        key = (dt - timedelta(days=offset)).strftime('%Y-%m-%d')
        if key in prices:
            return prices[key]
    return None


# ---------- Black-Scholes Greeks Estimation ----------

def normal_cdf(x: float) -> float:
    """Cumulative standard normal distribution (Abramowitz & Stegun approximation)."""
    a1, a2, a3 = 0.254829592, -0.284496736, 1.421413741
    a4, a5, p = -1.453152027, 1.061405429, 0.3275911
    sign = -1 if x < 0 else 1
    ax = abs(x) / math.sqrt(2)
    t = 1.0 / (1.0 + p * ax)
    y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * math.exp(-ax * ax)
    return 0.5 * (1.0 + sign * y)


def normal_pdf(x: float) -> float:
    return math.exp(-0.5 * x * x) / math.sqrt(2 * math.pi)


def bs_d1(S: float, K: float, T: float, r: float, sigma: float) -> float:
    if T <= 0 or sigma <= 0:
        return 10.0 if S >= K else -10.0
    return (math.log(S / K) + (r + 0.5 * sigma ** 2) * T) / (sigma * math.sqrt(T))


def bs_price(S: float, K: float, T: float, r: float, sigma: float, opt_type: str) -> float:
    """Black-Scholes theoretical option price."""
    if T <= 0:
        return max(0, S - K) if opt_type == 'call' else max(0, K - S)
    d1 = bs_d1(S, K, T, r, sigma)
    d2 = d1 - sigma * math.sqrt(T)
    if opt_type == 'call':
        return S * normal_cdf(d1) - K * math.exp(-r * T) * normal_cdf(d2)
    return K * math.exp(-r * T) * normal_cdf(-d2) - S * normal_cdf(-d1)


def solve_iv(market_price: float, S: float, K: float, T: float, r: float,
             opt_type: str, initial: float = 0.5) -> Optional[float]:
    """Solve for implied volatility using Newton-Raphson, falling back to bisection."""
    if market_price <= 0 or S <= 0 or K <= 0 or T <= 0:
        return None
    
    # Check if price is within boundary (market_price cannot be less than intrinsic value)
    intrinsic = max(0, S - K) if opt_type == 'call' else max(0, K - S)
    if market_price < intrinsic:
        return 0.01

    # Newton-Raphson
    sigma = initial
    for _ in range(50):
        price = bs_price(S, K, T, r, sigma, opt_type)
        d1 = bs_d1(S, K, T, r, sigma)
        vega = S * normal_pdf(d1) * math.sqrt(T)
        if vega < 1e-12:
            break
        diff = price - market_price
        if abs(diff) < 1e-6:
            return sigma
        sigma -= diff / vega
        if sigma <= 0 or sigma > 8.0:
            break

    # Bisection fallback
    low, high = 0.001, 8.0
    for _ in range(100):
        mid = (low + high) / 2
        price = bs_price(S, K, T, r, mid, opt_type)
        if abs(price - market_price) < 1e-5:
            return mid
        if price < market_price:
            low = mid
        else:
            high = mid
    return mid if (low < high) else None


def option_greeks(S: float, K: float, T: float, r: float, sigma: float,
                  opt_type: str) -> Dict[str, float]:
    """Compute delta, theta (per day), gamma, vega (per 1% vol) for one option."""
    d1 = bs_d1(S, K, T, r, sigma)
    d2 = d1 - sigma * math.sqrt(T)
    sqrt_T = math.sqrt(T)
    nd1 = normal_pdf(d1)

    delta = normal_cdf(d1) if opt_type == 'call' else normal_cdf(d1) - 1
    gamma = nd1 / (S * sigma * sqrt_T) if (S > 0 and sigma > 0 and sqrt_T > 0) else 0

    # Theta (annualized, then convert to per-day)
    theta_ann = -(S * nd1 * sigma) / (2 * sqrt_T)
    if opt_type == 'call':
        theta_ann -= r * K * math.exp(-r * T) * normal_cdf(d2)
    else:
        theta_ann += r * K * math.exp(-r * T) * normal_cdf(-d2)
    theta = theta_ann / 365.0

    vega = S * nd1 * sqrt_T / 100.0  # per 1% move in vol

    return {'delta': delta, 'theta': theta, 'gamma': gamma, 'vega': vega}


def estimate_trade_greeks(trade: Dict, ticker_prices: Dict[str, float],
                          vix_prices: Dict[str, float]) -> Dict[str, Any]:
    """Estimate position Greeks at entry using Black-Scholes.
    Uses real underlying price and VIX from Yahoo Finance historical data.
    Back-solves IV from the short leg premium for more accurate per-leg Greeks."""
    legs = trade.get('legs', [])
    if not legs:
        return {}

    trade_date = trade['tradeDate'][:10]

    # Real underlying price from Yahoo Finance
    S = get_price_on_date(ticker_prices, trade_date)
    if S is None or S <= 0:
        return {}

    # Real VIX from Yahoo Finance
    vix_val = get_price_on_date(vix_prices, trade_date)

    # Time to expiration at entry (in years)
    entry_date = datetime.strptime(trade_date, '%Y-%m-%d').date()
    exp_str = trade['expirationDate'].replace('Z', '') if 'Z' in trade['expirationDate'] else trade['expirationDate']
    exp_date = datetime.fromisoformat(exp_str[:10]).date()
    T = max((exp_date - entry_date).days / 365.25, 1.0 / 365.0)
    r = 0.045

    # Back-solve IV from the short leg premium for best accuracy
    short_leg = next((l for l in legs if l['type'].startswith('short_') and l.get('premium', 0) > 0), None)
    if short_leg:
        opt_type = 'call' if 'call' in short_leg['type'] else 'put'
        sigma = solve_iv(short_leg['premium'], S, short_leg['strike'], T, r, opt_type)
        if sigma is None or sigma <= 0.01:
            # Fallback: use VIX-based estimate
            sigma = (vix_val / 100.0 * 1.2) if vix_val else 0.25
    else:
        sigma = (vix_val / 100.0 * 1.2) if vix_val else 0.25

    # Sum per-leg Greeks into position Greeks (per 1 contract of the spread)
    pos_delta = 0.0
    pos_theta = 0.0
    pos_gamma = 0.0
    pos_vega = 0.0

    for leg in legs:
        is_call = 'call' in leg['type']
        is_short = leg['type'].startswith('short_')
        ot = 'call' if is_call else 'put'
        g = option_greeks(S, leg['strike'], T, r, sigma, ot)
        sign = -1 if is_short else 1
        pos_delta += sign * g['delta']
        pos_theta += sign * g['theta']
        pos_gamma += sign * g['gamma']
        pos_vega += sign * g['vega']

    result: Dict[str, Any] = {
        'delta': round(pos_delta, 4),
        'theta': round(pos_theta, 4),
        'gamma': round(pos_gamma, 4),
        'vega': round(pos_vega, 4),
        'underlyingPrice': round(S, 2),
        'impliedVolatility': round(sigma, 4),
    }
    if vix_val:
        result['vixAtEntry'] = round(vix_val, 1)

    return result

def parse_symbol(symbol: str) -> Optional[Dict]:
    """Parse Schwab symbol format: 'TICKER MM/DD/YYYY STRIKE.00 P|C'"""
    match = re.match(r'(\w+)\s+(\d{2}/\d{2}/\d{4})\s+([\d.]+)\s+([PC])', symbol)
    if match:
        return {
            'ticker': match.group(1),
            'exp_str': match.group(2),
            'expiration': datetime.strptime(match.group(2), '%m/%d/%Y'),
            'strike': float(match.group(3)),
            'opt_type': 'put' if match.group(4) == 'P' else 'call'
        }
    return None

def parse_price(price_str: str) -> float:
    """Parse price string like '$3.67' or '-$1,613.30'"""
    if not price_str:
        return 0
    clean = price_str.replace('$', '').replace(',', '').replace('-', '')
    return float(clean) if clean else 0

def parse_date(date_str: str) -> datetime:
    """Parse date, handling 'as of' format.
    Schwab format: '02/23/2026 as of 02/24/2026' = executed 02/23, settles 02/24.
    We use the FIRST date (execution/trade date), not the settlement date."""
    if ' as of ' in date_str:
        date_str = date_str.split(' as of ')[0]  # Use execution date, not settlement date
    return datetime.strptime(date_str, '%m/%d/%Y')

def find_close(closes: List[Dict], expirations: List[Dict], leg: Dict,
               used_closes: set, used_expirations: set, is_orphan_match: bool = False) -> tuple:
    """Find closing price, date, reason, and fees for a leg."""
    for i, c in enumerate(closes):
        if i in used_closes:
            continue
        # STRICT MATCH: Ticker, Strike, Qty, AND Expiration
        # For orphan matches, accept qty=1 closes against any-qty DB position
        # (Schwab exports each contract as qty=1 even for multi-contract positions)
        qty_match = (c['qty'] == leg['qty']) or (is_orphan_match and c['qty'] == 1)
        if (c['ticker'] == leg['ticker'] and
            c['exp_str'] == leg['exp_str'] and
            c['strike'] == leg['strike'] and
            c['opt_type'] == leg['opt_type'] and
            qty_match):
            if leg['action'] == 'Sell to Open' and c['action'] == 'Buy to Close':
                used_closes.add(i)
                return c['price'], c['date'], 'closed', c.get('fees', 0)
            elif leg['action'] == 'Buy to Open' and c['action'] == 'Sell to Close':
                used_closes.add(i)
                return c['price'], c['date'], 'closed', c.get('fees', 0)

    for i, e in enumerate(expirations):
        if i in used_expirations:
            continue
        if (e['ticker'] == leg['ticker'] and
            e['exp_str'] == leg['exp_str'] and
            e['strike'] == leg['strike'] and
            e['opt_type'] == leg['opt_type'] and
            e['qty'] == leg['qty']):
            used_expirations.add(i)
            return 0.0, e['expiration'], 'expired', 0
    return None, None, None, 0

def build_trade_dict(ticker, spread_type, qty, short_strike, long_strike,
                     short_prem, long_prem, credit, trade_date, expiration,
                     commission, api_legs, close_date=None, exit_reason=None,
                     close_commission=0, back_expiration=None):
    """Build trade dictionary"""
    entry_date = datetime.strptime(trade_date, '%Y-%m-%d')
    
    # Ensure every leg has an expirationDate key if it's missing
    # (using the trade's primary expiration as fallback)
    for leg in api_legs:
        if 'expirationDate' not in leg:
            leg['expirationDate'] = expiration.strftime('%Y-%m-%d')
        else:
            # Normalize existing to YYYY-MM-DD
            if 'T' in leg['expirationDate']:
                leg['expirationDate'] = leg['expirationDate'].split('T')[0]

    trade = {
        'ticker': ticker,
        'spreadType': spread_type,
        'quantity': qty,
        'shortStrike': short_strike,
        'longStrike': long_strike,
        'shortPremium': short_prem,
        'longPremium': long_prem,
        'creditReceived': round(credit, 2),
        'entryTime': entry_date.strftime('%Y-%m-%dT10:00:00Z'),
        'expirationDate': expiration.strftime('%Y-%m-%dT00:00:00Z'),
        'tradeDate': trade_date,
        'commission': round(commission + close_commission, 2),  # Total commission (open + close)
        'legs': api_legs
    }
    # Set back-month expiration for multi-expiration strategies (calendars, diagonals, PMCC)
    if back_expiration:
        trade['backMonthExpiration'] = back_expiration.strftime('%Y-%m-%dT00:00:00Z')
    is_closed = all(l.get('closePremium') is not None for l in api_legs)
    if is_closed and close_date:
        trade['exitTime'] = close_date.strftime('%Y-%m-%dT16:00:00Z')
    elif is_closed:
        trade['exitTime'] = expiration.strftime('%Y-%m-%dT16:00:00Z')

    # Set exit reason if provided
    if exit_reason and is_closed:
        trade['exitReason'] = exit_reason

    return trade


def enrich_trades_with_greeks(trades: List[Dict]) -> int:
    """Fetch historical prices from Yahoo Finance and estimate Greeks.
    Only fetches necessary data for the provided trades batch."""
    if not trades:
        return 0

    # Group trades by ticker to find specific date ranges per ticker
    by_ticker = defaultdict(list)
    all_dates = []
    for t in trades:
        by_ticker[t['ticker']].append(t)
        td = t.get('tradeDate')
        if td:
            all_dates.append(td[:10])
        et = t.get('exitTime')
        if et:
            all_dates.append(et[:10])

    if not all_dates:
        return 0

    # Fetch VIX for the entire range of this batch
    vix_start = min(all_dates)
    vix_end = max(all_dates)
    vix_prices = fetch_historical_prices('^VIX', vix_start, vix_end)
    if vix_prices:
        print(f"  Fetched ^VIX: {vix_start} to {vix_end}")

    # Fetch prices and enrich per ticker
    enriched = 0
    print(f"  Enriching {len(trades)} trades across {len(by_ticker)} ticker(s)...")

    for ticker, ticker_trades in by_ticker.items():
        # Find date range for THIS ticker
        ticker_dates = []
        for t in ticker_trades:
            td = t.get('tradeDate')
            if td:
                ticker_dates.append(td[:10])
            et = t.get('exitTime')
            if et:
                ticker_dates.append(et[:10])

        if not ticker_dates:
            continue

        start = min(ticker_dates)
        end = max(ticker_dates)

        # Fetch prices for this ticker only for its required range
        _YAHOO_TICKER_MAP = {'SPXW': '^SPX', 'NDXP': '^NDX'}
        yahoo_ticker = _YAHOO_TICKER_MAP.get(ticker, ticker)
        prices = fetch_historical_prices(yahoo_ticker, start, end)

        for trade in ticker_trades:
            greeks = estimate_trade_greeks(trade, prices, vix_prices)
            if greeks:
                trade.update(greeks)
                enriched += 1

            # Look up underlying price at close for closed trades
            if trade.get('exitTime'):
                close_date_str = trade['exitTime'][:10]
                close_price = get_price_on_date(prices, close_date_str)
                if close_price is not None:
                    trade['underlyingPriceAtClose'] = round(close_price, 2)

    return enriched

def process_same_exp_legs(legs: List[Dict], trade_date: str, ticker: str,
                          closes: List[Dict], expirations: List[Dict],
                          used_closes: set, used_expirations: set) -> tuple:
    """
    Process legs with same ticker/date/expiration into trades (IC, Verticals).
    Returns (trades, remaining_legs).
    """
    trades = []
    short_puts = [l for l in legs if l['action'] == 'Sell to Open' and l['opt_type'] == 'put']
    long_puts = [l for l in legs if l['action'] == 'Buy to Open' and l['opt_type'] == 'put']
    short_calls = [l for l in legs if l['action'] == 'Sell to Open' and l['opt_type'] == 'call']
    long_calls = [l for l in legs if l['action'] == 'Buy to Open' and l['opt_type'] == 'call']

    print(f"  [{ticker} {trade_date}] Legs: {len(short_puts)} short_put, {len(long_puts)} long_put, {len(short_calls)} short_call, {len(long_calls)} long_call")

    # Iron Condor
    if len(short_puts) == 1 and len(long_puts) == 1 and len(short_calls) == 1 and len(long_calls) == 1:
        sp, lp, sc, lc = short_puts[0], long_puts[0], short_calls[0], long_calls[0]
        if sp['qty'] == lp['qty'] == sc['qty'] == lc['qty']:
            qty = sp['qty']
            credit = (sp['price'] - lp['price']) + (sc['price'] - lc['price'])
            sp_close, sp_date, sp_reason, sp_cfee = find_close(closes, expirations, sp, used_closes, used_expirations)
            lp_close, lp_date, lp_reason, lp_cfee = find_close(closes, expirations, lp, used_closes, used_expirations)
            sc_close, sc_date, sc_reason, sc_cfee = find_close(closes, expirations, sc, used_closes, used_expirations)
            lc_close, lc_date, lc_reason, lc_cfee = find_close(closes, expirations, lc, used_closes, used_expirations)
            close_date = sp_date or lp_date or sc_date or lc_date
            exit_reason = sp_reason or lp_reason or sc_reason or lc_reason
            close_comm = sp_cfee + lp_cfee + sc_cfee + lc_cfee
            api_legs = [
                {'type': 'short_put', 'strike': sp['strike'], 'premium': sp['price'], 'closePremium': sp_close},
                {'type': 'long_put', 'strike': lp['strike'], 'premium': lp['price'], 'closePremium': lp_close},
                {'type': 'short_call', 'strike': sc['strike'], 'premium': sc['price'], 'closePremium': sc_close},
                {'type': 'long_call', 'strike': lc['strike'], 'premium': lc['price'], 'closePremium': lc_close},
            ]
            trade = build_trade_dict(ticker, 'iron_condor', qty, sp['strike'], lp['strike'],
                                     sp['price'], lp['price'], round(credit, 4), trade_date,
                                     sp['expiration'], sp['fees'] + lp['fees'] + sc['fees'] + lc['fees'],
                                     api_legs, close_date, exit_reason, close_comm)
            trades.append(trade)
            # All consumed
            return trades, []

    # Put verticals (pair adjacent strikes)
    while short_puts and long_puts:
        short_puts.sort(key=lambda x: -x['strike'])
        long_puts.sort(key=lambda x: -x['strike']) # Both Descending
        matched = False
        for sp in short_puts[:]:
            for lp in long_puts[:]:
                if sp['qty'] == lp['qty']:
                    qty = sp['qty']
                    # Determine direction from strikes (don't swap — keep sp=Sell leg, lp=Buy leg)
                    if sp['strike'] > lp['strike']:
                        # Standard: sell higher put, buy lower put → credit spread = Short Put Vertical
                        spread_type = 'short_put_vertical'
                    else:
                        # Inverted: sell lower put, buy higher put → debit spread = Long Put Vertical
                        spread_type = 'long_put_vertical'
                    credit = round(sp['price'] - lp['price'], 4)
                    sp_close, sp_date, sp_reason, sp_cfee = find_close(closes, expirations, sp, used_closes, used_expirations)
                    lp_close, lp_date, lp_reason, lp_cfee = find_close(closes, expirations, lp, used_closes, used_expirations)
                    close_date = sp_date or lp_date
                    exit_reason = sp_reason or lp_reason
                    close_comm = sp_cfee + lp_cfee
                    api_legs = [
                        {'type': 'short_put', 'strike': sp['strike'], 'premium': sp['price'], 'closePremium': sp_close},
                        {'type': 'long_put', 'strike': lp['strike'], 'premium': lp['price'], 'closePremium': lp_close},
                    ]
                    print(f"  Detected {spread_type}: {ticker} {sp['strike']}/{lp['strike']} cr={credit:+.2f}")
                    trade = build_trade_dict(ticker, spread_type, qty, sp['strike'], lp['strike'],
                                             sp['price'], lp['price'], credit, trade_date,
                                             sp['expiration'], sp['fees'] + lp['fees'], api_legs, close_date, exit_reason,
                                             close_comm)
                    trades.append(trade)
                    short_puts.remove(sp)
                    long_puts.remove(lp)
                    matched = True
                    break
            if matched:
                break
        if not matched:
            break

    # Call verticals (pair adjacent strikes)
    while short_calls and long_calls:
        short_calls.sort(key=lambda x: x['strike'])
        long_calls.sort(key=lambda x: x['strike']) # Both Ascending
        matched = False
        for sc in short_calls[:]:
            for lc in long_calls[:]:
                if sc['qty'] == lc['qty']:
                    qty = sc['qty']
                    # Determine direction from strikes (don't swap — keep sc=Sell leg, lc=Buy leg)
                    if sc['strike'] < lc['strike']:
                        # Standard: sell lower call, buy higher call → credit spread = Short Call Vertical
                        spread_type = 'short_call_vertical'
                    else:
                        # Inverted: sell higher call, buy lower call → debit spread = Long Call Vertical
                        spread_type = 'long_call_vertical'
                    credit = round(sc['price'] - lc['price'], 4)
                    sc_close, sc_date, sc_reason, sc_cfee = find_close(closes, expirations, sc, used_closes, used_expirations)
                    lc_close, lc_date, lc_reason, lc_cfee = find_close(closes, expirations, lc, used_closes, used_expirations)
                    close_date = sc_date or lc_date
                    exit_reason = sc_reason or lc_reason
                    close_comm = sc_cfee + lc_cfee
                    api_legs = [
                        {'type': 'short_call', 'strike': sc['strike'], 'premium': sc['price'], 'closePremium': sc_close},
                        {'type': 'long_call', 'strike': lc['strike'], 'premium': lc['price'], 'closePremium': lc_close},
                    ]
                    print(f"  Detected {spread_type}: {ticker} {sc['strike']}/{lc['strike']} cr={credit:+.2f}")
                    trade = build_trade_dict(ticker, spread_type, qty, sc['strike'], lc['strike'],
                                             sc['price'], lc['price'], credit, trade_date,
                                             sc['expiration'], sc['fees'] + lc['fees'], api_legs, close_date, exit_reason,
                                             close_comm)
                    trades.append(trade)
                    short_calls.remove(sc)
                    long_calls.remove(lc)
                    matched = True
                    break
            if matched:
                break
        if not matched:
            break

    # Strangle / Straddle: 1 short put + 1 short call, no long legs remaining
    # (If there were long legs they would have been paired above as a vertical or IC)
    if len(short_puts) == 1 and len(short_calls) == 1 and not long_puts and not long_calls:
        sp, sc = short_puts[0], short_calls[0]
        if sp['qty'] == sc['qty']:
            qty = sp['qty']
            credit = sp['price'] + sc['price']
            sp_close, sp_date, sp_reason, sp_cfee = find_close(closes, expirations, sp, used_closes, used_expirations)
            sc_close, sc_date, sc_reason, sc_cfee = find_close(closes, expirations, sc, used_closes, used_expirations)
            close_date = sp_date or sc_date
            exit_reason = sp_reason or sc_reason
            close_comm = sp_cfee + sc_cfee
            # Straddle = same strike; Strangle = different strikes
            spread_type = 'straddle' if sp['strike'] == sc['strike'] else 'strangle'
            api_legs = [
                {'type': 'short_put',  'strike': sp['strike'], 'premium': sp['price'], 'closePremium': sp_close},
                {'type': 'short_call', 'strike': sc['strike'], 'premium': sc['price'], 'closePremium': sc_close},
            ]
            print(f"  Detected {spread_type}: {ticker} {sp['strike']}/{sc['strike']} cr={credit:+.2f}")
            trade = build_trade_dict(ticker, spread_type, qty, sp['strike'], sc['strike'],
                                     sp['price'], sc['price'], round(credit, 4), trade_date,
                                     sp['expiration'], sp['fees'] + sc['fees'],
                                     api_legs, close_date, exit_reason, close_comm)
            trades.append(trade)
            short_puts.clear()
            short_calls.clear()

    leftovers = short_puts + long_puts + short_calls + long_calls
    return trades, leftovers


def build_0dte_roll_chain(txns: List[Dict], ticker: str, trade_date: str) -> List[Dict]:
    """Build an IC roll chain from 0DTE same-expiry transactions.

    Called when the same (date, ticker, expiry) has >4 open legs — an intraday roll
    campaign where multiple ICs are opened sequentially, each linked to the next.

    Schwab interleaves open and close transactions within a roll event (e.g. new STO
    may appear before the BTC of the old short). Sequential slot-tracking therefore
    fails. Instead this function uses VERTICAL-FAMILY IDENTIFICATION:

    Pass 1: Group STO+BTO pairs into put/call vertical families by positional sort
            (descending for puts, ascending for calls). Each family gets its open
            premiums and close premiums (from matching BTC/STC transactions).

    Pass 2: Sort verticals by the earliest txn index where each family was first
            opened (= temporal order). Build the IC chain by walking put and call
            verticals in parallel: whichever side's NEXT vertical starts earlier
            determines the roll direction for the current IC.

    Commission is only charged for NEW legs in each IC (not inherited sides).
    forcePnl is set for rolled ICs (partial close); fully-closed ICs use
    compute_pnl_and_debit in insert_trades.

    Each trade gets _is_0dte_chain, _chain_position, _chain_ticker, _chain_date so
    main() can set rolled_from_id by position after insert (bypassing link_roll_chains
    which is confused by same-day trades).
    """
    exp_dt: Optional[datetime] = txns[0]['expiration'] if txns else datetime.strptime(trade_date, '%Y-%m-%d')

    # ── Pass 1: identify vertical families ──────────────────────────────────────

    def _pair_verticals(legs_with_pos: List[Tuple[int, Dict]]) -> List[Dict]:
        """Group STO+BTO into vertical pairs by positional strike sort.

        For puts:  sort descending → highest short pairs with highest long
        For calls: sort ascending  → lowest short pairs with lowest long

        Each vertical dict records open premiums, close premiums (if any), and
        start_pos (earliest txn index) for temporal ordering.
        """
        if not legs_with_pos:
            return []
        opt_type = legs_with_pos[0][1]['opt_type']
        is_put   = (opt_type == 'put')

        shorts    = [(i, t) for i, t in legs_with_pos if t['action'] == 'Sell to Open']
        longs     = [(i, t) for i, t in legs_with_pos if t['action'] == 'Buy to Open']
        cls_btc   = [(i, t) for i, t in legs_with_pos if t['action'] == 'Buy to Close']   # closes short
        cls_stc   = [(i, t) for i, t in legs_with_pos if t['action'] == 'Sell to Close']  # closes long

        rev = True if is_put else False
        shorts.sort( key=lambda x:  x[1]['strike'], reverse=rev)
        longs.sort(  key=lambda x:  x[1]['strike'], reverse=rev)
        cls_btc.sort(key=lambda x:  x[1]['strike'], reverse=rev)
        cls_stc.sort(key=lambda x:  x[1]['strike'], reverse=rev)

        verticals = []
        used_btc = set()
        used_stc = set()
        for k in range(min(len(shorts), len(longs))):
            si, s = shorts[k]
            li, l = longs[k]

            # Find matching close for the short leg (BTC, same strike)
            sc_entry = next(
                ((ci, c) for ci, (ci2, c) in enumerate(cls_btc)
                 if ci2 not in used_btc and abs(c['strike'] - s['strike']) < 0.01),
                None
            )
            # Redo without the enumerate bug:
            sc_txn, sc_pos = None, None
            for bi, (bpos, bt) in enumerate(cls_btc):
                if bi not in used_btc and abs(bt['strike'] - s['strike']) < 0.01:
                    sc_txn, sc_pos = bt, bi
                    used_btc.add(bi)
                    break

            lc_txn, lc_pos = None, None
            for si2, (spos, st) in enumerate(cls_stc):
                if si2 not in used_stc and abs(st['strike'] - l['strike']) < 0.01:
                    lc_txn, lc_pos = st, si2
                    used_stc.add(si2)
                    break

            verticals.append({
                'start_pos':   min(si, li),   # temporal order: earliest open txn index
                'short':       s,             # STO transaction
                'long':        l,             # BTO transaction
                'close_short': sc_txn,        # BTC transaction (or None)
                'close_long':  lc_txn,        # STC transaction (or None)
            })

        verticals.sort(key=lambda v: v['start_pos'])
        return verticals

    puts_with_pos  = [(i, t) for i, t in enumerate(txns) if t['opt_type'] == 'put']
    calls_with_pos = [(i, t) for i, t in enumerate(txns) if t['opt_type'] == 'call']

    put_verticals  = _pair_verticals(puts_with_pos)
    call_verticals = _pair_verticals(calls_with_pos)

    if not put_verticals or not call_verticals:
        print(f"  [0DTE] Insufficient verticals ({len(put_verticals)} puts, "
              f"{len(call_verticals)} calls) — falling back to normal processing")
        return []

    # ── Pass 2: build IC chain ───────────────────────────────────────────────────

    trades_out: List[Dict] = []
    pi, ci       = 1, 1                    # pointers to NEXT (unused) vertical
    cur_put      = put_verticals[0]
    cur_call     = call_verticals[0]
    is_new_put   = True
    is_new_call  = True

    while True:
        next_put  = put_verticals[pi]   if pi  < len(put_verticals)  else None
        next_call = call_verticals[ci]  if ci  < len(call_verticals) else None
        is_last   = (next_put is None and next_call is None)

        # Which side closes in this IC?
        if is_last:
            close_side = 'both'
        elif next_call and (next_put is None or next_call['start_pos'] <= next_put['start_pos']):
            close_side = 'call'   # call vertical rolls next
        else:
            close_side = 'put'    # put vertical rolls next

        # Build leg dicts with close premiums for the appropriate side
        def _leg(ltype, v_entry, close_txn):
            return {
                'type':         ltype,
                'strike':       v_entry['strike'],
                'premium':      v_entry['price'],
                'closePremium': close_txn['price'] if close_txn else None,
            }

        sp_leg = _leg('short_put',  cur_put['short'],  cur_put['close_short']  if close_side in ('put',  'both') else None)
        lp_leg = _leg('long_put',   cur_put['long'],   cur_put['close_long']   if close_side in ('put',  'both') else None)
        sc_leg = _leg('short_call', cur_call['short'], cur_call['close_short'] if close_side in ('call', 'both') else None)
        lc_leg = _leg('long_call',  cur_call['long'],  cur_call['close_long']  if close_side in ('call', 'both') else None)

        all_closed = all(l['closePremium'] is not None for l in [sp_leg, lp_leg, sc_leg, lc_leg])

        # Commission: open fees for NEW verticals + close fees for closed legs
        commission = 0.0
        if is_new_put:
            commission += cur_put['short'].get('fees', 0) + cur_put['long'].get('fees', 0)
        if is_new_call:
            commission += cur_call['short'].get('fees', 0) + cur_call['long'].get('fees', 0)
        if close_side in ('call', 'both'):
            if cur_call['close_short']: commission += cur_call['close_short'].get('fees', 0)
            if cur_call['close_long']:  commission += cur_call['close_long'].get('fees', 0)
        if close_side in ('put', 'both'):
            if cur_put['close_short']:  commission += cur_put['close_short'].get('fees', 0)
            if cur_put['close_long']:   commission += cur_put['close_long'].get('fees', 0)
        commission = round(commission, 2)

        put_credit  = sp_leg['premium'] - lp_leg['premium']
        call_credit = sc_leg['premium'] - lc_leg['premium']
        credit      = round(put_credit + call_credit, 4)
        api_legs    = [sp_leg, lp_leg, sc_leg, lc_leg]

        trade = build_trade_dict(
            ticker, 'iron_condor', 1,
            sp_leg['strike'], lp_leg['strike'],
            sp_leg['premium'], lp_leg['premium'],
            credit, trade_date, exp_dt,
            commission, api_legs,
        )

        # build_trade_dict only sets exitTime/exitReason when ALL legs have closePremium.
        # For rolled ICs (partial close), set them manually.
        if close_side:
            trade['exitTime']   = f"{trade_date}T16:00:00Z"
            trade['exitReason'] = 'closed' if all_closed else 'rolled'
            # forcePnl for rolled ICs: realized P&L on closed side, net of commission.
            # Fully-closed ICs use compute_pnl_and_debit (which also deducts commission).
            if not all_closed:
                closed_legs = [l for l in api_legs if l['closePremium'] is not None]
                side_gross  = sum(
                    (l['premium'] - l['closePremium']) if l['type'].startswith('short_')
                    else (l['closePremium'] - l['premium'])
                    for l in closed_legs
                )
                trade['forcePnl'] = round(side_gross * 100.0 - commission, 2)

        trade['_is_0dte_chain']  = True
        trade['_chain_position'] = len(trades_out)
        trade['_chain_ticker']   = ticker
        trade['_chain_date']     = trade_date

        print(f"  [0DTE IC {len(trades_out) + 1}] {ticker} "
              f"P:{sp_leg['strike']:.0f}/{lp_leg['strike']:.0f} "
              f"C:{sc_leg['strike']:.0f}/{lc_leg['strike']:.0f} "
              f"cr={credit:+.2f} comm={commission:.2f} "
              f"exit={trade.get('exitReason', 'OPEN')}")
        trades_out.append(trade)

        if is_last:
            break
        elif close_side == 'call':
            cur_call    = next_call
            is_new_put  = False
            is_new_call = True
            ci += 1
        else:  # put rolled
            cur_put     = next_put
            is_new_put  = True
            is_new_call = False
            pi += 1

    return trades_out


def build_trades(transactions: List[Dict], conn: Optional[sqlite3.Connection] = None) -> List[Dict]:
    """Build trades from Schwab transactions. 
    If conn is provided, matches orphan closes against open trades in the DB."""
    parsed_txns = []
    for txn in transactions:
        sym = parse_symbol(txn['Symbol'])
        if not sym:
            continue
        qty = int(float(txn['Quantity']))  # float() first handles "1.00" / "-1.00" strings
        parsed_txns.append({
            'date': parse_date(txn['Date']),
            'action': txn['Action'],
            'qty': abs(qty),
            'signed_qty': qty,
            'price': parse_price(txn['Price']),
            'fees': parse_price(txn.get('Fees & Comm', '0')),
            **sym
        })

    opens_by_date_ticker = defaultdict(list)
    closes = []
    expirations = []

    # --- 0DTE Roll Campaign Detection ---
    # Count open legs per (date, ticker, expiry) for same-day, same-expiry groups.
    # A group with >4 opens is a roll campaign (a single IC has exactly 4 open legs).
    _0dte_open_counts: Dict[Tuple, int] = defaultdict(int)
    for txn in parsed_txns:
        if txn['action'] in ['Sell to Open', 'Buy to Open']:
            d = txn['date'].strftime('%Y-%m-%d')
            e = txn['expiration'].strftime('%Y-%m-%d')
            if d == e:  # 0DTE: trade date == expiry date
                _0dte_open_counts[(d, txn['ticker'], e)] += 1

    # Keys with >4 opens → route ALL their transactions to build_0dte_roll_chain
    _0dte_roll_keys: set = {
        (d, tk)
        for (d, tk, _e), cnt in _0dte_open_counts.items()
        if cnt > 4
    }

    zero_dte_txns: Dict[Tuple, List] = defaultdict(list)  # (date_str, ticker) → ordered txns

    for txn in parsed_txns:
        d   = txn['date'].strftime('%Y-%m-%d')
        e   = txn['expiration'].strftime('%Y-%m-%d')
        key = (d, txn['ticker'])

        if key in _0dte_roll_keys and d == e:
            # 0DTE roll campaign: collect opens AND closes in chronological order
            if txn['action'] in ['Sell to Open', 'Buy to Open', 'Sell to Close', 'Buy to Close']:
                zero_dte_txns[key].append(txn)
        else:
            if txn['action'] in ['Sell to Open', 'Buy to Open']:
                opens_by_date_ticker[key].append(txn)
            elif txn['action'] in ['Sell to Close', 'Buy to Close']:
                closes.append(txn)
            elif txn['action'] == 'Expired':
                expirations.append(txn)

    trades = []
    used_closes = set()
    used_expirations = set()

    # 1. PRIORITIZE: Match closes in file to open trades already in DB
    # This prevents 'to Close' transactions from being 'stolen' to create new trades
    if conn:
        db_opens = get_open_trades_from_db(conn)
        for t_db in db_opens:
            matched_legs = 0
            temp_used_closes = set()
            new_legs = []
            close_dates: list = []  # actual BTC/STC dates from matched transactions

            # For multi-expiry trades (Calendars/Diagonals), legs might have different expiries
            possible_expirations = [t_db['expiration_date'][:10]]
            if t_db.get('back_month_expiration'):
                possible_expirations.append(t_db['back_month_expiration'][:10])

            for leg in t_db['legs']:
                found_leg_close = False
                for exp_date_str in possible_expirations:
                    leg_match = {
                        'ticker': t_db['ticker'],
                        'exp_str': datetime.strptime(exp_date_str, '%Y-%m-%d').strftime('%m/%d/%Y'),
                        'strike': leg['strike'],
                        'opt_type': 'call' if 'call' in leg['type'] else 'put',
                        'qty': t_db['quantity'],
                        'action': 'Sell to Open' if 'short' in leg['type'] else 'Buy to Open'
                    }

                    c_price, c_date, c_reason, c_fee = find_close(closes, expirations, leg_match, temp_used_closes, used_expirations, is_orphan_match=True)

                    if c_price is not None:
                        matched_legs += 1
                        leg_copy = dict(leg)
                        leg_copy['closePremium'] = c_price
                        new_legs.append(leg_copy)
                        if c_date:
                            close_dates.append(c_date)
                        found_leg_close = True
                        break

                if not found_leg_close:
                    new_legs.append(leg)

            if matched_legs > 0:
                # Build a fingerprint for this specific close event so it can't re-fire
                # on the next import run when the same overlapping date range is used.
                oc_dates_str = ','.join(sorted(d.strftime('%Y-%m-%d') for d in close_dates)) if close_dates else 'unknown'
                oc_fp = f"oc|{t_db['id']}|{oc_dates_str}"
                if conn.execute("SELECT 1 FROM import_log WHERE fingerprint=?", (oc_fp,)).fetchone():
                    # Already processed — skip to avoid double-closing the trade.
                    # But if a roll was recorded for this IC, consume the rolled-to open
                    # legs from opens_by_date_ticker so they don't become new standalone trades.
                    roll_row = conn.execute(
                        "SELECT fingerprint FROM import_log WHERE fingerprint LIKE ?",
                        (f'roll|{t_db["id"]}|%',)
                    ).fetchone()
                    if roll_row:
                        roll_date_str = roll_row[0].split('|')[2]
                        roll_key = (roll_date_str, t_db['ticker'])
                        roll_pool = opens_by_date_ticker.get(roll_key, [])
                        if roll_pool:
                            # Remove any open legs from the pool matching the IC's
                            # new rolled-to legs (those without closePremium in the DB).
                            for leg in t_db['legs']:
                                if leg.get('closePremium') is not None:
                                    continue
                                opt = 'call' if 'call' in leg['type'] else 'put'
                                action = 'Sell to Open' if 'short' in leg['type'] else 'Buy to Open'
                                to_remove = [
                                    o for o in roll_pool
                                    if o['opt_type'] == opt
                                    and o['action'] == action
                                    and o['strike'] == leg['strike']
                                ]
                                for item in to_remove:
                                    roll_pool.remove(item)
                    continue

                used_closes.update(temp_used_closes)

                updated_trade = {
                    'id': t_db['id'],
                    'ticker': t_db['ticker'],
                    'spreadType': t_db['strategy'],
                    'quantity': t_db['quantity'],
                    'creditReceived': t_db['credit_received'],
                    'tradeDate': t_db['trade_date'] or t_db['entry_date'],
                    'expirationDate': t_db['expiration_date'],
                    'entryTime': t_db['entry_date'],
                    'commission': t_db['commission'] or 0.0,
                    'legs': new_legs,
                    'is_update': True
                }

                if matched_legs == len(t_db['legs']):
                    print(f"  Matched orphan close for {t_db['ticker']} ID {t_db['id']} ({t_db['strategy']})")
                    if close_dates:
                        updated_trade['exitTime'] = max(close_dates).strftime('%Y-%m-%dT16:00:00Z')
                    else:
                        updated_trade['exitTime'] = datetime.now().strftime('%Y-%m-%dT16:00:00Z')
                    updated_trade['exitReason'] = 'closed'
                else:
                    print(f"  Matched partial orphan close ({matched_legs}/{len(t_db['legs'])}) for {t_db['ticker']} ID {t_db['id']} ({t_db['strategy']})")

                    # ── ROLL DETECTION ────────────────────────────────────────────────
                    # If this is an IC and exactly one side (call or put) was closed,
                    # look for replacement open legs in opens_by_date_ticker that form a
                    # replacement spread of the same type and absorb them into the IC.
                    if t_db['strategy'] == 'iron_condor' and matched_legs == 2:
                        closed_types = {leg['type'] for leg in new_legs if leg.get('closePremium') is not None}

                        if closed_types == {'short_call', 'long_call'}:
                            roll_opt_type = 'call'
                        elif closed_types == {'short_put', 'long_put'}:
                            roll_opt_type = 'put'
                        else:
                            roll_opt_type = None

                        if roll_opt_type and close_dates:
                            roll_date = max(close_dates).strftime('%Y-%m-%d')
                            roll_key  = (roll_date, t_db['ticker'])
                            roll_pool = opens_by_date_ticker.get(roll_key, [])

                            cands_short = [
                                o for o in roll_pool
                                if o['opt_type'] == roll_opt_type and o['action'] == 'Sell to Open'
                            ]
                            cands_long = [
                                o for o in roll_pool
                                if o['opt_type'] == roll_opt_type and o['action'] == 'Buy to Open'
                            ]

                            if cands_short and cands_long:
                                new_short = cands_short[0]
                                new_long  = cands_long[0]

                                if roll_opt_type == 'call':
                                    is_credit = new_short['strike'] < new_long['strike']
                                    new_leg_short = {'type': 'short_call', 'strike': new_short['strike'],
                                                     'premium': new_short['price'], 'closePremium': None}
                                    new_leg_long  = {'type': 'long_call',  'strike': new_long['strike'],
                                                     'premium': new_long['price'],  'closePremium': None}
                                else:
                                    is_credit = new_short['strike'] > new_long['strike']
                                    new_leg_short = {'type': 'short_put', 'strike': new_short['strike'],
                                                     'premium': new_short['price'], 'closePremium': None}
                                    new_leg_long  = {'type': 'long_put',  'strike': new_long['strike'],
                                                     'premium': new_long['price'],  'closePremium': None}

                                if is_credit:
                                    # Close the original trade (rolled away) — keep original legs only
                                    updated_trade['exitTime']   = roll_date + 'T16:00:00Z'
                                    updated_trade['exitReason'] = 'rolled'
                                    updated_trade['legs']       = new_legs  # original legs with closePremium on rolled side

                                    # Compute realized P&L on the closed side for the parent trade
                                    closed_legs = [l for l in new_legs if l.get('closePremium') is not None]
                                    side_pnl = 0.0
                                    for l in closed_legs:
                                        cp = l['closePremium']
                                        if l['type'].startswith('short_'):
                                            side_pnl += l['premium'] - cp   # short: kept the difference
                                        else:
                                            side_pnl += cp - l['premium']   # long: received minus paid
                                    updated_trade['forcePnl'] = round(side_pnl * 100.0 * t_db['quantity'], 2)

                                    # Build child trade (new IC: open put legs + new call/put spread)
                                    exp_str = t_db['expiration_date'][:10]
                                    new_leg_short['expirationDate'] = exp_str
                                    new_leg_long['expirationDate']  = exp_str
                                    put_legs_open = [l for l in new_legs if 'put' in l['type']]
                                    child_legs    = put_legs_open + [new_leg_short, new_leg_long]
                                    child_credit  = sum(
                                        l['premium'] if l['type'].startswith('short_') else -l['premium']
                                        for l in child_legs
                                    )
                                    child_trade = {
                                        'ticker':          t_db['ticker'],
                                        'spreadType':      t_db['strategy'],
                                        'quantity':        t_db['quantity'],
                                        'creditReceived':  round(child_credit, 2),
                                        'tradeDate':       roll_date,
                                        'entryTime':       roll_date + 'T10:00:00Z',
                                        'expirationDate':  exp_str,
                                        'legs':            child_legs,
                                        'commission':      t_db.get('commission'),
                                        'underlyingPrice': t_db.get('underlying_price'),
                                        'vixAtEntry':      t_db.get('vix_at_entry'),
                                    }
                                    trades.append(child_trade)

                                    # Consume the roll opens so they don't become new standalone trades
                                    if new_short in roll_pool:
                                        roll_pool.remove(new_short)
                                    if new_long in roll_pool:
                                        roll_pool.remove(new_long)

                                    # Roll fingerprint — prevents re-processing this roll on future imports
                                    roll_fp = f"roll|{t_db['id']}|{roll_date}"
                                    conn.execute(
                                        "INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)",
                                        (roll_fp, t_db['id'])
                                    )
                                    conn.commit()

                                    print(f"  [ROLL] Chain created: {t_db['ticker']} IC {t_db['id']} → new child IC "
                                          f"new legs {new_leg_short['strike']}/{new_leg_long['strike']}, "
                                          f"parent pnl=${updated_trade['forcePnl']:+.2f}, child cr={child_credit:+.2f}")

                # Pass the oc fingerprint through to insert_trades so it's only persisted
                # AFTER the UPDATE commit succeeds — avoids blocking future retries on failure.
                updated_trade['_oc_fp'] = oc_fp

                trades.append(updated_trade)

    # 2. Build NEW trades from the 'Open' transactions in this file
    for (trade_date, ticker), legs in opens_by_date_ticker.items():
        # PRIORITIZE: Exactly 3 legs on the same day = Custom/Ratio Spread (e.g. SMR)
        if len(legs) == 3:
            total_credit = 0.0
            api_legs = []
            all_strikes = []
            total_fees = 0.0
            
            # Find the "base" quantity (usually the spread quantity)
            qtys = [l['qty'] for l in legs]
            base_qty = min(qtys)
            
            for leg in legs:
                sign = -1 if leg['action'] == 'Buy to Open' else 1
                total_credit += sign * (leg['price'] * leg['qty'])
                total_fees += leg.get('fees', 0.0)
                all_strikes.append(leg['strike'])
                api_legs.append({
                    'type': f"{'short' if sign > 0 else 'long'}_{leg['opt_type']}",
                    'strike': leg['strike'],
                    'premium': leg['price'],
                    'closePremium': None,
                    'quantity': leg['qty'],
                    'expirationDate': leg['expiration'].strftime('%Y-%m-%d')
                })
            
            # Unit credit is total dollars / base quantity
            unit_credit = total_credit / base_qty if base_qty > 0 else 0.0
            
            print(f"  Detected Custom (3 legs): {ticker} cr={unit_credit:+.2f} qty={base_qty} fees={total_fees:.2f}")
            trade = build_trade_dict(ticker, 'custom', base_qty, min(all_strikes), max(all_strikes),
                                     0, 0, unit_credit, trade_date,
                                     legs[0]['expiration'], total_fees,
                                     api_legs)
            trades.append(trade)
            continue

        # 1. Group by expiration for Vertical/IC detection
        by_exp = defaultdict(list)
        for leg in legs:
            by_exp[leg['exp_str']].append(leg)

        all_leftovers = []

        # 2. Extract Verticals and ICs first (Same Expiration Priority)
        for exp_str, exp_legs in by_exp.items():
            vertical_trades, leftovers = process_same_exp_legs(exp_legs, trade_date, ticker, closes, expirations, used_closes, used_expirations)
            trades.extend(vertical_trades)
            all_leftovers.extend(leftovers)

        # 3. Calendar Spread Detection (Cross Expiration Priority)
        # Re-group leftovers by expiration
        leftover_by_exp = defaultdict(list)
        for leg in all_leftovers:
            leftover_by_exp[leg['exp_str']].append(leg)

        exp_dates = sorted(list(leftover_by_exp.keys()), key=lambda d: datetime.strptime(d, '%m/%d/%Y'))

        # Iterate through pairs of expirations to find matches
        for i in range(len(exp_dates)):
            for j in range(i + 1, len(exp_dates)):
                exp1 = exp_dates[i]
                exp2 = exp_dates[j]

                legs1 = leftover_by_exp[exp1]
                legs2 = leftover_by_exp[exp2]

                matched = True
                while matched:
                    matched = False
                    for l1 in legs1[:]:
                        for l2 in legs2[:]:
                            # Match found: Same Type, Cross Expiration
                            if l1['opt_type'] == l2['opt_type']:
                                # Identify Short vs Long based on action
                                short_leg = l1 if l1['action'] == 'Sell to Open' else l2
                                long_leg = l2 if l1['action'] == 'Sell to Open' else l1

                                # STRATEGY DETECTION: Calendar vs Diagonal
                                if l1['strike'] == l2['strike']:
                                    spread_type = 'calendar_spread'
                                else:
                                    # Different strikes + Different expirations = Diagonal
                                    # If Long is lower strike (Call) or higher strike (Put) -> Bullish/Long Diagonal
                                    is_call = l1['opt_type'] == 'call'
                                    if (is_call and long_leg['strike'] < short_leg['strike']) or \
                                       (not is_call and long_leg['strike'] > short_leg['strike']):
                                        spread_type = 'long_diagonal_spread'
                                    else:
                                        spread_type = 'short_diagonal_spread'

                                qty = min(short_leg['qty'], long_leg['qty'])
                                credit = short_leg['price'] - long_leg['price']

                                short_close, short_date, short_reason, short_cfee = find_close(closes, expirations, short_leg, used_closes, used_expirations)
                                long_close, long_date, long_reason, long_cfee = find_close(closes, expirations, long_leg, used_closes, used_expirations)
                                close_date = short_date or long_date
                                exit_reason = short_reason or long_reason
                                close_comm = short_cfee + long_cfee

                                api_legs = [
                                    {'type': f"short_{short_leg['opt_type']}", 'strike': short_leg['strike'],
                                     'premium': short_leg['price'], 'closePremium': short_close},
                                    {'type': f"long_{long_leg['opt_type']}", 'strike': long_leg['strike'],
                                     'premium': long_leg['price'], 'closePremium': long_close}
                                ]

                                if spread_type != 'calendar_spread':
                                    print(f"  Detected Diagonal: {ticker} {l1['opt_type']} {long_leg['strike']}/{short_leg['strike']}")

                                trade = build_trade_dict(ticker, spread_type, qty, short_leg['strike'], long_leg['strike'],
                                                         short_leg['price'], long_leg['price'], credit, trade_date,
                                                         short_leg['expiration'], short_leg['fees'] + long_leg['fees'],
                                                         api_legs, close_date, exit_reason, close_comm,
                                                         back_expiration=long_leg['expiration'])
                                trades.append(trade)

                                # Remove consumed legs
                                legs1.remove(l1)
                                legs2.remove(l2)
                                matched = True
                                break
                        if matched: break

    # 3. Singles (CSP/CC) from final leftovers
        final_leftovers = []
        for l_list in leftover_by_exp.values():
            final_leftovers.extend(l_list)

        for leg in final_leftovers:
            leg_close, leg_date, leg_reason, leg_cfee = find_close(closes, expirations, leg, used_closes, used_expirations)

            if leg['opt_type'] == 'put' and leg['action'] == 'Sell to Open':
                # Cash Secured Put
                api_legs = [{'type': 'short_put', 'strike': leg['strike'], 'premium': leg['price'], 'closePremium': leg_close}]
                trade = build_trade_dict(ticker, 'cash_secured_put', leg['qty'], leg['strike'], 0,
                                         leg['price'], 0, leg['price'], trade_date, leg['expiration'],
                                         leg['fees'], api_legs, leg_date, leg_reason, leg_cfee)
                trades.append(trade)
            elif leg['opt_type'] == 'call' and leg['action'] == 'Sell to Open':
                # Covered Call
                api_legs = [{'type': 'short_call', 'strike': leg['strike'], 'premium': leg['price'], 'closePremium': leg_close}]
                trade = build_trade_dict(ticker, 'covered_call', leg['qty'], leg['strike'], 0,
                                         leg['price'], 0, leg['price'], trade_date, leg['expiration'],
                                         leg['fees'], api_legs, leg_date, leg_reason, leg_cfee)
                trades.append(trade)
            elif leg['opt_type'] == 'put' and leg['action'] == 'Buy to Open':
                # Long Put (standalone: protective put, LEAP, etc.)
                api_legs = [{'type': 'long_put', 'strike': leg['strike'], 'premium': leg['price'], 'closePremium': leg_close}]
                trade = build_trade_dict(ticker, 'long_put', leg['qty'], 0, leg['strike'],
                                         0, leg['price'], -leg['price'], trade_date, leg['expiration'],
                                         leg['fees'], api_legs, leg_date, leg_reason, leg_cfee)
                trades.append(trade)
            elif leg['opt_type'] == 'call' and leg['action'] == 'Buy to Open':
                # Long Call (standalone: speculative call, LEAP, etc.)
                api_legs = [{'type': 'long_call', 'strike': leg['strike'], 'premium': leg['price'], 'closePremium': leg_close}]
                trade = build_trade_dict(ticker, 'long_call', leg['qty'], 0, leg['strike'],
                                         0, leg['price'], -leg['price'], trade_date, leg['expiration'],
                                         leg['fees'], api_legs, leg_date, leg_reason, leg_cfee)
                trades.append(trade)
            else:
                print(f"  WARNING: Unmatched single leg: {ticker} {leg['expiration'].strftime('%m/%d')} {leg['strike']} {leg['opt_type']} {leg['action']}")

    # Process 0DTE roll campaigns (intraday IC rolls, all same expiry)
    # These are routed here instead of through process_same_exp_legs because greedy
    # leg-pooling cannot reconstruct the correct roll chain from the transaction soup.
    for (date_str, ticker), txns in zero_dte_txns.items():
        print(f"  [0DTE Roll Campaign] {ticker} {date_str}: {len(txns)} transactions")
        roll_chain = build_0dte_roll_chain(txns, ticker, date_str)
        trades.extend(roll_chain)

    # Merge SPV + SCV pairs into Iron Condors (cross-date detection)
    # Handles ICs where the put side and call side were opened on different dates
    trades = merge_verticals_to_ic(trades)

    # DB-aware IC promotion: pair new SPV/SCV in this batch with an existing open
    # SCV/SPV already in the DB (incremental narrow-window imports use this path)
    trades = upgrade_verticals_to_ic(trades, conn)

    # REMOVED: merge_rolled_ic_sides(trades)
    # Merging rolled sides collapses campaign history. We want separate linked trades
    # so the Chain View (G) correctly shows the progression of the position.

    # Detect rolls: closed trade followed by new trade with same ticker on same/next day
    detect_rolls(trades)

    return trades


def merge_verticals_to_ic(trades: List[Dict]) -> List[Dict]:
    """
    Post-processing: merge SPV + SCV pairs with same ticker/expiry/qty into Iron Condors.
    Handles cases where the put side and call side were opened on different dates.
    Also handles slight expiration date string mismatches by normalizing to YYYY-MM-DD.
    """
    # Index verticals by (ticker, normalized expiration date YYYY-MM-DD)
    spv_indices = defaultdict(list)  # key -> list of indices
    scv_indices = defaultdict(list)
    for i, t in enumerate(trades):
        # Normalize expiration to YYYY-MM-DD for robust matching
        exp_normalized = t['expirationDate'][:10]
        key = (t['ticker'], exp_normalized)
        if t['spreadType'] == 'short_put_vertical':
            spv_indices[key].append(i)
        elif t['spreadType'] == 'short_call_vertical':
            scv_indices[key].append(i)

    if spv_indices and scv_indices:
        spv_tickers = set(k[0] for k in spv_indices.keys())
        scv_tickers = set(k[0] for k in scv_indices.keys())
        common = spv_tickers & scv_tickers
        print(f"  IC merge candidates: {len(spv_indices)} SPV, {len(scv_indices)} SCV, {len(common)} common tickers")

    merged_out = set()  # indices consumed by merging
    new_trades = []

    for key in set(spv_indices.keys()) & set(scv_indices.keys()):
        for si in spv_indices[key]:
            if si in merged_out:
                continue
            spv = trades[si]
            for ci in scv_indices[key]:
                if ci in merged_out:
                    continue
                scv = trades[ci]
                if spv['quantity'] != scv['quantity']:
                    continue
                # Only merge if entries are within IC_MERGE_MAX_DAYS — prevents matching unrelated
                # trades that happen to share ticker+expiration (e.g. SPV opened 25+ days after SCV)
                date_spv = datetime.strptime(spv['tradeDate'][:10], '%Y-%m-%d')
                date_scv = datetime.strptime(scv['tradeDate'][:10], '%Y-%m-%d')
                if abs((date_spv - date_scv).days) > IC_MERGE_MAX_DAYS:
                    continue
                # Match found — merge into IC
                sp_leg = next(l for l in spv['legs'] if l['type'] == 'short_put')
                lp_leg = next(l for l in spv['legs'] if l['type'] == 'long_put')
                sc_leg = next(l for l in scv['legs'] if l['type'] == 'short_call')
                lc_leg = next(l for l in scv['legs'] if l['type'] == 'long_call')

                ic_legs = [
                    {'type': 'short_put', 'strike': sp_leg['strike'], 'premium': sp_leg['premium'], 'closePremium': sp_leg.get('closePremium')},
                    {'type': 'long_put', 'strike': lp_leg['strike'], 'premium': lp_leg['premium'], 'closePremium': lp_leg.get('closePremium')},
                    {'type': 'short_call', 'strike': sc_leg['strike'], 'premium': sc_leg['premium'], 'closePremium': sc_leg.get('closePremium')},
                    {'type': 'long_call', 'strike': lc_leg['strike'], 'premium': lc_leg['premium'], 'closePremium': lc_leg.get('closePremium')},
                ]
                put_credit = sp_leg['premium'] - lp_leg['premium']
                call_credit = sc_leg['premium'] - lc_leg['premium']
                total_credit = round(put_credit + call_credit, 4)

                # Use earlier entry date
                entry_spv = spv['entryTime'][:10]
                entry_scv = scv['entryTime'][:10]
                entry_date = min(entry_spv, entry_scv)

                # Determine close status
                is_closed = all(l.get('closePremium') is not None for l in ic_legs)
                ic = {
                    'ticker': spv['ticker'],
                    'spreadType': 'iron_condor',
                    'quantity': spv['quantity'],
                    'shortStrike': sp_leg['strike'],
                    'longStrike': lp_leg['strike'],
                    'shortPremium': sp_leg['premium'],
                    'longPremium': lp_leg['premium'],
                    'creditReceived': round(total_credit, 2),
                    'entryTime': f"{entry_date}T10:00:00Z",
                    'expirationDate': spv['expirationDate'],
                    'tradeDate': entry_date,  # plain YYYY-MM-DD, not ISO datetime
                    'commission': round((spv.get('commission', 0) or 0) + (scv.get('commission', 0) or 0), 2),
                    'legs': ic_legs,
                }
                if is_closed:
                    exit_spv = spv.get('exitTime')
                    exit_scv = scv.get('exitTime')
                    exits = [x for x in [exit_spv, exit_scv] if x]
                    if exits:
                        ic['exitTime'] = max(exits)
                    reason = spv.get('exitReason') or scv.get('exitReason')
                    if reason:
                        ic['exitReason'] = reason

                # Carry over greeks from the SPV side (if enriched)
                for gk in ['delta', 'theta', 'gamma', 'vega', 'pop', 'underlyingPrice', 'ivRank', 'underlyingPriceAtClose']:
                    v = spv.get(gk) or scv.get(gk)
                    if v is not None:
                        ic[gk] = v

                print(f"  Merged IC: {spv['ticker']} SPV({sp_leg['strike']}/{lp_leg['strike']}) + SCV({sc_leg['strike']}/{lc_leg['strike']}) exp={spv['expirationDate'][:10]} qty={spv['quantity']}")
                new_trades.append(ic)
                merged_out.add(si)
                merged_out.add(ci)
                break  # This SPV is consumed, move to next

    if not new_trades:
        return trades

    # Rebuild: keep non-merged trades + add new ICs
    result = [t for i, t in enumerate(trades) if i not in merged_out]
    result.extend(new_trades)
    print(f"  Merged {len(new_trades)} Iron Condor(s) from separate verticals")
    return result


def upgrade_verticals_to_ic(trades: List[Dict], conn: Optional[sqlite3.Connection]) -> List[Dict]:
    """
    DB-aware IC promotion: if a new SPV/SCV in this batch matches an open standalone
    SCV/SPV already in the DB (same ticker, expiry, qty), upgrade the DB record to
    iron_condor in-place and remove the batch trade from further processing.

    Called after merge_verticals_to_ic() so in-batch IC merges are handled first.
    No IC_MERGE_MAX_DAYS limit: narrow-window incremental imports are intentional;
    same expiry + qty is sufficient to avoid false positives.
    """
    if conn is None:
        return trades

    # Fetch all open standalone verticals from DB
    conn.row_factory = sqlite3.Row
    db_rows = conn.execute(
        """SELECT * FROM trades
           WHERE strategy IN ('short_call_vertical', 'short_put_vertical')
             AND exit_date IS NULL"""
    ).fetchall()

    db_scv_by_key: Dict[tuple, Dict] = {}  # (ticker, exp_YYYY-MM-DD, qty) -> db trade dict
    db_spv_by_key: Dict[tuple, Dict] = {}

    for row in db_rows:
        t = dict(row)
        t['legs'] = json.loads(t.get('legs_json', '[]'))
        # Skip DB verticals with any closePremium set — partially closed, do not merge
        if any(l.get('closePremium') is not None for l in t['legs']):
            continue
        exp_norm = t['expiration_date'][:10]
        key = (t['ticker'], exp_norm, t['quantity'])
        if t['strategy'] == 'short_call_vertical':
            db_scv_by_key[key] = t
        elif t['strategy'] == 'short_put_vertical':
            db_spv_by_key[key] = t

    if not db_scv_by_key and not db_spv_by_key:
        return trades

    consumed: set = set()

    for i, trade in enumerate(trades):
        if i in consumed:
            continue
        exp_norm = trade['expirationDate'][:10]
        key = (trade['ticker'], exp_norm, trade['quantity'])

        # CAUTIOUS PROMOTION: Only promote if there isn't already a complex campaign
        # (prevents re-merging manual splits like IWM/QQQ)
        ticker_campaign_count = sum(1 for row in db_rows if row['ticker'] == trade['ticker'] and row['expiration_date'][:10] == exp_norm)
        if ticker_campaign_count > 1:
            continue

        if trade['spreadType'] == 'short_put_vertical' and key in db_scv_by_key:
            db_rec = db_scv_by_key.pop(key)  # pop prevents double-use
            promote_to_ic(conn, db_rec, trade, new_is_put=True)
            consumed.add(i)

        elif trade['spreadType'] == 'short_call_vertical' and key in db_spv_by_key:
            db_rec = db_spv_by_key.pop(key)
            promote_to_ic(conn, db_rec, trade, new_is_put=False)
            consumed.add(i)

    if consumed:
        print(f"  DB-promoted {len(consumed)} IC(s) from existing standalone vertical(s)")

    return [t for i, t in enumerate(trades) if i not in consumed]


def promote_to_ic(conn: sqlite3.Connection, db_record: Dict, new_side: Dict, new_is_put: bool) -> None:
    """
    Upgrade an existing standalone DB vertical (SCV or SPV) to iron_condor in-place
    by merging it with a new counterpart arriving in the current batch.

    db_record  - existing DB trade dict (has 'id', 'strategy', 'legs', 'credit_received', etc.)
    new_side   - new batch trade dict (SPV if new_is_put=True, SCV if new_is_put=False)
    new_is_put - True if new_side is the put side (SPV pairing with DB SCV)
               - False if new_side is the call side (SCV pairing with DB SPV)
    """
    ticker = db_record['ticker']
    qty    = db_record['quantity']
    db_legs = db_record['legs']

    if new_is_put:
        # New SPV arrives; DB holds the SCV (call side)
        sp_leg = next(l for l in new_side['legs'] if l['type'] == 'short_put')
        lp_leg = next(l for l in new_side['legs'] if l['type'] == 'long_put')
        sc_leg = next(l for l in db_legs if l['type'] == 'short_call')
        lc_leg = next(l for l in db_legs if l['type'] == 'long_call')

        put_credit   = sp_leg['premium'] - lp_leg['premium']
        call_credit  = db_record['credit_received']  # pure call credit stored in DB
        total_credit = round(put_credit + call_credit, 2)
        ic_trade_date = min(db_record['trade_date'][:10], new_side['tradeDate'][:10])

        ic_legs = [
            {'type': 'short_put',  'strike': sp_leg['strike'], 'premium': sp_leg['premium'],
             'closePremium': None, 'expirationDate': sp_leg.get('expirationDate')},
            {'type': 'long_put',   'strike': lp_leg['strike'], 'premium': lp_leg['premium'],
             'closePremium': None, 'expirationDate': lp_leg.get('expirationDate')},
            {'type': 'short_call', 'strike': sc_leg['strike'], 'premium': sc_leg['premium'],
             'closePremium': None, 'expirationDate': sc_leg.get('expirationDate')},
            {'type': 'long_call',  'strike': lc_leg['strike'], 'premium': lc_leg['premium'],
             'closePremium': None, 'expirationDate': lc_leg.get('expirationDate')},
        ]
        put_width    = abs(sp_leg['strike'] - lp_leg['strike'])
        call_width   = abs(sc_leg['strike'] - lc_leg['strike'])
        spread_width = max(put_width, call_width) if (put_width or call_width) else None
        bpr          = (spread_width - total_credit) * 100 * qty if spread_width else None

        conn.execute(
            """UPDATE trades SET
                 strategy        = 'iron_condor',
                 short_strike    = ?,
                 long_strike     = ?,
                 short_premium   = ?,
                 long_premium    = ?,
                 credit_received = ?,
                 trade_date      = ?,
                 entry_date      = ?,
                 spread_width    = ?,
                 bpr             = ?,
                 legs_json       = ?
               WHERE id = ?""",
            (sp_leg['strike'], lp_leg['strike'], sp_leg['premium'], lp_leg['premium'],
             total_credit, ic_trade_date + 'T10:00:00Z', ic_trade_date + 'T10:00:00Z',
             spread_width, bpr, json.dumps(ic_legs), db_record['id'])
        )

    else:
        # New SCV arrives; DB holds the SPV (put side)
        sp_leg = next(l for l in db_legs if l['type'] == 'short_put')
        lp_leg = next(l for l in db_legs if l['type'] == 'long_put')
        sc_leg = next(l for l in new_side['legs'] if l['type'] == 'short_call')
        lc_leg = next(l for l in new_side['legs'] if l['type'] == 'long_call')

        put_credit   = db_record['credit_received']  # pure put credit stored in DB
        call_credit  = new_side['creditReceived']
        total_credit = round(put_credit + call_credit, 2)
        ic_trade_date = min(db_record['trade_date'][:10], new_side['tradeDate'][:10])

        ic_legs = [
            {'type': 'short_put',  'strike': sp_leg['strike'], 'premium': sp_leg['premium'],
             'closePremium': None, 'expirationDate': sp_leg.get('expirationDate')},
            {'type': 'long_put',   'strike': lp_leg['strike'], 'premium': lp_leg['premium'],
             'closePremium': None, 'expirationDate': lp_leg.get('expirationDate')},
            {'type': 'short_call', 'strike': sc_leg['strike'], 'premium': sc_leg['premium'],
             'closePremium': None, 'expirationDate': sc_leg.get('expirationDate')},
            {'type': 'long_call',  'strike': lc_leg['strike'], 'premium': lc_leg['premium'],
             'closePremium': None, 'expirationDate': lc_leg.get('expirationDate')},
        ]
        put_width    = abs(sp_leg['strike'] - lp_leg['strike'])
        call_width   = abs(sc_leg['strike'] - lc_leg['strike'])
        spread_width = max(put_width, call_width) if (put_width or call_width) else None
        bpr          = (spread_width - total_credit) * 100 * qty if spread_width else None

        conn.execute(
            """UPDATE trades SET
                 strategy        = 'iron_condor',
                 credit_received = ?,
                 trade_date      = ?,
                 entry_date      = ?,
                 spread_width    = ?,
                 bpr             = ?,
                 legs_json       = ?
               WHERE id = ?""",
            (total_credit, ic_trade_date + 'T10:00:00Z', ic_trade_date + 'T10:00:00Z',
             spread_width, bpr, json.dumps(ic_legs), db_record['id'])
        )

    conn.commit()

    # Write batch trade's fingerprint → blocks standalone re-import of this leg
    batch_fp = get_trade_fingerprint(new_side)
    conn.execute(
        "INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)",
        (batch_fp, db_record['id'])
    )
    # Write IC fingerprint → blocks IC re-creation on overlapping future runs
    ic_fp = f"{ic_trade_date}|{ticker}|{qty}|{total_credit:.2f}"
    conn.execute(
        "INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)",
        (ic_fp, db_record['id'])
    )
    conn.commit()

    # Recalculate Greeks for the new IC using stored underlying_price and vix_at_entry.
    # This prevents stale Greeks from the original standalone vertical being carried over.
    _s = db_record.get('underlying_price')
    _vix = db_record.get('vix_at_entry')
    _exp = db_record.get('expiration_date', '')[:10]
    if _s and _exp:
        _trade_dict = {
            'legs':           ic_legs,
            'tradeDate':      ic_trade_date,
            'expirationDate': _exp,
        }
        _ticker_prices = {ic_trade_date: _s}
        _vix_prices    = {ic_trade_date: _vix} if _vix is not None else {}
        _greeks = estimate_trade_greeks(_trade_dict, _ticker_prices, _vix_prices)
        if _greeks:
            conn.execute(
                "UPDATE trades SET theta=?, delta=?, gamma=?, vega=? WHERE id=?",
                (_greeks['theta'], _greeks['delta'], _greeks['gamma'], _greeks['vega'],
                 db_record['id'])
            )
            conn.commit()

    print(f"  DB-promoted IC: {ticker} #{db_record['id']} "
          f"({db_record['strategy']} → iron_condor) "
          f"put={sp_leg['strike']}/{lp_leg['strike']} "
          f"call={sc_leg['strike']}/{lc_leg['strike']} "
          f"total_cr=${total_credit:.2f}")


def merge_rolled_ic_sides(trades: List[Dict]) -> List[Dict]:
    """
    Handle the case where one side of an IC was rolled to new strikes on a later date.
    Example: ORCL IC opened 02/13 (135P/125P + 190C/200C). On 02/23 the call side
    was rolled (BTC 190C + STC 200C → STO 175C + BTO 185C). The 02/13 IC now has
    its call legs with closePremium set (they were closed), and a new SCV (175C/185C)
    exists for 02/23. We merge: keep 02/13 put legs + 02/23 call legs → one IC dated 02/23.
    """
    # Index standalone verticals by (ticker, normalized expiration)
    scv_by_key = {}  # (ticker, exp) -> list of (index, trade)
    spv_by_key = {}

    for i, t in enumerate(trades):
        exp_norm = t['expirationDate'][:10]
        key = (t['ticker'], exp_norm)
        if t['spreadType'] == 'short_call_vertical':
            scv_by_key.setdefault(key, []).append((i, t))
        elif t['spreadType'] == 'short_put_vertical':
            spv_by_key.setdefault(key, []).append((i, t))

    merged_out = set()
    new_trades = []

    for i, t in enumerate(trades):
        if t['spreadType'] != 'iron_condor' or i in merged_out:
            continue

        ic_legs = t.get('legs', [])
        call_legs = [l for l in ic_legs if 'call' in l['type']]
        put_legs  = [l for l in ic_legs if 'put'  in l['type']]

        # Check if call side was rolled (all call legs have closePremium, puts still open)
        call_side_rolled = call_legs and all(l.get('closePremium') is not None for l in call_legs)
        put_side_open    = put_legs  and all(l.get('closePremium') is None for l in put_legs)

        # Check if put side was rolled (all put legs have closePremium, calls still open)
        put_side_rolled  = put_legs  and all(l.get('closePremium') is not None for l in put_legs)
        call_side_open   = call_legs and all(l.get('closePremium') is None for l in call_legs)

        exp_norm = t['expirationDate'][:10]
        key = (t['ticker'], exp_norm)
        date_ic = datetime.strptime(t['tradeDate'][:10], '%Y-%m-%d')

        if call_side_rolled and put_side_open and key in scv_by_key:
            # Find a SCV opened AFTER this IC to serve as the new call side
            for j, scv in scv_by_key[key]:
                if j in merged_out:
                    continue
                if scv['quantity'] != t['quantity']:
                    continue
                date_scv = datetime.strptime(scv['tradeDate'][:10], '%Y-%m-%d')
                if date_scv <= date_ic:
                    continue  # Must be opened after the IC
                # Found the rolled call side — merge
                sp_leg = next(l for l in put_legs if l['type'] == 'short_put')
                lp_leg = next(l for l in put_legs if l['type'] == 'long_put')
                sc_leg = next(l for l in scv['legs'] if l['type'] == 'short_call')
                lc_leg = next(l for l in scv['legs'] if l['type'] == 'long_call')

                new_legs = [
                    {'type': 'short_put',  'strike': sp_leg['strike'], 'premium': sp_leg['premium'], 'closePremium': None},
                    {'type': 'long_put',   'strike': lp_leg['strike'], 'premium': lp_leg['premium'], 'closePremium': None},
                    {'type': 'short_call', 'strike': sc_leg['strike'], 'premium': sc_leg['premium'], 'closePremium': None},
                    {'type': 'long_call',  'strike': lc_leg['strike'], 'premium': lc_leg['premium'], 'closePremium': None},
                ]

                # Net credit = put_credit + (original_call_credit - cost_to_close_calls) + new_call_credit
                put_credit       = sp_leg['premium'] - lp_leg['premium']
                orig_call_sc     = next(l for l in call_legs if l['type'] == 'short_call')
                orig_call_lc     = next(l for l in call_legs if l['type'] == 'long_call')
                orig_call_credit = orig_call_sc['premium'] - orig_call_lc['premium']
                call_close_cost  = orig_call_sc.get('closePremium', 0) - orig_call_lc.get('closePremium', 0)
                new_call_credit  = sc_leg['premium'] - lc_leg['premium']
                total_credit     = round(put_credit + orig_call_credit - call_close_cost + new_call_credit, 2)

                # Preserving original entry date for campaign tracking
                roll_date = scv['tradeDate'][:10]
                ic = dict(t)
                ic['legs']           = new_legs
                ic['creditReceived'] = total_credit
                ic['shortStrike']    = sp_leg['strike']
                ic['longStrike']     = lp_leg['strike']
                ic['shortPremium']   = sp_leg['premium']
                ic['longPremium']    = lp_leg['premium']
                # Keep original tradeDate and entryTime from 't'
                
                # Update notes with roll history
                old_notes = t.get('notes') or ""
                roll_note = f"Rolled calls on {roll_date} to {sc_leg['strike']}/{lc_leg['strike']}. Total credit updated to {total_credit:+.2f}."
                ic['notes'] = f"{old_notes}\n{roll_note}".strip()
                # Clear exitTime — the IC is not closed
                ic.pop('exitTime', None)
                ic.pop('exitReason', None)

                print(f"  Rolled IC merge: {t['ticker']} kept puts ({sp_leg['strike']}/{lp_leg['strike']}) + rolled calls to ({sc_leg['strike']}/{lc_leg['strike']}) cr={total_credit:+.2f} dated {roll_date}")
                
                # IMPORTANT: Record fingerprint for the standalone vertical too!
                # This prevents it from being imported separately on future runs
                if conn:
                    v_fp = get_trade_fingerprint(scv)
                    conn.execute("INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)", (v_fp, t.get('id')))
                    conn.commit()

                new_trades.append(ic)
                merged_out.add(i)   # remove original IC
                merged_out.add(j)   # remove standalone SCV
                break

        elif put_side_rolled and call_side_open and key in spv_by_key:
            # Mirror: put side was rolled — find a new SPV opened after the IC
            for j, spv in spv_by_key[key]:
                if j in merged_out:
                    continue
                if spv['quantity'] != t['quantity']:
                    continue
                date_spv = datetime.strptime(spv['tradeDate'][:10], '%Y-%m-%d')
                if date_spv <= date_ic:
                    continue
                sc_leg = next(l for l in call_legs if l['type'] == 'short_call')
                lc_leg = next(l for l in call_legs if l['type'] == 'long_call')
                sp_leg = next(l for l in spv['legs'] if l['type'] == 'short_put')
                lp_leg = next(l for l in spv['legs'] if l['type'] == 'long_put')

                new_legs = [
                    {'type': 'short_put',  'strike': sp_leg['strike'], 'premium': sp_leg['premium'], 'closePremium': None},
                    {'type': 'long_put',   'strike': lp_leg['strike'], 'premium': lp_leg['premium'], 'closePremium': None},
                    {'type': 'short_call', 'strike': sc_leg['strike'], 'premium': sc_leg['premium'], 'closePremium': None},
                    {'type': 'long_call',  'strike': lc_leg['strike'], 'premium': lc_leg['premium'], 'closePremium': None},
                ]

                call_credit      = sc_leg['premium'] - lc_leg['premium']
                orig_put_sp      = next(l for l in put_legs if l['type'] == 'short_put')
                orig_put_lp      = next(l for l in put_legs if l['type'] == 'long_put')
                orig_put_credit  = orig_put_sp['premium'] - orig_put_lp['premium']
                put_close_cost   = orig_put_sp.get('closePremium', 0) - orig_put_lp.get('closePremium', 0)
                new_put_credit   = sp_leg['premium'] - lp_leg['premium']
                total_credit     = round(call_credit + orig_put_credit - put_close_cost + new_put_credit, 2)

                # Preserving original entry date for campaign tracking
                roll_date = spv['tradeDate'][:10]
                ic = dict(t)
                ic['legs']           = new_legs
                ic['creditReceived'] = total_credit
                ic['shortStrike']    = sp_leg['strike']
                ic['longStrike']     = lp_leg['strike']
                ic['shortPremium']   = sp_leg['premium']
                ic['longPremium']    = lp_leg['premium']
                # Keep original tradeDate and entryTime from 't'

                # Update notes with roll history
                old_notes = t.get('notes') or ""
                roll_note = f"Rolled puts on {roll_date} to {sp_leg['strike']}/{lp_leg['strike']}. Total credit updated to {total_credit:+.2f}."
                ic['notes'] = f"{old_notes}\n{roll_note}".strip()
                ic.pop('exitTime', None)
                ic.pop('exitReason', None)


                print(f"  Rolled IC merge: {t['ticker']} kept calls ({sc_leg['strike']}/{lc_leg['strike']}) + rolled puts to ({sp_leg['strike']}/{lp_leg['strike']}) cr={total_credit:+.2f} dated {roll_date}")
                
                # IMPORTANT: Record fingerprint for the standalone vertical too!
                if conn:
                    v_fp = get_trade_fingerprint(spv)
                    conn.execute("INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)", (v_fp, t.get('id')))
                    conn.commit()

                new_trades.append(ic)
                merged_out.add(i)
                merged_out.add(j)
                break

    if not new_trades:
        return trades

    result = [t for i, t in enumerate(trades) if i not in merged_out]
    result.extend(new_trades)
    print(f"  Merged {len(new_trades)} rolled IC side(s)")
    return result


def detect_rolls(trades: List[Dict]) -> None:
    """
    Detect rolled trades: when a position is closed and a new one opened
    on the same day with the same ticker.
    Modifies trades in place to set exitReason='rolled'.
    """
    # Get closed trades with 'closed' exit reason (not expired)
    # Skip 0DTE chain trades — their exitReason is set directly by build_0dte_roll_chain
    closed_trades = [t for t in trades if t.get('exitReason') == 'closed'
                     and not t.get('_is_0dte_chain')]

    for closed in closed_trades:
        close_date = closed.get('exitTime', '')[:10]  # YYYY-MM-DD
        ticker = closed['ticker']

        # Find a trade that was opened on the close date (potential roll target)
        for candidate in trades:
            if candidate is closed:
                continue
            if candidate['ticker'] != ticker:
                continue

            open_date = candidate['tradeDate'][:10]

            # Roll: new trade opened on same day as close (or within 1 day)
            if open_date == close_date:
                # This is a roll!
                closed['exitReason'] = 'rolled'
                break

def get_trade_fingerprint(trade: Dict) -> str:
    """Create a unique fingerprint for a trade to detect duplicates."""
    date = trade['tradeDate'][:10]
    ticker = trade['ticker']
    qty = trade['quantity']
    credit = f"{trade['creditReceived']:.2f}"

    return f"{date}|{ticker}|{qty}|{credit}"


def filter_new_trades(trades: List[Dict], existing_fingerprints: set) -> List[Dict]:
    """Filter out trades that already exist in the database, unless they are updates."""
    if not existing_fingerprints:
        return trades

    print(f"Filtering against {len(existing_fingerprints)} existing trades...")
    new_trades = []
    for t in trades:
        # Updates must always pass through — their regular fingerprint already exists
        # in import_log from the original open import, but the oc| guard in build_trades
        # already ensures we don't double-close.
        if t.get('is_update') or t.get('id'):
            new_trades.append(t)
            continue

        fp = get_trade_fingerprint(t)
        if fp in existing_fingerprints:
            continue

        new_trades.append(t)

    skipped = len(trades) - len(new_trades)
    if skipped > 0:
        print(f"  Skipped {skipped} duplicate trades.")
    return new_trades


def assign_playbook_ids(trades: List[Dict], spread_to_id: Dict[str, int]) -> int:
    """Assign playbookId to trades based on their spreadType. Returns count assigned."""
    assigned = 0
    for trade in trades:
        st = trade.get('spreadType', '')
        if st in spread_to_id:
            trade['playbookId'] = spread_to_id[st]
            assigned += 1
    return assigned


# ---------- Computed Fields ----------

def compute_pnl_and_debit(trade: Dict) -> Tuple[Optional[float], Optional[float]]:
    """Compute P&L and debit_paid for a closed trade from leg close premiums.
    Returns (pnl, debit_paid). Both None if not closed.
    Commission is included in P&L to match OptionsTradingJournal exactly."""
    legs = trade.get('legs', [])
    if not legs:
        return None, None
    if not all(l.get('closePremium') is not None for l in legs):
        return None, None

    qty = trade['quantity']
    # Debit to close: what we pay net to close (positive = paid)
    # For each short leg we buy back (pay close premium)
    # For each long leg we sell back (receive close premium)
    close_credit = 0.0
    for l in legs:
        cp = l.get('closePremium', 0) or 0
        if l['type'].startswith('short_'):
            close_credit -= cp   # paid to close short
        else:
            close_credit += cp   # received closing long

    # debit_paid is the net cost to close (positive means we paid)
    debit_paid = -close_credit  # if we received more than paid, debit is negative

    # P&L = (credit_received - debit_to_close) * 100 * qty - commission
    # Commission included to match OTJ (no rounding — true decimal figures)
    commission = trade.get('commission') or 0.0
    pnl = (trade['creditReceived'] + close_credit) * 100.0 * qty - commission
    return pnl, debit_paid


def compute_spread_width(trade: Dict) -> Optional[float]:
    """Width of the widest leg pair."""
    spread_type = trade['spreadType']
    legs = trade.get('legs', [])
    short_strike = trade.get('shortStrike', 0) or 0
    long_strike  = trade.get('longStrike', 0) or 0

    if spread_type in ('short_put_vertical', 'short_call_vertical',
                       'long_put_vertical', 'long_call_vertical'):
        return abs(short_strike - long_strike) if short_strike and long_strike else None

    if spread_type == 'iron_condor':
        puts  = [(l['strike'], l['type']) for l in legs if 'put'  in l['type']]
        calls = [(l['strike'], l['type']) for l in legs if 'call' in l['type']]
        put_width  = abs(puts[0][0]  - puts[1][0])  if len(puts)  >= 2 else 0
        call_width = abs(calls[0][0] - calls[1][0]) if len(calls) >= 2 else 0
        return max(put_width, call_width) if (put_width or call_width) else None

    return None


def compute_bpr(trade: Dict) -> Optional[float]:
    """Buying Power Reduction."""
    spread_type   = trade['spreadType']
    qty           = trade['quantity']
    credit        = trade['creditReceived']
    underlying    = trade.get('underlyingPrice')
    short_strike  = trade.get('shortStrike', 0) or 0
    long_strike   = trade.get('longStrike', 0) or 0
    legs          = trade.get('legs', [])

    if spread_type in ('short_put_vertical', 'short_call_vertical',
                       'long_put_vertical', 'long_call_vertical'):
        width = abs(short_strike - long_strike)
        return (width - credit) * 100.0 * qty

    if spread_type == 'iron_condor':
        puts  = [l for l in legs if 'put'  in l['type']]
        calls = [l for l in legs if 'call' in l['type']]
        put_width  = abs(puts[0]['strike']  - puts[1]['strike'])  if len(puts)  >= 2 else 0
        call_width = abs(calls[0]['strike'] - calls[1]['strike']) if len(calls) >= 2 else 0
        max_width  = max(put_width, call_width)
        return (max_width - credit) * 100.0 * qty

    if spread_type == 'iron_butterfly':
        puts  = [l for l in legs if 'put'  in l['type']]
        calls = [l for l in legs if 'call' in l['type']]
        width = max(
            abs(puts[0]['strike']  - puts[1]['strike'])  if len(puts)  >= 2 else 0,
            abs(calls[0]['strike'] - calls[1]['strike']) if len(calls) >= 2 else 0,
        )
        return (width - credit) * 100.0 * qty

    if spread_type == 'cash_secured_put':
        return short_strike * 100.0 * qty

    if spread_type == 'covered_call':
        return short_strike * 100.0 * qty

    # Strangle / Straddle — Reg-T approximation
    if spread_type in ('strangle', 'straddle') and underlying:
        sp_leg = next((l for l in legs if l['type'] == 'short_put'),  None)
        sc_leg = next((l for l in legs if l['type'] == 'short_call'), None)
        if sp_leg and sc_leg:
            up = underlying
            sp_prem = sp_leg['premium']
            sc_prem = sc_leg['premium']
            put_otm  = max(up - sp_leg['strike'], 0)
            call_otm = max(sc_leg['strike'] - up, 0)
            put_margin  = max(0.20 * up - put_otm  + sp_prem, 0.10 * up + sp_prem)
            call_margin = max(0.20 * up - call_otm + sc_prem, 0.10 * up + sc_prem)
            margin = (put_margin + sc_prem) if put_margin >= call_margin else (call_margin + sp_prem)
            return margin * 100.0 * qty

    if spread_type in ('calendar_spread', 'long_diagonal_spread', 'short_diagonal_spread', 'pmcc'):
        # Debit paid = abs(credit) when credit is negative
        return abs(credit) * 100.0 * qty

    return None


def compute_dte(trade_date_str: str, expiration_str: str) -> Optional[int]:
    """DTE at entry."""
    try:
        td  = datetime.strptime(trade_date_str[:10], '%Y-%m-%d')
        exp = datetime.fromisoformat(expiration_str.replace('Z', ''))
        return max((exp.date() - td.date()).days, 0)
    except Exception:
        return None


def compute_dte_at_close(exit_time: Optional[str], expiration_str: str) -> Optional[int]:
    """DTE at close."""
    if not exit_time:
        return None
    try:
        close_dt = datetime.fromisoformat(exit_time.replace('Z', ''))
        exp_dt   = datetime.fromisoformat(expiration_str.replace('Z', ''))
        return max((exp_dt.date() - close_dt.date()).days, 0)
    except Exception:
        return None


# ---------- SQLite Import ----------

def init_db(conn: sqlite3.Connection) -> None:
    """Create tables if they don't exist (mirrors Rust storage.rs schema)."""
    conn.executescript("""
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS trades (
            id                        INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker                    TEXT NOT NULL,
            strategy                  TEXT NOT NULL,
            quantity                  INTEGER NOT NULL,
            short_strike              REAL NOT NULL DEFAULT 0,
            long_strike               REAL NOT NULL DEFAULT 0,
            short_premium             REAL NOT NULL DEFAULT 0,
            long_premium              REAL NOT NULL DEFAULT 0,
            credit_received           REAL NOT NULL,
            entry_date                TEXT NOT NULL,
            exit_date                 TEXT,
            expiration_date           TEXT NOT NULL,
            trade_date                TEXT NOT NULL,
            back_month_expiration     TEXT,
            pnl                       REAL,
            debit_paid                REAL,
            delta                     REAL,
            theta                     REAL,
            gamma                     REAL,
            vega                      REAL,
            pop                       REAL,
            underlying_price          REAL,
            underlying_price_at_close REAL,
            iv_rank                   REAL,
            vix_at_entry              REAL,
            implied_volatility        REAL,
            commission                REAL,
            entry_reason              TEXT,
            exit_reason               TEXT,
            management_rule           TEXT,
            target_profit_pct         REAL,
            spread_width              REAL,
            bpr                       REAL,
            entry_dte                 INTEGER,
            dte_at_close              INTEGER,
            playbook_id               INTEGER,
            rolled_from_id            INTEGER,
            is_earnings_play          INTEGER NOT NULL DEFAULT 0,
            is_tested                 INTEGER NOT NULL DEFAULT 0,
            trade_grade               TEXT,
            grade_notes               TEXT,
            legs_json                 TEXT NOT NULL DEFAULT '[]',
            tags                      TEXT NOT NULL DEFAULT '',
            notes                     TEXT,
            UNIQUE(trade_date, ticker, strategy, short_strike, long_strike, quantity)
        );

        CREATE TABLE IF NOT EXISTS playbook_strategies (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            name                TEXT NOT NULL,
            description         TEXT,
            spread_type         TEXT,
            entry_criteria_json TEXT
        );

        CREATE TABLE IF NOT EXISTS import_log (
            fingerprint TEXT PRIMARY KEY,
            trade_id    INTEGER,
            imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
    """)


def migrate_db(conn: sqlite3.Connection) -> None:
    """Add any missing columns to existing DB."""
    cur = conn.execute("PRAGMA table_info(trades)")
    existing = {row[1] for row in cur.fetchall()}

    migrations = [
        ("back_month_expiration",     "ALTER TABLE trades ADD COLUMN back_month_expiration TEXT"),
        ("debit_paid",                "ALTER TABLE trades ADD COLUMN debit_paid REAL"),
        ("delta",                     "ALTER TABLE trades ADD COLUMN delta REAL"),
        ("theta",                     "ALTER TABLE trades ADD COLUMN theta REAL"),
        ("gamma",                     "ALTER TABLE trades ADD COLUMN gamma REAL"),
        ("vega",                      "ALTER TABLE trades ADD COLUMN vega REAL"),
        ("pop",                       "ALTER TABLE trades ADD COLUMN pop REAL"),
        ("underlying_price",          "ALTER TABLE trades ADD COLUMN underlying_price REAL"),
        ("underlying_price_at_close", "ALTER TABLE trades ADD COLUMN underlying_price_at_close REAL"),
        ("iv_rank",                   "ALTER TABLE trades ADD COLUMN iv_rank REAL"),
        ("vix_at_entry",              "ALTER TABLE trades ADD COLUMN vix_at_entry REAL"),
        ("implied_volatility",        "ALTER TABLE trades ADD COLUMN implied_volatility REAL"),
        ("commission",                "ALTER TABLE trades ADD COLUMN commission REAL"),
        ("entry_reason",              "ALTER TABLE trades ADD COLUMN entry_reason TEXT"),
        ("exit_reason",               "ALTER TABLE trades ADD COLUMN exit_reason TEXT"),
        ("management_rule",           "ALTER TABLE trades ADD COLUMN management_rule TEXT"),
        ("target_profit_pct",         "ALTER TABLE trades ADD COLUMN target_profit_pct REAL"),
        ("spread_width",              "ALTER TABLE trades ADD COLUMN spread_width REAL"),
        ("bpr",                       "ALTER TABLE trades ADD COLUMN bpr REAL"),
        ("entry_dte",                 "ALTER TABLE trades ADD COLUMN entry_dte INTEGER"),
        ("dte_at_close",              "ALTER TABLE trades ADD COLUMN dte_at_close INTEGER"),
        ("playbook_id",               "ALTER TABLE trades ADD COLUMN playbook_id INTEGER"),
        ("rolled_from_id",            "ALTER TABLE trades ADD COLUMN rolled_from_id INTEGER"),
        ("is_earnings_play",          "ALTER TABLE trades ADD COLUMN is_earnings_play INTEGER NOT NULL DEFAULT 0"),
        ("is_tested",                 "ALTER TABLE trades ADD COLUMN is_tested INTEGER NOT NULL DEFAULT 0"),
        ("trade_grade",               "ALTER TABLE trades ADD COLUMN trade_grade TEXT"),
        ("grade_notes",               "ALTER TABLE trades ADD COLUMN grade_notes TEXT"),
        ("legs_json",                 "ALTER TABLE trades ADD COLUMN legs_json TEXT NOT NULL DEFAULT '[]'"),
        ("tags",                      "ALTER TABLE trades ADD COLUMN tags TEXT NOT NULL DEFAULT ''"),
        ("notes",                     "ALTER TABLE trades ADD COLUMN notes TEXT"),
    ]
    for col_name, sql in migrations:
        if col_name not in existing:
            try:
                conn.execute(sql)
            except Exception as e:
                print(f"  Migration warning ({col_name}): {e}")

    # Create import_log table if it doesn't exist (not a column migration)
    conn.execute("""
        CREATE TABLE IF NOT EXISTS import_log (
            fingerprint TEXT PRIMARY KEY,
            trade_id    INTEGER,
            imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
    """)
    conn.commit()


def get_existing_fingerprints(conn: sqlite3.Connection) -> set:
    """Fetch all existing trade fingerprints for duplicate detection.

    Uses import_log as the primary source so fingerprints survive trade deletions
    and manual credit edits.  Falls back to a direct trades query so the first
    ever run (before import_log is populated) still works correctly.
    """
    fps = set()

    # Primary: import_log persists fingerprints even after deletion or credit changes
    try:
        for (fp,) in conn.execute("SELECT fingerprint FROM import_log").fetchall():
            fps.add(fp)
    except Exception:
        pass  # table not yet created; fall through to trades query

    # Fallback: current trades (catches trades inserted before import_log existed)
    for trade_date, ticker, qty, credit in conn.execute(
        "SELECT trade_date, ticker, quantity, credit_received FROM trades"
    ).fetchall():
        fp = f"{trade_date[:10]}|{ticker}|{qty}|{credit:.2f}"
        fps.add(fp)

    return fps


def get_open_trades_from_db(conn: sqlite3.Connection) -> List[Dict]:
    """Fetch currently open trades from the DB to match against new closing transactions."""
    conn.row_factory = sqlite3.Row
    cur = conn.execute("SELECT * FROM trades WHERE exit_date IS NULL")
    open_trades = []
    for row in cur.fetchall():
        t = dict(row)
        # Parse the legs JSON back into a list
        t['legs'] = json.loads(t.get('legs_json', '[]'))
        open_trades.append(t)
    return open_trades


def get_playbook_map(conn: sqlite3.Connection) -> Dict[str, int]:
    """Returns {spread_type: playbook_id} map."""
    cur = conn.execute("SELECT id, spread_type FROM playbook_strategies WHERE spread_type IS NOT NULL")
    return {row[1]: row[0] for row in cur.fetchall() if row[1]}


def iso(dt_str: str, time_part: str = 'T10:00:00Z') -> str:
    """Normalize a date/datetime string to RFC3339."""
    if 'T' in dt_str:
        return dt_str if dt_str.endswith('Z') else dt_str + 'Z'
    return dt_str[:10] + time_part


def insert_trades(conn: sqlite3.Connection, trades: List[Dict],
                  pb_map: Dict[str, int]) -> Tuple[int, Dict[int, int]]:
    """Insert trades into SQLite. Returns (success_count, index_to_db_id)."""
    success = 0
    failed = 0
    id_map: Dict[int, int] = {}

    for i, trade in enumerate(trades):
        pnl, debit_paid = compute_pnl_and_debit(trade)
        # Roll trades: partial-close means not all legs have closePremium → forcePnl overrides
        if trade.get('forcePnl') is not None:
            pnl = trade['forcePnl']
        spread_width    = compute_spread_width(trade)
        bpr             = compute_bpr(trade)
        entry_dte       = compute_dte(trade.get('tradeDate', ''), trade['expirationDate'])
        dte_at_close    = compute_dte_at_close(trade.get('exitTime'), trade['expirationDate'])

        playbook_id = pb_map.get(trade['spreadType'])

        legs_json = json.dumps(trade.get('legs', []))
        trade_date_str = trade.get('tradeDate', '')[:10]

        # Calculate POP if missing
        pop_val = trade.get('pop')
        if pop_val is None and trade.get('delta') is not None:
            # Rough estimate: POP = (1 - 2*abs(delta)) * 100
            pop_val = (1.0 - (2.0 * abs(trade['delta']))) * 100.0
            pop_val = max(min(pop_val, 95.0), 10.0)

        try:
            db_id = None
            if trade.get('is_update') and trade.get('id'):
                # UPDATE existing open trade (could be a simple close or a roll)
                conn.execute(
                    """UPDATE trades SET 
                        strategy = ?,
                        short_strike = ?, long_strike = ?,
                        short_premium = ?, long_premium = ?,
                        credit_received = ?,
                        trade_date = ?, entry_date = ?,
                        exit_date = ?, pnl = ?, debit_paid = ?, 
                        exit_reason = ?, dte_at_close = ?, legs_json = ?,
                        underlying_price_at_close = ?, pop = ?,
                        spread_width = ?, bpr = ?
                        WHERE id = ?""",
                    (
                        trade['spreadType'],
                        trade.get('shortStrike', 0) or 0,
                        trade.get('longStrike', 0) or 0,
                        trade.get('shortPremium', 0) or 0,
                        trade.get('longPremium', 0) or 0,
                        trade['creditReceived'],
                        trade_date_str + 'T10:00:00Z',
                        iso(trade['entryTime']),
                        iso(trade['exitTime']) if trade.get('exitTime') else None,
                        pnl, debit_paid,
                        trade.get('exitReason'),
                        dte_at_close,
                        legs_json,
                        trade.get('underlyingPriceAtClose'),
                        pop_val,
                        spread_width,
                        bpr,
                        trade['id']
                    )
                )
                conn.commit()
                db_id = trade['id']
                # Persist oc fingerprint only after successful UPDATE commit so a failed
                # update doesn't permanently block future retries.
                if trade.get('_oc_fp'):
                    conn.execute(
                        "INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)",
                        (trade['_oc_fp'], db_id)
                    )
                    conn.commit()
                # Part 3: roll fingerprint — prevents re-import of this roll on future runs
                if trade.get('is_roll_update') and trade.get('rollDate'):
                    roll_fp = f"roll|{db_id}|{trade['rollDate']}"
                    conn.execute(
                        "INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)",
                        (roll_fp, db_id)
                    )
                    conn.commit()
            else:
                # INSERT new trade
                cur = conn.execute(
                    """INSERT OR IGNORE INTO trades (
                        ticker, strategy, quantity,
                        short_strike, long_strike, short_premium, long_premium,
                        credit_received,
                        entry_date, exit_date, expiration_date, trade_date, back_month_expiration,
                        pnl, debit_paid,
                        delta, theta, gamma, vega, pop,
                        underlying_price, underlying_price_at_close,
                        iv_rank, vix_at_entry, implied_volatility,
                        commission, entry_reason, exit_reason, management_rule, target_profit_pct,
                        spread_width, bpr, entry_dte, dte_at_close,
                        playbook_id, rolled_from_id,
                        is_earnings_play, is_tested,
                        trade_grade, grade_notes,
                        legs_json, tags, notes
                    ) VALUES (
                        ?,?,?,
                        ?,?,?,?,
                        ?,
                        ?,?,?,?,?,
                        ?,?,
                        ?,?,?,?,?,
                        ?,?,
                        ?,?,?,
                        ?,?,?,?,?,
                        ?,?,?,?,
                        ?,?,
                        ?,?,
                        ?,?,
                        ?,?,?
                    )""",
                    (
                        trade['ticker'],                                            # ticker
                        trade['spreadType'],                                        # strategy
                        trade['quantity'],                                          # quantity
                        trade.get('shortStrike', 0) or 0,                          # short_strike
                        trade.get('longStrike', 0) or 0,                           # long_strike
                        trade.get('shortPremium', 0) or 0,                         # short_premium
                        trade.get('longPremium', 0) or 0,                          # long_premium
                        trade['creditReceived'],                                    # credit_received
                        iso(trade['entryTime']),                                    # entry_date
                        iso(trade['exitTime']) if trade.get('exitTime') else None,  # exit_date
                        iso(trade['expirationDate'], 'T00:00:00Z'),                 # expiration_date
                        trade_date_str + 'T10:00:00Z',                             # trade_date
                        iso(trade['backMonthExpiration'], 'T00:00:00Z') if trade.get('backMonthExpiration') else None,
                        pnl,                                                        # pnl
                        debit_paid,                                                 # debit_paid
                        trade.get('delta'),                                         # delta
                        trade.get('theta'),                                         # theta
                        trade.get('gamma'),                                         # gamma
                        trade.get('vega'),                                          # vega
                        pop_val,                                                    # pop
                        trade.get('underlyingPrice'),                               # underlying_price
                        trade.get('underlyingPriceAtClose'),                        # underlying_price_at_close
                        trade.get('ivRank'),                                        # iv_rank
                        trade.get('vixAtEntry'),                                    # vix_at_entry
                        trade.get('impliedVolatility'),                             # implied_volatility
                        trade.get('commission'),                                    # commission
                        trade.get('entryReason'),                                   # entry_reason
                        trade.get('exitReason'),                                    # exit_reason
                        None,                                                       # management_rule
                        None,                                                       # target_profit_pct
                        spread_width,                                               # spread_width
                        bpr,                                                        # bpr
                        entry_dte,                                                  # entry_dte
                        dte_at_close,                                               # dte_at_close
                        playbook_id,                                                # playbook_id
                        None,                                                       # rolled_from_id (set later)
                        0,                                                          # is_earnings_play
                        0,                                                          # is_tested
                        None,                                                       # trade_grade
                        None,                                                       # grade_notes
                        legs_json,                                                  # legs_json
                        '',                                                         # tags
                        None,                                                       # notes
                    )
                )
                conn.commit()
                db_id = cur.lastrowid
                # Record fingerprint so it survives future deletions or credit edits
                if db_id:
                    fp = f"{trade_date_str}|{trade['ticker']}|{trade['quantity']}|{trade['creditReceived']:.2f}"
                    conn.execute(
                        "INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)",
                        (fp, db_id)
                    )
                    conn.commit()
            id_map[i] = db_id

            status = 'CLOSED' if trade.get('exitTime') else 'OPEN'
            pnl_str = f"pnl=${pnl:+.2f}" if pnl is not None else "pnl=open"
            print(f"  {i+1:3d}. {trade['ticker']:6s} {trade['spreadType']:22s} "
                  f"{trade_date_str} cr=${trade['creditReceived']:+6.2f} "
                  f"{pnl_str} [{status}] -> ID {db_id}")
            success += 1

        except Exception as e:
            failed += 1
            print(f"  {i+1:3d}. FAILED: {trade['ticker']} {trade['spreadType']} — {e}")

    return success, id_map


def link_roll_chains(conn: sqlite3.Connection, trades: List[Dict],
                     id_map: Dict[int, int]) -> int:
    """Post-import: link rolled trades by setting rolled_from_id on the new trade.
    For each trade with exitReason='rolled', find the candidate new trade
    (same ticker, opened on close date) and UPDATE its rolled_from_id."""
    linked = 0

    for i, closed_trade in enumerate(trades):
        if closed_trade.get('exitReason') != 'rolled':
            continue
        if i not in id_map:
            continue
        # 0DTE chains are linked by position in main() — skip here to avoid wrong matches
        if closed_trade.get('_is_0dte_chain'):
            continue

        closed_db_id = id_map[i]
        close_date = closed_trade.get('exitTime', '')[:10]  # YYYY-MM-DD
        ticker = closed_trade['ticker']

        # Find the new trade (roll target)
        for j, candidate in enumerate(trades):
            if j == i or j not in id_map:
                continue
            if candidate['ticker'] != ticker:
                continue
            if candidate['tradeDate'][:10] == close_date:
                # This is the roll target — set its rolled_from_id to the closed trade's DB id
                new_db_id = id_map[j]
                conn.execute(
                    "UPDATE trades SET rolled_from_id = ? WHERE id = ?",
                    (closed_db_id, new_db_id)
                )
                conn.commit()
                linked += 1
                print(f"  Roll linked: ID {closed_db_id} -> ID {new_db_id} ({ticker})")
                break

    return linked


def is_protected_trade(trade: Dict) -> bool:
    """Protected = closed/expired trade from January or February 2026."""
    is_closed = (trade.get('exit_date') is not None or
                 trade.get('exit_reason') is not None or
                 trade.get('pnl') is not None)
    if not is_closed:
        return False
    trade_date_str = (trade.get('trade_date') or '')[:10]
    if not trade_date_str:
        return False
    try:
        td = datetime.strptime(trade_date_str, '%Y-%m-%d')
        return td.year == 2026 and td.month in (1, 2)
    except Exception:
        return False


def delete_unprotected_trades(conn: sqlite3.Connection) -> None:
    """Delete all trades from DB for a clean reimport."""
    cur = conn.execute("SELECT COUNT(*) FROM trades")
    count = cur.fetchone()[0]
    conn.execute("DELETE FROM trades")
    conn.commit()
    print(f"  Deleted {count} trades (full clean reimport)")


# ---------- DB Cleanup ----------

def cleanup_bad_rolls(conn: sqlite3.Connection) -> None:
    """Remove incorrectly imported roll trades so re-import can fix them.

    Deletes the 3 standalone short_call_vertical trades that were created from
    rolled IC legs (IDs 1407 AVGO, 1408 GLD, 1409 META) and removes their
    import_log fingerprints so the legs can be re-matched and absorbed into the
    correct iron condors (IDs 1372, 1398, 1404).
    """
    bad_ids = [1407, 1408, 1409]
    bad_fps = [
        '2026-03-17|AVGO|1|1.15',
        '2026-03-17|GLD|1|1.24',
        '2026-03-17|META|1|3.02',
    ]
    # ICs whose partial-close log entries must be cleared so they can re-match
    bad_ic_ids = [1372, 1398, 1404]

    placeholders = ','.join('?' * len(bad_ids))
    conn.execute(f"DELETE FROM trades WHERE id IN ({placeholders})", bad_ids)
    for fp in bad_fps:
        conn.execute("DELETE FROM import_log WHERE fingerprint = ?", (fp,))
    for ic_id in bad_ic_ids:
        conn.execute("DELETE FROM import_log WHERE fingerprint LIKE ?", (f'oc|{ic_id}|%',))
        conn.execute("DELETE FROM import_log WHERE fingerprint LIKE ?", (f'roll|{ic_id}|%',))
    conn.commit()
    print(f"cleanup_bad_rolls: deleted {len(bad_ids)} bad roll trades and their import_log entries.")
    print(f"  Re-cleared oc| fingerprints for ICs {bad_ic_ids} so they can re-match on next import.")


# ---------- Main ----------

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/import_schwab_rust.py /path/to/schwab.json [trades.db] [--delete-existing]")
        sys.exit(1)

    args = sys.argv[1:]
    json_path       = args[0]
    db_path         = next((a for a in args[1:] if not a.startswith('--')), 'trades.db')
    delete_existing = '--delete-existing' in args
    fix_rolls       = '--fix-rolls' in args

    # Open DB first (needed for --fix-rolls cleanup before parsing JSON)
    conn = sqlite3.connect(db_path)
    init_db(conn)
    migrate_db(conn)
    print(f"Connected to {db_path}")

    if fix_rolls:
        print("\nRunning --fix-rolls cleanup...")
        cleanup_bad_rolls(conn)
        print()

    # Schwab exports contain ASCII control characters (\x1a prefix plus scattered \x1f etc.).
    # Strip all control bytes (0x00-0x1F) except JSON-valid whitespace (tab, LF, CR).
    import re as _re
    with open(json_path, encoding='utf-8-sig') as f:
        raw = f.read()
    raw = _re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f]', '', raw)
    data = json.loads(raw)

    transactions = data['BrokerageTransactions']
    # Process in chronological order: Schwab JSON is newest-first (top=most recent,
    # bottom=oldest). Reversing ensures opens are seen before their subsequent closes,
    # which is required for correct 0DTE roll campaign detection (build_0dte_roll_chain).
    transactions = list(reversed(transactions))
    print(f"Loaded {len(transactions)} transactions from {json_path}")

    trades = build_trades(transactions, conn)
    trades.sort(key=lambda x: (x['tradeDate'][:10], x['ticker']))
    print(f"Parsed {len(trades)} trades")

    if delete_existing:
        # Delete non-protected trades first, then import everything fresh (no duplicate check needed)
        print("\nDeleting unprotected trades...")
        delete_unprotected_trades(conn)
        final_trades = trades
        print(f"  Fresh import: {len(final_trades)} trades (skipping duplicate check)")
    else:
        # Incremental: skip trades already in DB
        existing_fps = get_existing_fingerprints(conn)
        final_trades = filter_new_trades(trades, existing_fps)

    if not final_trades:
        print("No new trades to import.")
        conn.close()
        return

    print(f"\nTrades to import ({len(final_trades)}):")
    for t in final_trades:
        status = 'OPEN' if 'exitTime' not in t else 'CLOSED'
        print(f"  {t['ticker']:6s} {t['spreadType']:22s} {t['tradeDate'][:10]} cr={t['creditReceived']:+.2f} [{status}]")

    # Estimate Greeks using historical prices from Yahoo Finance
    # Only runs for the filtered list of new trades
    print("\nEstimating Greeks via Black-Scholes...")
    enriched = enrich_trades_with_greeks(final_trades)
    print(f"Greeks estimated for {enriched}/{len(final_trades)} trades")

    # Auto-assign playbook strategy IDs based on spreadType
    spread_to_id = get_playbook_map(conn)
    if spread_to_id:
        assigned = assigned = assign_playbook_ids(final_trades, spread_to_id)
        print(f"\nPlaybook: matched {assigned}/{len(final_trades)} trades to {len(spread_to_id)} strategies")
    else:
        print("\nPlaybook: no strategies found")

    print(f"\nImporting {len(final_trades)} trades...")
    success, id_map = insert_trades(conn, final_trades, spread_to_id)
    print(f"\nDone: {success} imported")

    # Post-import verification: confirm what landed in the DB
    if success > 0 and id_map:
        print(f"\nVerification — trades now in DB:")
        for idx, trade_idx in sorted(id_map.items()):
            t = final_trades[idx]
            status = 'OPEN' if 'exitTime' not in t else 'CLOSED'
            print(f"  ID {trade_idx:5d}: {t['ticker']:6s} {t['spreadType']:22s} {t['tradeDate'][:10]} cr={t['creditReceived']:+.2f} [{status}]")

    # Link roll chains (set rolled_from_id on new trades)
    rolled_count = sum(1 for t in final_trades if t.get('exitReason') == 'rolled'
                       and not t.get('_is_0dte_chain'))
    if rolled_count > 0:
        print(f"\nLinking {rolled_count} roll chains...")
        linked = link_roll_chains(conn, final_trades, id_map)
        print(f"Linked {linked} roll chains")

    # Link 0DTE roll chains by insertion order (rolled_from_id = previous IC in chain)
    chain_items = [
        (i, t) for i, t in enumerate(final_trades)
        if t.get('_chain_position') is not None and i in id_map
    ]
    if chain_items:
        from collections import defaultdict as _ChainDD
        chain_groups: Dict = _ChainDD(list)
        for i, t in chain_items:
            chain_groups[(t['_chain_ticker'], t['_chain_date'])].append(
                (t['_chain_position'], id_map[i])
            )
        print(f"\nLinking {len(chain_items)} 0DTE chain IC(s)...")
        for (cticker, cdate), items in chain_groups.items():
            items.sort()  # sort by _chain_position
            for k in range(1, len(items)):
                parent_id = items[k - 1][1]
                child_id  = items[k][1]
                conn.execute(
                    "UPDATE trades SET rolled_from_id = ? WHERE id = ?",
                    (parent_id, child_id)
                )
            conn.commit()
            print(f"  0DTE chain: {cticker} {cdate} — {len(items)} IC(s) linked")

    conn.close()

if __name__ == '__main__':
    main()
