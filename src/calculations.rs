/// Calculations — exact port of OptionsTradingJournal/client/src/lib/trade-calculations.ts
///
/// All formulas are matched 100% to the TypeScript original.
use crate::models::{LegType, StrategyType, TradeLeg, Trade};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Campaign metrics
pub struct CampaignMetrics {
    pub total_pnl: f64,
    pub total_credit: f64,
    pub total_debit: f64,
    pub roll_count: usize,
    pub net_credit: f64,
}

pub fn calculate_campaign_metrics(chain: &[Trade]) -> CampaignMetrics {
    let mut total_pnl = 0.0;
    let mut total_credit = 0.0;
    let mut total_debit = 0.0;
    
    for t in chain {
        if let Some(pnl) = t.pnl {
            total_pnl += pnl;
        }
        total_credit += t.credit_received;
        if let Some(dp) = t.debit_paid {
            total_debit += dp;
        }
    }
    
    CampaignMetrics {
        total_pnl,
        total_credit,
        total_debit,
        roll_count: chain.len().saturating_sub(1),
        net_credit: total_credit - total_debit, // net_credit = credits collected minus debits paid on rolls
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Basic P&L / Profit Metrics
// ──────────────────────────────────────────────────────────────────────────────

/// Compute spread width from legs — port of OTJ computeFromLegs() spread_width logic.
/// Returns the max of put-side and call-side widths (handles IC/IB and single verticals).
pub fn compute_spread_width_from_legs(legs: &[TradeLeg]) -> f64 {
    let short_put  = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
    let long_put   = legs.iter().find(|l| l.leg_type == LegType::LongPut);
    let short_call = legs.iter().find(|l| l.leg_type == LegType::ShortCall);
    let long_call  = legs.iter().find(|l| l.leg_type == LegType::LongCall);

    let put_width = match (short_put, long_put) {
        (Some(sp), Some(lp)) => (sp.strike - lp.strike).abs(),
        _ => 0.0,
    };
    let call_width = match (short_call, long_call) {
        (Some(sc), Some(lc)) => (sc.strike - lc.strike).abs(),
        _ => 0.0,
    };
    put_width.max(call_width)
}

/// Max profit = credit_received × 100 × quantity
/// Matches: calculateMaxProfit(creditReceived, quantity)
pub fn calculate_max_profit(credit_received: f64, quantity: i32) -> f64 {
    credit_received * 100.0 * quantity as f64
}

/// Max profit accounting for PBWB structure.
/// For "put_broken_wing_butterfly": (long_width + credit) × 100 × qty
/// For all other strategies: falls through to credit × 100 × qty.
pub fn calculate_max_profit_from_legs(
    legs: &[TradeLeg],
    credit: f64,
    quantity: i32,
    spread_type: &str,
) -> f64 {
    if spread_type == "put_broken_wing_butterfly" {
        let short_put = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
        let mut long_puts: Vec<&TradeLeg> = legs.iter()
            .filter(|l| l.leg_type == LegType::LongPut)
            .collect();
        long_puts.sort_by(|a, b| b.strike.partial_cmp(&a.strike).unwrap_or(std::cmp::Ordering::Equal));
        if let (Some(sp), Some(atm)) = (short_put, long_puts.first()) {
            let long_width = atm.strike - sp.strike;
            return (long_width + credit) * 100.0 * quantity as f64;
        }
    }
    calculate_max_profit(credit, quantity)
}

/// Max loss from legs — exact port of calculateMaxLossFromLegs()
pub fn calculate_max_loss_from_legs(
    legs: &[TradeLeg],
    credit: f64,
    quantity: i32,
    spread_type: &str,
) -> f64 {
    let qty = quantity as f64;

    // Calendar/diagonal: max loss = net debit paid = abs(credit) * 100 * qty
    if matches!(
        spread_type,
        "calendar_spread" | "pmcc" | "long_diagonal_spread" | "short_diagonal_spread"
    ) {
        return credit.abs() * 100.0 * qty;
    }

    // Cash Secured Put: (strike - credit) * 100 * qty (stock goes to zero)
    if spread_type == "cash_secured_put" {
        if let Some(sp) = legs.iter().find(|l| l.leg_type == LegType::ShortPut) {
            return (sp.strike - credit) * 100.0 * qty;
        }
    }

    // Covered Call: stock risk not tracked by options → 0
    if spread_type == "covered_call" {
        return 0.0;
    }

    let short_put  = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
    let long_put   = legs.iter().find(|l| l.leg_type == LegType::LongPut);
    let short_call = legs.iter().find(|l| l.leg_type == LegType::ShortCall);
    let long_call  = legs.iter().find(|l| l.leg_type == LegType::LongCall);

    // Put vertical: (width - credit) * 100 * qty
    if legs.len() == 2 && short_put.is_some() && long_put.is_some() {
        let width = (short_put.unwrap().strike - long_put.unwrap().strike).abs();
        return (width - credit) * 100.0 * qty;
    }

    // Call vertical
    if legs.len() == 2 && short_call.is_some() && long_call.is_some() {
        let width = (short_call.unwrap().strike - long_call.unwrap().strike).abs();
        return (width - credit) * 100.0 * qty;
    }

    // IC / IB: max of put-side or call-side width, minus credit
    if short_put.is_some() && long_put.is_some() && short_call.is_some() && long_call.is_some() {
        let put_width  = (short_put.unwrap().strike  - long_put.unwrap().strike).abs();
        let call_width = (short_call.unwrap().strike - long_call.unwrap().strike).abs();
        let max_width  = put_width.max(call_width);
        return (max_width - credit) * 100.0 * qty;
    }

    // Put Broken Wing Butterfly: (short_width - long_width - credit) * 100 * qty
    // long_puts sorted descending by strike: [0]=ATM anchor, [1]=wing
    if spread_type == "put_broken_wing_butterfly" {
        let short_put = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
        let mut long_puts: Vec<&TradeLeg> = legs.iter()
            .filter(|l| l.leg_type == LegType::LongPut)
            .collect();
        long_puts.sort_by(|a, b| b.strike.partial_cmp(&a.strike).unwrap_or(std::cmp::Ordering::Equal));
        if let (Some(sp), [atm, wing, ..]) = (short_put, long_puts.as_slice()) {
            let long_width  = atm.strike - sp.strike;   // ATM long − short
            let short_width = sp.strike - wing.strike;  // short − wing long
            let ml = (short_width - long_width - credit) * 100.0 * qty;
            if ml > 0.0 { return ml; }
        }
        return 0.0;
    }

    // Strangle / Straddle: undefined risk → 0 (signal undefined)
    0.0
}

/// BPR (Buying Power Reduction) — exact port of calculateBPR()
pub fn calculate_bpr(
    legs: &[TradeLeg],
    credit: f64,
    quantity: i32,
    underlying_price: Option<f64>,
    spread_type: &str,
) -> f64 {
    let qty = quantity as f64;

    // Calendar/diagonal: BPR = net debit
    if matches!(
        spread_type,
        "calendar_spread" | "pmcc" | "long_diagonal_spread" | "short_diagonal_spread"
    ) {
        return credit.abs() * 100.0 * qty;
    }

    // Cash Secured Put: 20% Naked Put Margin Rule
    if spread_type == "cash_secured_put" {
        if let Some(sp) = legs.iter().find(|l| l.leg_type == LegType::ShortPut) {
            if let Some(up) = underlying_price.filter(|&p| p > 0.0) {
                let put_otm = (up - sp.strike).max(0.0);
                let margin = (0.20 * up - put_otm + sp.premium)
                    .max(0.10 * up + sp.premium);
                return margin * 100.0 * qty;
            }
            // Fallback: 20% of strike
            return sp.strike * 0.20 * 100.0 * qty;
        }
    }

    // Covered Call: Stock Margin Rule (50%)
    if spread_type == "covered_call" {
        if let Some(up) = underlying_price.filter(|&p| p > 0.0) {
            return up * 0.50 * 100.0 * qty;
        }
        return 0.0;
    }

    let short_put  = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
    let long_put   = legs.iter().find(|l| l.leg_type == LegType::LongPut);
    let short_call = legs.iter().find(|l| l.leg_type == LegType::ShortCall);
    let long_call  = legs.iter().find(|l| l.leg_type == LegType::LongCall);

    // Put vertical
    if legs.len() == 2 && short_put.is_some() && long_put.is_some() {
        let width = (short_put.unwrap().strike - long_put.unwrap().strike).abs();
        return (width - credit) * 100.0 * qty;
    }

    // Call vertical
    if legs.len() == 2 && short_call.is_some() && long_call.is_some() {
        let width = (short_call.unwrap().strike - long_call.unwrap().strike).abs();
        return (width - credit) * 100.0 * qty;
    }

    // IC / IB: wider side width - credit
    if short_put.is_some() && long_put.is_some() && short_call.is_some() && long_call.is_some() {
        let put_width  = (short_put.unwrap().strike  - long_put.unwrap().strike).abs();
        let call_width = (short_call.unwrap().strike - long_call.unwrap().strike).abs();
        return (put_width.max(call_width) - credit) * 100.0 * qty;
    }

    // Strangle / Straddle: Reg-T margin approximation
    // Per-leg margin = max(20% * underlying - OTM + legPremium, 10% * underlying + legPremium)
    // Total = max(put_margin, call_margin) — thinkorswim uses greater leg only
    if short_put.is_some() && short_call.is_some() && long_put.is_none() && long_call.is_none() {
        if let Some(up) = underlying_price.filter(|&p| p > 0.0) {
            let sp = short_put.unwrap();
            let sc = short_call.unwrap();
            let put_otm  = (up - sp.strike).max(0.0);
            let call_otm = (sc.strike - up).max(0.0);
            let put_margin = (0.20 * up - put_otm + sp.premium)
                .max(0.10 * up + sp.premium);
            let call_margin = (0.20 * up - call_otm + sc.premium)
                .max(0.10 * up + sc.premium);
            let margin = put_margin.max(call_margin);
            return margin * 100.0 * qty;
        }
        // Fallback: 2x credit
        return credit * 2.0 * 100.0 * qty;
    }

    0.0
}

/// % of max profit captured — matches calculatePctMaxProfit()
pub fn calculate_pct_max_profit(pnl: f64, credit_received: f64, quantity: i32) -> f64 {
    let max_profit = credit_received * 100.0 * quantity as f64;
    if max_profit > 0.0 {
        (pnl / max_profit) * 100.0
    } else {
        0.0
    }
}

/// ROC (Return on Capital) — BPR-based denominator for undefined-risk strategies.
/// Returns None when capital cannot be determined (shows "—" in UI, not "0%").
///
/// Capital at risk:
///   - CC:              BPR (stored) — stock margin basis. None if no BPR.
///   - CSP:             BPR (stored) preferred; fallback to (strike−credit)×100×qty
///   - Strangle/Strad:  BPR (stored) preferred; fallback to 2×credit×100×qty
///   - Vertical/IC/IB:  (width − credit) × 100 × qty  [unchanged]
///   - Calendar/Diag:   abs(credit) × 100 × qty        [unchanged]
pub fn calculate_roc(
    pnl: f64,
    legs: &[TradeLeg],
    credit_received: f64,
    quantity: i32,
    spread_type: &str,
    bpr: Option<f64>,
    underlying_price: Option<f64>,
) -> Option<f64> {
    // Covered Call: stock margin (BPR) is the only valid capital basis.
    // If BPR not stored (old trades), return None — display "—" not "0%".
    if spread_type == "covered_call" {
        return bpr.map(|b| if b > 0.0 { (pnl / b) * 100.0 } else { 0.0 });
    }

    let mut capital_at_risk =
        calculate_max_loss_from_legs(legs, credit_received, quantity, spread_type);

    // Cash Secured Put: prefer BPR; fallback to standard Reg-T 20% margin estimate.
    if spread_type == "cash_secured_put" {
        if let Some(b) = bpr {
            if b > 0.0 { capital_at_risk = b; }
        } else if capital_at_risk <= 0.0 || capital_at_risk > underlying_price.unwrap_or(0.0) * 20.0 * quantity as f64 {
            // Fallback: use 20% of underlying (Reg-T standard), not (strike - credit) which overstates
            if let Some(u) = underlying_price {
                if u > 0.0 {
                    capital_at_risk = 0.20 * u * 100.0 * quantity as f64;
                }
            }
        }
    }

    // Strangle / Straddle: prefer BPR; fall back to 2× credit proxy.
    if capital_at_risk == 0.0 && matches!(spread_type, "strangle" | "straddle") {
        capital_at_risk = bpr
            .filter(|&b| b > 0.0)
            .unwrap_or_else(|| credit_received * 2.0 * 100.0 * quantity as f64);
    }

    if capital_at_risk > 0.0 { Some((pnl / capital_at_risk) * 100.0) } else { None }
}

/// Credit-to-Width ratio — matches calculateCreditWidthRatio()
/// Returns 0 for strategies where it doesn't apply.
pub fn calculate_credit_width_ratio(
    credit_received: f64,
    legs: &[TradeLeg],
    spread_type: &str,
) -> f64 {
    // Doesn't apply to single-leg, calendar, diagonal, undefined-risk
    if matches!(
        spread_type,
        "cash_secured_put"
            | "covered_call"
            | "calendar_spread"
            | "strangle"
            | "straddle"
            | "pmcc"
            | "long_diagonal_spread"
            | "short_diagonal_spread"
    ) {
        return 0.0;
    }

    let short_put  = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
    let long_put   = legs.iter().find(|l| l.leg_type == LegType::LongPut);
    let short_call = legs.iter().find(|l| l.leg_type == LegType::ShortCall);
    let long_call  = legs.iter().find(|l| l.leg_type == LegType::LongCall);

    let width = if short_put.is_some() && long_put.is_some() && short_call.is_some() && long_call.is_some() {
        // IC/IB: wider side
        let put_width  = (short_put.unwrap().strike  - long_put.unwrap().strike).abs();
        let call_width = (short_call.unwrap().strike - long_call.unwrap().strike).abs();
        put_width.max(call_width)
    } else if short_put.is_some() && long_put.is_some() {
        (short_put.unwrap().strike - long_put.unwrap().strike).abs()
    } else if short_call.is_some() && long_call.is_some() {
        (short_call.unwrap().strike - long_call.unwrap().strike).abs()
    } else {
        0.0
    };

    if width > 0.0 {
        (credit_received / width) * 100.0
    } else {
        0.0
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// P&L Calculation from Legs
// ──────────────────────────────────────────────────────────────────────────────

/// Calculate gross P&L for any strategy using per-leg data.
/// Matches calculateCustomPnl() — works for standard AND custom spreads.
/// Returns None if any leg is missing closePremium.
pub fn calculate_pnl_from_legs(
    legs: &[TradeLeg],
    quantity: i32,
    commission: Option<f64>,
    spread_type: &str,
) -> Option<f64> {
    if legs.is_empty() || legs.iter().any(|l| l.close_premium.is_none()) {
        return None;
    }

    let is_custom = spread_type == "custom";
    let mut gross_pnl = 0.0;

    for leg in legs {
        let qty = if is_custom {
            leg.quantity.unwrap_or(1)
        } else {
            quantity
        } as f64;

        let close_prem = leg.close_premium.unwrap_or(0.0);

        if leg.leg_type.is_short() {
            // Short leg: profit when we buy back cheaper
            gross_pnl += (leg.premium - close_prem) * 100.0 * qty;
        } else {
            // Long leg: profit when we sell higher
            gross_pnl += (close_prem - leg.premium) * 100.0 * qty;
        }
    }

    Some(gross_pnl - commission.unwrap_or(0.0))
}

// ──────────────────────────────────────────────────────────────────────────────
// Breakevens
// ──────────────────────────────────────────────────────────────────────────────

/// Calculate breakeven(s) for a trade — matches calculateBreakevens()
/// `credit_override`: when Some, uses this value instead of computing from leg premiums.
/// Pass `Some(trade.credit_received)` for PBWB; pass `None` for all other strategies.
pub fn calculate_breakevens(legs: &[TradeLeg], spread_type: &str, credit_override: Option<f64>) -> Vec<f64> {
    if legs.is_empty() {
        return vec![];
    }

    // Net credit from legs (or caller-supplied override)
    let computed: f64 = legs.iter().map(|l| {
        if l.leg_type.is_short() { l.premium } else { -l.premium }
    }).sum();
    let credit = credit_override.unwrap_or(computed);

    let short_put  = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
    let short_call = legs.iter().find(|l| l.leg_type == LegType::ShortCall);

    match spread_type {
        "short_put_vertical" | "cash_secured_put" => {
            if let Some(sp) = short_put {
                return vec![sp.strike - credit];
            }
        }
        "short_call_vertical" | "covered_call" => {
            if let Some(sc) = short_call {
                return vec![sc.strike + credit];
            }
        }
        "iron_condor" | "iron_butterfly" | "strangle" | "straddle" => {
            let be_put  = short_put.map(|sp| sp.strike - credit);
            let be_call = short_call.map(|sc| sc.strike + credit);
            return [be_put, be_call].iter().filter_map(|x| *x).collect();
        }
        // Calendar/diagonal: IV-dependent, cannot compute statically
        "calendar_spread" | "pmcc" | "long_diagonal_spread" | "short_diagonal_spread" => {
            return vec![];
        }
        "put_broken_wing_butterfly" => {
            if let Some(sp) = short_put {
                let mut long_puts: Vec<&TradeLeg> = legs.iter()
                    .filter(|l| l.leg_type == LegType::LongPut)
                    .collect();
                long_puts.sort_by(|a, b| b.strike.partial_cmp(&a.strike).unwrap_or(std::cmp::Ordering::Equal));
                if let Some(atm) = long_puts.first() {
                    let long_width = atm.strike - sp.strike;
                    return vec![sp.strike - (long_width + credit)];
                }
            }
        }
        _ => {}
    }

    vec![]
}

/// P&L at expiration for one underlying price, at qty = 1 contract (in dollars).
/// Callers multiply by trade.quantity for total P&L.
/// Note: calendar/diagonal spreads are IV-dependent and not supported here.
pub fn calculate_payoff_at_price(legs: &[TradeLeg], credit_received: f64, price: f64) -> f64 {
    let intrinsic: f64 = legs.iter().map(|leg| {
        let qty = leg.quantity.unwrap_or(1) as f64;
        let base = match leg.leg_type {
            LegType::ShortPut  => -(leg.strike - price).max(0.0),
            LegType::LongPut   =>  (leg.strike - price).max(0.0),
            LegType::ShortCall => -(price - leg.strike).max(0.0),
            LegType::LongCall  =>  (price - leg.strike).max(0.0),
        };
        qty * base
    }).sum();
    (credit_received + intrinsic) * 100.0
}

/// P&L at front-month expiration for a calendar spread, qty = 1 contract (in dollars).
/// Uses Black-Scholes to estimate the remaining back-month option value after the
/// short front-month leg has expired.
///
/// * `remaining_dte` – years remaining on the back-month after the front-month expires
///   (e.g., 30.0 / 365.25 for 30 days).
/// * `iv` – implied volatility as a decimal (0.25 = 25 %).
pub fn calculate_calendar_payoff_at_price(
    legs: &[TradeLeg],
    credit_received: f64,
    price: f64,
    remaining_dte: f64,
    iv: f64,
) -> f64 {
    let r = 0.0_f64; // risk-free rate approximation

    // Detect call vs put calendar from which long leg is present
    let is_call_calendar = legs.iter().any(|l| l.leg_type == LegType::LongCall);

    // Both legs share the same strike in a plain calendar spread
    let strike = legs.iter().map(|l| l.strike).find(|&s| s > 0.0).unwrap_or(price);

    let t  = remaining_dte.max(1.0 / 365.25);
    let d1 = bs_d1(price, strike, t, r, iv.max(0.001));
    let d2 = d1 - iv * t.sqrt();

    // Black-Scholes value of the back-month long leg still alive after front expires
    let long_leg_bs = if is_call_calendar {
        price * normal_cdf(d1) - strike * (-r * t).exp() * normal_cdf(d2)
    } else {
        strike * (-r * t).exp() * normal_cdf(-d2) - price * normal_cdf(-d1)
    };

    // Intrinsic of the expired short leg (cost to close / assignment)
    let short_intrinsic = if is_call_calendar {
        (price - strike).max(0.0)
    } else {
        (strike - price).max(0.0)
    };

    (credit_received + long_leg_bs - short_intrinsic) * 100.0
}

// ──────────────────────────────────────────────────────────────────────────────
// Time / DTE helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Remaining days to expiration (clamped to 0)
/// Matches calculateRemainingDTE()
pub fn calculate_remaining_dte(expiration_date: &DateTime<Utc>) -> i64 {
    let now = Utc::now().date_naive();
    let exp = expiration_date.date_naive();
    (exp - now).num_days().max(0)
}

/// Held duration in (days, hours, minutes)
/// Matches calculateHeldDuration()
pub fn calculate_held_duration(
    entry: &DateTime<Utc>,
    exit: &DateTime<Utc>,
) -> (i64, i64, i64) {
    let diff_ms = (exit.timestamp_millis() - entry.timestamp_millis()).max(0);
    let total_minutes = diff_ms / 60_000;
    let days    = total_minutes / (60 * 24);
    let hours   = (total_minutes % (60 * 24)) / 60;
    let minutes = total_minutes % 60;
    (days, hours, minutes)
}

/// P&L per day held — matches calculatePnlPerDay()
pub fn calculate_pnl_per_day(
    pnl: Option<f64>,
    entry: &DateTime<Utc>,
    exit: Option<&DateTime<Utc>>,
) -> Option<f64> {
    let effective_pnl = pnl?;
    let end = exit.map(|e| *e).unwrap_or_else(Utc::now);
    let days = ((end.timestamp_millis() - entry.timestamp_millis()) as f64
        / (1000.0 * 60.0 * 60.0 * 24.0))
        .round()
        .max(1.0);
    Some(effective_pnl / days)
}

/// Format held duration as human-readable string
pub fn format_held_duration(entry: &DateTime<Utc>, exit: &DateTime<Utc>) -> String {
    let (days, hours, minutes) = calculate_held_duration(entry, exit);
    if days > 0 {
        format!("{}d", days)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Trade Description / Badges
// ──────────────────────────────────────────────────────────────────────────────

/// Format trade description from legs — matches formatTradeDescription()
pub fn format_trade_description(legs: &[TradeLeg], spread_type: &str) -> String {
    if legs.is_empty() {
        return "N/A".to_string();
    }

    let short_put  = legs.iter().find(|l| l.leg_type == LegType::ShortPut);
    let long_put   = legs.iter().find(|l| l.leg_type == LegType::LongPut);
    let short_call = legs.iter().find(|l| l.leg_type == LegType::ShortCall);
    let long_call  = legs.iter().find(|l| l.leg_type == LegType::LongCall);

    match spread_type {
        "short_put_vertical" => {
            if let (Some(sp), Some(lp)) = (short_put, long_put) {
                return format!("{:.0}/{:.0}P", sp.strike, lp.strike);
            }
        }
        "short_call_vertical" => {
            if let (Some(sc), Some(lc)) = (short_call, long_call) {
                return format!("{:.0}/{:.0}C", sc.strike, lc.strike);
            }
        }
        "iron_condor" | "iron_butterfly" => {
            if let (Some(lp), Some(sp), Some(sc), Some(lc)) = (long_put, short_put, short_call, long_call) {
                return format!(
                    "{:.0}/{:.0}P | {:.0}/{:.0}C",
                    lp.strike, sp.strike, sc.strike, lc.strike
                );
            }
        }
        "strangle" => {
            if let (Some(sp), Some(sc)) = (short_put, short_call) {
                return format!("{:.0}P | {:.0}C", sp.strike, sc.strike);
            }
        }
        "straddle" => {
            if let Some(sp) = short_put {
                return format!("{:.0} Straddle", sp.strike);
            }
        }
        "cash_secured_put" => {
            if let Some(sp) = short_put {
                return format!("{:.0}P CSP", sp.strike);
            }
        }
        "covered_call" => {
            if let Some(sc) = short_call {
                return format!("{:.0}C CC", sc.strike);
            }
        }
        "calendar_spread" => {
            let strike = short_call
                .map(|l| l.strike)
                .or_else(|| short_put.map(|l| l.strike))
                .unwrap_or(0.0);
            let leg_char = if short_call.is_some() { "C" } else { "P" };
            return format!("{:.0}{} Cal", strike, leg_char);
        }
        "pmcc" => {
            if let (Some(lc), Some(sc)) = (long_call, short_call) {
                return format!("{:.0}/{:.0}C PMCC", lc.strike, sc.strike);
            }
        }
        "long_diagonal_spread" | "short_diagonal_spread" => {
            let badge = if spread_type == "long_diagonal_spread" { "LDS" } else { "SDS" };
            let short_leg = short_call.or(short_put);
            let long_leg  = long_call.or(long_put);
            let leg_char  = if short_call.is_some() || long_call.is_some() { "C" } else { "P" };
            if let (Some(sl), Some(ll)) = (short_leg, long_leg) {
                return format!("{:.0}/{:.0}{} {}", sl.strike, ll.strike, leg_char, badge);
            }
        }
        "custom" | "put_broken_wing_butterfly" => {
            return legs.iter().map(|l| {
                let sign    = if l.leg_type.is_short() { "-" } else { "+" };
                let qty     = l.quantity.unwrap_or(1);
                let opt_char = if l.leg_type.is_call() { "C" } else { "P" };
                format!("{}{} {:.0}{}", sign, qty, l.strike, opt_char)
            }).collect::<Vec<_>>().join(" / ");
        }
        _ => {}
    }

    "N/A".to_string()
}

// ──────────────────────────────────────────────────────────────────────────────
// Streak & Drawdown Analytics
// ──────────────────────────────────────────────────────────────────────────────

/// Streak analysis — matches getStreakAnalysis()
/// Returns (current_streak, max_win_streak, max_loss_streak)
/// current_streak: positive = win streak length, negative = loss streak length
pub fn get_streak_analysis(trades: &[&Trade]) -> (i64, usize, usize) {
    let completed: Vec<&&Trade> = trades.iter()
        .filter(|t| t.pnl.is_some())
        .collect();

    if completed.is_empty() {
        return (0, 0, 0);
    }

    let mut max_win_streak  = 0usize;
    let mut max_loss_streak = 0usize;
    let mut current_streak_type: Option<bool> = None; // Some(true)=win, Some(false)=loss
    let mut current_streak_len  = 0usize;

    for trade in &completed {
        let is_win = trade.pnl.unwrap_or(0.0) > 0.0;

        if current_streak_type == Some(is_win) {
            current_streak_len += 1;
        } else {
            if let Some(st) = current_streak_type {
                if st {
                    max_win_streak  = max_win_streak.max(current_streak_len);
                } else {
                    max_loss_streak = max_loss_streak.max(current_streak_len);
                }
            }
            current_streak_type = Some(is_win);
            current_streak_len  = 1;
        }
    }

    // Handle the final streak
    let current_streak = if let Some(st) = current_streak_type {
        if st {
            max_win_streak  = max_win_streak.max(current_streak_len);
            current_streak_len as i64
        } else {
            max_loss_streak = max_loss_streak.max(current_streak_len);
            -(current_streak_len as i64)
        }
    } else {
        0
    };

    (current_streak, max_win_streak, max_loss_streak)
}

/// Max drawdown — matches calculateDrawdown()
/// Returns (max_drawdown, max_drawdown_pct, current_drawdown)
pub fn calculate_drawdown(balance_history: &[f64]) -> (f64, f64, f64) {
    if balance_history.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut peak           = balance_history[0];
    let mut max_drawdown   = 0.0_f64;
    let mut max_drawdown_pct = 0.0_f64;

    for &bal in balance_history.iter().skip(1) {
        if bal > peak {
            peak = bal;
        }
        let drawdown     = peak - bal;
        let drawdown_pct = if peak > 0.0 { (drawdown / peak) * 100.0 } else { 0.0 };
        if drawdown > max_drawdown {
            max_drawdown     = drawdown;
            max_drawdown_pct = drawdown_pct;
        }
    }

    let current_peak = balance_history.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let current_bal  = *balance_history.last().unwrap_or(&0.0);
    let current_drawdown = current_peak - current_bal;

    (max_drawdown, max_drawdown_pct, current_drawdown)
}

// ──────────────────────────────────────────────────────────────────────────────
// Portfolio Stats Builder
// ──────────────────────────────────────────────────────────────────────────────

/// Build PortfolioStats from a slice of trades.
/// Called once at startup (and on refresh) in main.rs.
/// `account_size` is the configured account size (default 100_000).
/// `vix` is the current VIX fetched from Yahoo Finance (None if unavailable).
/// `beta_map` maps ticker → beta vs SPY (default 1.0 if missing).
/// `spy_price` is the current SPY price (None → BWD skipped).
pub fn build_portfolio_stats(
    trades: &[Trade],
    account_size: f64,
    vix: Option<f64>,
    beta_map: &HashMap<String, f64>,
    spy_price: Option<f64>,
    target_undefined_pct: f64,
    monthly_pnl_target: f64,
) -> crate::models::PortfolioStats {
    use crate::models::PortfolioStats;

    let today            = chrono::Utc::now().date_naive();
    let mut total_pnl    = 0.0_f64;
    let mut realized_pnl = 0.0_f64;
    let mut unrealized_pnl = 0.0_f64;
    let mut wins         = 0usize;
    let mut open_count   = 0usize;
    let mut closed_count = 0usize;
    let mut roc_sum      = 0.0_f64;
    let mut roc_count    = 0usize;
    let mut best_pnl     = f64::NEG_INFINITY;
    let mut worst_pnl    = f64::INFINITY;
    let mut net_theta    = 0.0_f64;
    let mut net_vega     = 0.0_f64;
    let mut net_bwd      = 0.0_f64;

    // OTJ Dashboard metrics for open trades
    let mut total_open_bpr  = 0.0_f64;
    let mut undefined_bpr   = 0.0_f64;
    let mut defined_bpr     = 0.0_f64;
    let mut pop_sum         = 0.0_f64;
    let mut pop_count       = 0usize;

    // Collect (exit_date, pnl) pairs — sorted later for accurate drawdown
    let mut closed_pnl_series: Vec<(chrono::NaiveDate, f64)> = Vec::new();

    // M10: IVR accumulator for open trades
    let mut ivr_open_sum   = 0.0_f64;
    let mut ivr_open_count = 0usize;

    // M5: next critical positions (DTE ≤ 21)
    let mut critical_positions: Vec<(String, i32)> = Vec::new();

    // M1: P50 accumulator for open trades
    let mut p50_open_sum   = 0.0_f64;
    let mut p50_open_count = 0usize;

    // L1: first trade date for pace computation
    let mut first_trade_date: Option<chrono::NaiveDate> = None;

    // L3: strategy counts for open trades
    let mut open_strategy_map: HashMap<String, usize> = HashMap::new();

    // Sort by exit_date so streak and drawdown are chronological
    let mut all_refs: Vec<&Trade> = trades.iter().collect();
    all_refs.sort_by_key(|t| t.exit_date);

    for trade in trades {
        // L1: track first trade date
        let td = trade.trade_date.date_naive();
        first_trade_date = Some(match first_trade_date {
            None => td,
            Some(prev) => prev.min(td),
        });

        if let Some(pnl) = trade.pnl {
            realized_pnl += pnl;
            total_pnl    += pnl;
            closed_count += 1;

            if pnl > 0.0 { wins += 1; }
            if pnl > best_pnl  { best_pnl  = pnl; }
            if pnl < worst_pnl { worst_pnl = pnl; }

            if let Some(roc) = calculate_roc(pnl, &trade.legs, trade.credit_received, trade.quantity, trade.spread_type(), trade.bpr, trade.underlying_price) {
                if roc.abs() > 0.001 {
                    roc_sum   += roc;
                    roc_count += 1;
                }
            }

            if let Some(exit) = trade.exit_date {
                closed_pnl_series.push((exit.date_naive(), pnl));
            }
        } else {
            open_count += 1;

            // Net theta and vega from open positions
            if let Some(th) = trade.theta {
                net_theta += th * 100.0 * trade.quantity as f64;
            }
            if let Some(vg) = trade.vega {
                net_vega += vg * 100.0 * trade.quantity as f64;
            }

            // Beta-Weighted Delta (only calculable when SPY price and delta are available)
            if let Some(delta) = trade.delta {
                if let Some(spy) = spy_price {
                    if spy > 0.0 {
                        let underlying = trade.underlying_price.unwrap_or(spy);
                        let beta = beta_map.get(trade.ticker.as_str()).copied().unwrap_or(1.0);
                        net_bwd += delta * beta * (underlying / spy) * trade.quantity as f64 * 100.0;
                    }
                }
            }

            // BPR: use stored value or recompute
            let bpr = trade.bpr.unwrap_or_else(|| {
                calculate_bpr(
                    &trade.legs,
                    trade.credit_received,
                    trade.quantity,
                    trade.underlying_price,
                    trade.spread_type(),
                )
            });
            total_open_bpr += bpr;

            // Undefined risk: CSP, CC, Strangle, Straddle
            let is_undefined = matches!(
                trade.strategy,
                StrategyType::CashSecuredPut
                    | StrategyType::CoveredCall
                    | StrategyType::Strangle
                    | StrategyType::Straddle
            );
            if is_undefined { undefined_bpr += bpr; } else { defined_bpr += bpr; }

            // Use stored POP if available; fall back to BS estimate for
            // trades entered before POP was wired (or imported trades).
            let effective_pop = trade.pop.unwrap_or_else(|| estimate_pop(trade));
            pop_sum   += effective_pop;
            pop_count += 1;

            // M10: avg IVR
            if let Some(ivr) = trade.iv_rank {
                ivr_open_sum   += ivr;
                ivr_open_count += 1;
            }

            // M5: next critical positions (DTE ≤ 21)
            let remaining_dte = calculate_remaining_dte(&trade.expiration_date);
            if remaining_dte <= 21 {
                critical_positions.push((trade.ticker.clone(), remaining_dte as i32));
            }

            // M1: P50
            if let Some(p50) = calculate_p50(trade) {
                p50_open_sum   += p50;
                p50_open_count += 1;
            }

            // L3: strategy distribution
            *open_strategy_map.entry(trade.strategy.badge().to_string()).or_insert(0) += 1;

            // Unrealized P&L estimate: theta ($/share/day) × days_held × 100 × qty
            // theta is stored as positive for credit sellers (we benefit from decay)
            if let Some(theta) = trade.theta {
                let days_held = (today - trade.trade_date.date_naive()).num_days().max(0) as f64;
                unrealized_pnl += theta * 100.0 * trade.quantity as f64 * days_held;
            }
        }
    }

    let win_rate = if closed_count > 0 {
        wins as f64 / closed_count as f64
    } else {
        0.0
    };

    let avg_roc = if roc_count > 0 {
        roc_sum / roc_count as f64
    } else {
        0.0
    };

    let avg_pnl_per_trade = if closed_count > 0 {
        realized_pnl / closed_count as f64
    } else {
        0.0
    };

    // Build sorted balance history anchored at account_size for accurate drawdown %
    closed_pnl_series.sort_by_key(|(d, _)| *d);
    let mut balance_history: Vec<f64> = Vec::with_capacity(closed_pnl_series.len() + 1);
    balance_history.push(account_size);
    let mut bh_running = account_size;
    for (_, pnl) in &closed_pnl_series {
        bh_running += pnl;
        balance_history.push(bh_running);
    }
    let (max_drawdown, max_drawdown_pct, _) = calculate_drawdown(&balance_history);

    let (current_streak, max_win_streak, max_loss_streak) =
        get_streak_analysis(&all_refs);

    // OTJ Dashboard derived values
    let alloc_pct = if account_size > 0.0 {
        total_open_bpr / account_size * 100.0
    } else {
        0.0
    };
    let undefined_pct = if total_open_bpr > 0.0 {
        undefined_bpr / total_open_bpr * 100.0
    } else {
        0.0
    };
    let defined_pct = if total_open_bpr > 0.0 {
        defined_bpr / total_open_bpr * 100.0
    } else {
        0.0
    };
    let drift   = undefined_pct - target_undefined_pct;
    let avg_pop = if pop_count > 0 { pop_sum / pop_count as f64 } else { 0.0 };
    let balance = account_size + realized_pnl;

    // M5: sort critical positions by DTE ascending
    critical_positions.sort_by_key(|(_, dte)| *dte);

    // L1: monthly P&L pace
    let monthly_pnl_pace = if let Some(first) = first_trade_date {
        let days_elapsed = (today - first).num_days().max(1) as f64;
        let months_elapsed = days_elapsed / 30.44;
        if months_elapsed > 0.0 { realized_pnl / months_elapsed } else { 0.0 }
    } else {
        0.0
    };

    // L2: theta/delta efficiency ratio
    let theta_delta_ratio = if net_bwd.abs() >= 0.01 {
        Some(net_theta / net_bwd.abs())
    } else {
        None
    };

    // tastytrade KPI: Θ/NetLiq = net_theta / account_size × 100 (target 0.1–0.3%)
    let theta_netliq_ratio = if account_size > 0.0 && open_count > 0 {
        Some((net_theta / account_size) * 100.0)
    } else {
        None
    };

    // Θ/BPR efficiency: net_theta / total_open_bpr × 100 (% return on capital per day)
    let theta_bpr_ratio = if total_open_bpr > 0.0 && open_count > 0 {
        Some((net_theta / total_open_bpr) * 100.0)
    } else {
        None
    };

    // L3: open strategy counts sorted by count desc
    let mut open_strategy_counts: Vec<(String, usize)> = open_strategy_map.into_iter().collect();
    open_strategy_counts.sort_by(|a, b| b.1.cmp(&a.1));

    // Item 13: Portfolio stress test (beta-adjusted, 7 scenarios)
    use crate::models::StressPoint;
    let open_trades_for_stress: Vec<&Trade> = trades.iter().filter(|t| t.is_open()).collect();
    let stress_open_count = open_trades_for_stress.len();
    let stress_priced_count = open_trades_for_stress.iter()
        .filter(|t| t.underlying_price.map(|p| p > 0.0).unwrap_or(false) && !t.legs.is_empty())
        .count();
    let stress_test: Vec<StressPoint> = [-20.0f64, -15.0, -10.0, -5.0, 0.0, 5.0, 10.0].iter().map(|&move_pct| {
        let mut total = 0.0f64;
        let mut worst_pnl = f64::MAX;
        let mut worst_ticker = String::new();
        for trade in &open_trades_for_stress {
            if let Some(underlying) = trade.underlying_price {
                if underlying > 0.0 && !trade.legs.is_empty() {
                    let beta = beta_map.get(trade.ticker.as_str()).copied().unwrap_or(1.0);
                    let adjusted = underlying * (1.0 + (move_pct / 100.0) * beta);
                    let pnl = calculate_payoff_at_price(&trade.legs, trade.credit_received, adjusted)
                              * trade.quantity as f64;
                    total += pnl;
                    if pnl < worst_pnl {
                        worst_pnl = pnl;
                        worst_ticker = trade.ticker.clone();
                    }
                }
            }
        }
        StressPoint {
            spy_move_pct: move_pct,
            total_pnl: total,
            pct_of_account: if account_size > 0.0 { total / account_size * 100.0 } else { 0.0 },
            worst_ticker: if worst_ticker.is_empty() { "—".to_string() } else { worst_ticker },
            worst_pnl: if worst_pnl == f64::MAX { 0.0 } else { worst_pnl },
        }
    }).collect();

    PortfolioStats {
        total_pnl,
        realized_pnl,
        win_rate,
        open_trades:   open_count,
        closed_trades: closed_count,
        total_trades:  trades.len(),
        avg_roc,
        avg_pnl_per_trade,
        best_trade_pnl:  if best_pnl.is_finite()  { best_pnl  } else { 0.0 },
        worst_trade_pnl: if worst_pnl.is_finite()  { worst_pnl } else { 0.0 },
        max_drawdown,
        max_drawdown_pct,
        current_streak,
        max_win_streak,
        max_loss_streak,
        net_beta_weighted_delta: net_bwd,
        net_theta,
        spy_price,
        // OTJ Dashboard
        account_size,
        balance,
        alloc_pct,
        total_open_bpr,
        undefined_risk_bpr: undefined_bpr,
        defined_risk_bpr:   defined_bpr,
        undefined_risk_pct: undefined_pct,
        defined_risk_pct:   defined_pct,
        target_undefined_pct,
        drift,
        avg_pop,
        vix,
        net_vega,
        bp_available: account_size - total_open_bpr,
        unrealized_pnl,
        avg_ivr_open:             if ivr_open_count > 0 { Some(ivr_open_sum / ivr_open_count as f64) } else { None },
        next_critical_positions:  critical_positions,
        avg_p50_open:             if p50_open_count > 0 { Some(p50_open_sum / p50_open_count as f64) } else { None },
        theta_delta_ratio,
        theta_netliq_ratio,
        theta_bpr_ratio,
        monthly_pnl_target,
        monthly_pnl_pace,
        open_strategy_counts,
        stress_test,
        stress_priced_count,
        stress_open_count,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Black-Scholes Greeks & POP Estimation
// ──────────────────────────────────────────────────────────────────────────────

fn normal_cdf(x: f64) -> f64 {
    let a1 =  0.254829592;
    let a2 = -0.284496736;
    let a3 =  1.421413741;
    let a4 = -1.453152027;
    let a5 =  1.061405429;
    let p  =  0.3275911;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs() / 2.0f64.sqrt();
    let t = 1.0 / (1.0 + p * ax);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-ax * ax).exp();
    0.5 * (1.0 + sign * y)
}

fn normal_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

fn bs_d1(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        return if s >= k { 10.0 } else { -10.0 };
    }
    ( (s / k).ln() + (r + 0.5 * sigma.powi(2)) * t ) / (sigma * t.sqrt())
}

pub fn estimate_greeks(
    s: f64, k: f64, t_days: i32, r: f64, sigma: f64, is_call: bool, is_short: bool
) -> (f64, f64, f64, f64) {
    let t = (t_days as f64 / 365.25).max(1.0 / 365.0);
    let d1 = bs_d1(s, k, t, r, sigma);
    let d2 = d1 - sigma * t.sqrt();
    let nd1 = normal_pdf(d1);

    let mut delta = if is_call { normal_cdf(d1) } else { normal_cdf(d1) - 1.0 };
    let gamma = nd1 / (s * sigma * t.sqrt());
    
    let mut theta_ann = -(s * nd1 * sigma) / (2.0 * t.sqrt());
    if is_call {
        theta_ann -= r * k * (-r * t).exp() * normal_cdf(d2);
    } else {
        theta_ann += r * k * (-r * t).exp() * normal_cdf(-d2);
    }
    let mut theta = theta_ann / 365.0;
    let mut vega = s * nd1 * t.sqrt() / 100.0;

    // Apply signs for short positions
    if is_short {
        delta = -delta;
        theta = -theta;
        vega = -vega;
        // gamma is technically always positive for long, negative for short
    }

    (delta, theta, gamma, vega)
}

/// POP (Probability of Profit) estimation using Black-Scholes d2.
/// For short puts/verticals: POP = N(d2) = P(S_T > K).
/// For short calls/verticals: POP = N(-d2) = P(S_T < K).
/// For ICs/strangles: P(K_put < S_T < K_call) ≈ N(d2_put) + N(-d2_call) - 1.
/// Falls back to delta-based estimate when IV or price unavailable.
pub fn estimate_pop(trade: &Trade) -> f64 {
    // IV stored as whole-number % (e.g. 25.0 → 0.25 for BS)
    let iv_raw = trade.implied_volatility.unwrap_or(0.0);
    let iv = if iv_raw > 2.0 { iv_raw / 100.0 } else if iv_raw > 0.0 { iv_raw } else { 0.0 };

    let s = match trade.underlying_price {
        Some(p) if p > 0.0 => p,
        _ => return delta_based_pop_fallback(trade),
    };

    if iv <= 0.0 {
        return delta_based_pop_fallback(trade);
    }

    let dte = calculate_remaining_dte(&trade.expiration_date);
    let t = (dte as f64 / 365.25).max(1.0 / 365.0);
    let r = 0.045_f64;

    let pop = match trade.strategy {
        StrategyType::CashSecuredPut | StrategyType::ShortPutVertical => {
            let k = trade.legs.iter()
                .find(|l| l.leg_type == LegType::ShortPut)
                .map(|l| l.strike)
                .unwrap_or(trade.short_strike);
            if k <= 0.0 { return 60.0; }
            let d2 = bs_d1(s, k, t, r, iv) - iv * t.sqrt();
            normal_cdf(d2) * 100.0
        }
        StrategyType::CoveredCall | StrategyType::ShortCallVertical => {
            let k = trade.legs.iter()
                .find(|l| l.leg_type == LegType::ShortCall)
                .map(|l| l.strike)
                .unwrap_or(trade.short_strike);
            if k <= 0.0 { return 60.0; }
            let d2 = bs_d1(s, k, t, r, iv) - iv * t.sqrt();
            normal_cdf(-d2) * 100.0
        }
        StrategyType::IronCondor | StrategyType::IronButterfly
        | StrategyType::Strangle | StrategyType::Straddle => {
            let kp = trade.legs.iter().find(|l| l.leg_type == LegType::ShortPut).map(|l| l.strike);
            let kc = trade.legs.iter().find(|l| l.leg_type == LegType::ShortCall).map(|l| l.strike);
            match (kp, kc) {
                (Some(kp), Some(kc)) if kp > 0.0 && kc > 0.0 => {
                    let d2_put  = bs_d1(s, kp, t, r, iv) - iv * t.sqrt();
                    let d2_call = bs_d1(s, kc, t, r, iv) - iv * t.sqrt();
                    // P(above short put) + P(below short call) - 1
                    (normal_cdf(d2_put) + normal_cdf(-d2_call) - 1.0) * 100.0
                }
                (Some(kp), None) if kp > 0.0 => {
                    let d2 = bs_d1(s, kp, t, r, iv) - iv * t.sqrt();
                    normal_cdf(d2) * 100.0
                }
                _ => 68.0,
            }
        }
        _ => delta_based_pop_fallback(trade),
    };
    pop.clamp(10.0, 95.0)
}

/// P50 = probability of reaching 50% of max profit before expiry.
/// Evaluated at the 50%-profit threshold strike using Black-Scholes d2.
pub fn calculate_p50(trade: &Trade) -> Option<f64> {
    let iv_raw = trade.implied_volatility.unwrap_or(0.0);
    let iv = if iv_raw > 2.0 { iv_raw / 100.0 } else if iv_raw > 0.0 { iv_raw } else { return None; };

    let s = trade.underlying_price?;
    if s <= 0.0 { return None; }

    let dte = calculate_remaining_dte(&trade.expiration_date);
    let t = (dte as f64 / 365.25).max(1.0 / 365.0);
    let r = 0.045_f64;
    let credit = trade.credit_received;

    let p50 = match trade.strategy {
        StrategyType::CashSecuredPut | StrategyType::ShortPutVertical => {
            // 50% profit when stock stays above: short_strike + credit*0.5
            let k = trade.legs.iter()
                .find(|l| l.leg_type == LegType::ShortPut)
                .map(|l| l.strike)
                .unwrap_or(trade.short_strike);
            if k <= 0.0 { return None; }
            let threshold = k + credit * 0.5;
            let d2 = bs_d1(s, threshold, t, r, iv) - iv * t.sqrt();
            normal_cdf(d2) * 100.0
        }
        StrategyType::CoveredCall | StrategyType::ShortCallVertical => {
            // 50% profit when stock stays below: short_strike - credit*0.5
            let k = trade.legs.iter()
                .find(|l| l.leg_type == LegType::ShortCall)
                .map(|l| l.strike)
                .unwrap_or(trade.short_strike);
            if k <= 0.0 { return None; }
            let threshold = k - credit * 0.5;
            let d2 = bs_d1(s, threshold, t, r, iv) - iv * t.sqrt();
            normal_cdf(-d2) * 100.0
        }
        StrategyType::IronCondor | StrategyType::IronButterfly
        | StrategyType::Strangle | StrategyType::Straddle => {
            let kp = trade.legs.iter().find(|l| l.leg_type == LegType::ShortPut).map(|l| l.strike);
            let kc = trade.legs.iter().find(|l| l.leg_type == LegType::ShortCall).map(|l| l.strike);
            match (kp, kc) {
                (Some(kp), Some(kc)) if kp > 0.0 && kc > 0.0 => {
                    let tp = kp + credit * 0.5;
                    let tc = kc - credit * 0.5;
                    let d2_put  = bs_d1(s, tp, t, r, iv) - iv * t.sqrt();
                    let d2_call = bs_d1(s, tc, t, r, iv) - iv * t.sqrt();
                    (normal_cdf(d2_put) + normal_cdf(-d2_call) - 1.0) * 100.0
                }
                _ => return None,
            }
        }
        _ => return None,
    };
    Some(p50.clamp(5.0, 95.0))
}

fn delta_based_pop_fallback(trade: &Trade) -> f64 {
    match trade.strategy {
        StrategyType::IronCondor | StrategyType::IronButterfly => 68.0,
        StrategyType::Strangle | StrategyType::Straddle => 66.0,
        _ => {
            let raw_abs = trade.delta.map(|d| d.abs()).unwrap_or(0.30);
            // Normalize: delta stored as whole degrees (e.g. 30.0) → convert to decimal
            let delta_abs = if raw_abs > 1.0 { raw_abs / 100.0 } else { raw_abs };
            (1.0 - delta_abs).clamp(0.25, 0.95) * 100.0
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Compliance Violation check... (remaining code)

/// A single compliance violation (field + rule + actual value)
pub struct ComplianceViolation {
    pub field:  String,
    pub rule:   String,
    pub actual: String,
}

/// Check a trade against playbook entry criteria — matches checkPlaybookCompliance()
pub fn check_playbook_compliance(
    trade: &Trade,
    criteria: &crate::models::EntryCriteria,
) -> Vec<ComplianceViolation> {
    let mut violations = Vec::new();

    // IVR check
    if let Some(ivr) = trade.iv_rank {
        if let Some(min) = criteria.min_ivr {
            if ivr < min {
                violations.push(ComplianceViolation {
                    field: "IV Rank".to_string(),
                    rule: format!("≥ {}", min),
                    actual: format!("{:.0}", ivr),
                });
            }
        }
        if let Some(max) = criteria.max_ivr {
            if ivr > max {
                violations.push(ComplianceViolation {
                    field: "IV Rank".to_string(),
                    rule: format!("≤ {}", max),
                    actual: format!("{:.0}", ivr),
                });
            }
        }
    }

    // Delta check
    if let Some(delta) = trade.delta {
        // Normalize: stored as decimal [0,1] → scale to 0-100; already-scaled values kept as-is
        let mut check_delta = if delta.abs() <= 1.0 { delta.abs() * 100.0 } else { delta.abs() };

        // TASTYTRADE IMPROVEMENT: For neutral strategies (IC, Strangle, etc.), the net delta
        // is often zero but the short legs are NOT safe. Use credit/width ratio as a surrogate.
        let is_neutral = matches!(trade.strategy, 
            crate::models::StrategyType::IronCondor |
            crate::models::StrategyType::IronButterfly |
            crate::models::StrategyType::Strangle |
            crate::models::StrategyType::Straddle |
            crate::models::StrategyType::CalendarSpread
        );
        
        if is_neutral && check_delta < 5.0 {
            // Surrogate Delta = (Credit / Width) * 100. 
            // e.g. $3.16 credit on $10 wings = 31.6 'delta' proxy.
            if let Some(width) = trade.spread_width {
                if width > 0.0 {
                    let surrogate = (trade.credit_received / width).abs() * 100.0;
                    if surrogate > check_delta {
                        check_delta = surrogate;
                    }
                }
            }
        }

        if let Some(min) = criteria.min_delta {
            if check_delta < min {
                violations.push(ComplianceViolation {
                    field: "Delta".to_string(),
                    rule: format!("≥ {}", min),
                    actual: format!("{:.0}", check_delta),
                });
            }
        }
        if let Some(max) = criteria.max_delta {
            if check_delta > max {
                violations.push(ComplianceViolation {
                    field: "Delta".to_string(),
                    rule: format!("≤ {}", max),
                    actual: format!("{:.0}", check_delta),
                });
            }
        }
    }

    // DTE check
    if let Some(dte) = trade.entry_dte {
        if let Some(min) = criteria.min_dte {
            if dte < min {
                violations.push(ComplianceViolation {
                    field: "DTE".to_string(),
                    rule: format!("≥ {}", min),
                    actual: format!("{}", dte),
                });
            }
        }
        if let Some(max) = criteria.max_dte {
            if dte > max {
                violations.push(ComplianceViolation {
                    field: "DTE".to_string(),
                    rule: format!("≤ {}", max),
                    actual: format!("{}", dte),
                });
            }
        }
    }

    // VIX check
    if let Some(vix) = trade.vix_at_entry {
        if let Some(min) = criteria.vix_min {
            if vix < min {
                violations.push(ComplianceViolation {
                    field: "VIX".to_string(),
                    rule: format!("≥ {:.1}", min),
                    actual: format!("{:.1}", vix),
                });
            }
        }
        if let Some(max) = criteria.vix_max {
            if vix > max {
                violations.push(ComplianceViolation {
                    field: "VIX".to_string(),
                    rule: format!("≤ {:.1}", max),
                    actual: format!("{:.1}", vix),
                });
            }
        }
    }

    // POP check
    if let Some(pop) = trade.pop {
        if let Some(min) = criteria.min_pop {
            if pop < min {
                violations.push(ComplianceViolation {
                    field: "POP".to_string(),
                    rule: format!("≥ {:.0}%", min),
                    actual: format!("{:.0}%", pop),
                });
            }
        }
    }

    violations
}

// ──────────────────────────────────────────────────────────────────────────────
// Performance Stats
// ──────────────────────────────────────────────────────────────────────────────

fn calculate_sharpe_ratio(returns: &[f64], risk_free_annual: f64) -> f64 {
    if returns.len() < 2 { return 0.0; }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    if std_dev <= 0.0 { return 0.0; }
    let risk_free_daily = risk_free_annual / 252.0;
    let daily_sharpe = (mean - risk_free_daily) / std_dev;
    daily_sharpe * 252.0_f64.sqrt()
}

/// Sortino ratio: (mean_return − rf) / downside_std × √252
/// Uses daily returns (pnl / account_size per day). Penalises only truly negative returns
/// (not returns below rf — that is the common but incorrect implementation).
fn calculate_sortino_ratio(returns: &[f64], risk_free_annual: f64) -> f64 {
    if returns.len() < 2 { return 0.0; }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let risk_free_daily = risk_free_annual / 252.0;
    let (d_count, d_sum_sq) = returns.iter().filter(|&&r| r < 0.0)
        .fold((0usize, 0.0_f64), |(cnt, sq), &r| (cnt + 1, sq + r.powi(2)));
    let downside_var = d_sum_sq / d_count.max(1) as f64;
    let downside_std = downside_var.sqrt();
    if downside_std <= 0.0 { return 0.0; }
    (mean - risk_free_daily) / downside_std * 252.0_f64.sqrt()
}

/// Calmar ratio: annual_return_pct / max_drawdown_pct
/// annual_return_pct = total_pnl / account_size / span_years × 100
fn calculate_calmar_ratio(total_pnl: f64, account_size: f64, span_days: f64, max_drawdown_pct: f64) -> f64 {
    if max_drawdown_pct <= 0.0 || span_days <= 0.0 { return 0.0; }
    let annual_return_pct = (total_pnl / account_size) / (span_days / 365.0) * 100.0;
    annual_return_pct / max_drawdown_pct
}

pub fn build_performance_stats(trades: &[Trade], account_size: f64, risk_free_rate_pct: f64, rolling_window: usize) -> crate::models::PerformanceStats {
    use chrono::Datelike;
    use crate::models::{PerformanceStats, StrategyBreakdown, TickerBreakdown, MonthlyPnl, IvrBucket, VixRegime, IvrEntryBucket, HeldBucket};

    // Step 1: filter and sort closed trades by exit_date ascending
    let mut sorted_closed: Vec<&Trade> = trades.iter()
        .filter(|t| t.exit_date.is_some() && t.pnl.is_some())
        .collect();
    sorted_closed.sort_by_key(|t| t.exit_date.unwrap());

    if sorted_closed.is_empty() {
        return PerformanceStats::default();
    }

    // Step 2: single pass
    let closed_count = sorted_closed.len() as f64;
    let mut gross_wins = 0.0_f64;
    let mut gross_losses = 0.0_f64;
    let mut win_count = 0.0_f64;
    let mut scratch_count = 0.0_f64;
    let mut loss_count = 0.0_f64;

    let mut pct_max_sum = 0.0_f64;
    let mut pct_max_count = 0u32;

    let mut dte_sum = 0.0_f64;
    let mut dte_count = 0u32;

    let mut held_sum = 0.0_f64;

    let mut ann_roc_sum   = 0.0_f64;
    let mut ann_roc_count = 0u32;

    // daily_pnl_map: NaiveDate -> total pnl on that date
    let mut daily_pnl_map: HashMap<chrono::NaiveDate, f64> = HashMap::new();

    // monthly_map: (year, month) -> (pnl, trade_count)
    let mut monthly_map: HashMap<(i32, u32), (f64, usize)> = HashMap::new();

    // strategy_map: StrategyType -> (trades, wins, scratches, total_pnl, roc_sum, roc_count)
    let mut strategy_map: HashMap<String, (usize, usize, usize, f64, f64, u32)> = HashMap::new();
    // per-strategy credit/width ratio accumulator (defined-risk only)
    let mut strategy_cw_map: HashMap<String, (f64, u32)> = HashMap::new();
    // per-strategy entry DTE accumulator
    let mut strategy_dte_map: HashMap<String, (f64, u32)> = HashMap::new();

    // ticker_map: ticker -> (trades, wins, scratches, total_pnl, roc_sum, roc_count)
    let mut ticker_map: HashMap<String, (usize, usize, usize, f64, f64, u32)> = HashMap::new();

    // credit/width ratio accumulator
    let mut cw_ratio_sum   = 0.0_f64;
    let mut cw_ratio_count = 0u32;

    // IV crush accumulator (entry_iv - close_iv, both as 0-1 decimals → display as % pts)
    let mut iv_crush_sum   = 0.0_f64;
    let mut iv_crush_count = 0u32;

    // IVR buckets: (trades, wins, pnl_sum) for [0-25), [25-50), [50-75), [75+)
    let mut ivr_data: [(usize, usize, f64); 4] = [(0, 0, 0.0); 4];

    // VIX regime buckets: (trades, wins, pnl_sum) for Calm/<15, Normal/15-20, Elevated/20-25, High/25-35, Stress/35+
    let mut vix_data: [(usize, usize, f64); 5] = [(0, 0, 0.0); 5];

    // balance history: start at account_size, push running after each trade
    let mut balance_history: Vec<f64> = vec![account_size];
    let mut running = account_size;

    // L6: premium recapture
    let mut recapture_sum   = 0.0_f64;
    let mut recapture_count = 0u32;

    // L7: per-trade theta capture pct (filled in loop, used for rolling 30-window after)
    let mut theta_capture_per_trade: Vec<f64> = Vec::new();

    // L10: IVR entry histogram: (count, wins) for [<25, 25-50, 50-75, 75+]
    let mut ivr_entry_data: [(usize, usize); 4] = [(0, 0); 4];

    // Commission analysis accumulators
    let mut total_commissions = 0.0_f64;
    let mut comm_count = 0u32;
    let mut fill_vs_mid_sum = 0.0_f64;
    let mut fill_vs_mid_count = 0u32;

    for t in &sorted_closed {
        let pnl = t.pnl.unwrap();
        let exit = t.exit_date.unwrap();
        let exit_date_naive = exit.date_naive();

        // Scratch threshold: 5% of max profit per trade (min $10 for tiny trades)
        let max_profit = calculate_max_profit(t.credit_received, t.quantity);
        let scratch_threshold = (max_profit * 0.05).max(10.0);
        if pnl.abs() < scratch_threshold {
            scratch_count += 1.0;
        } else if pnl > 0.0 {
            gross_wins += pnl;
            win_count += 1.0;
        } else {
            gross_losses += pnl.abs();
            loss_count += 1.0;
        }

        // Avg % max captured (winners only)
        if max_profit > 0.0 && pnl > 0.0 {
            pct_max_sum += (pnl / max_profit) * 100.0;
            pct_max_count += 1;
        }

        // Per-trade ROC for strategy breakdown
        let roc_opt = calculate_roc(pnl, &t.legs, t.credit_received, t.quantity, &t.spread_type(), t.bpr, t.underlying_price);

        // DTE at close
        if let Some(dte) = t.dte_at_close {
            dte_sum += dte as f64;
            dte_count += 1;
        }

        // Held days
        let held_days = (exit_date_naive - t.trade_date.date_naive()).num_days().max(0);
        held_sum += held_days as f64;

        // Annualized ROC — cap at 300% to prevent <7 DTE trades from inflating the average
        if let Some(roc) = roc_opt {
            if held_days > 0 {
                let raw_ann = roc * (365.0 / held_days as f64);
                ann_roc_sum   += raw_ann.clamp(-300.0, 300.0); // cap prevents <7 DTE inflation
                ann_roc_count += 1;
            }
        }

        // Daily P&L
        *daily_pnl_map.entry(exit_date_naive).or_insert(0.0) += pnl;

        // Monthly P&L
        let month_key = (exit_date_naive.year(), exit_date_naive.month());
        let entry = monthly_map.entry(month_key).or_insert((0.0, 0));
        entry.0 += pnl;
        entry.1 += 1;

        // Strategy breakdown
        let strat_key = format!("{:?}", t.strategy);
        let se = strategy_map.entry(strat_key.clone()).or_insert((0, 0, 0, 0.0, 0.0, 0));
        se.0 += 1;
        if pnl.abs() < scratch_threshold { se.2 += 1; }
        else if pnl > 0.0 { se.1 += 1; }
        se.3 += pnl;
        if let Some(roc) = roc_opt {
            se.4 += roc;
            se.5 += 1;
        }
        // per-strategy credit/width ratio
        let cw_strat = calculate_credit_width_ratio(t.credit_received, &t.legs, t.spread_type());
        if cw_strat > 0.0 {
            let cwe = strategy_cw_map.entry(strat_key.clone()).or_insert((0.0, 0));
            cwe.0 += cw_strat;
            cwe.1 += 1;
        }
        // per-strategy entry DTE
        if let Some(dte_e) = t.entry_dte {
            if dte_e > 0 {
                let de = strategy_dte_map.entry(strat_key).or_insert((0.0, 0));
                de.0 += dte_e as f64;
                de.1 += 1;
            }
        }

        // Ticker breakdown
        let te = ticker_map.entry(t.ticker.clone()).or_insert((0, 0, 0, 0.0, 0.0, 0));
        te.0 += 1;
        if pnl.abs() < scratch_threshold { te.2 += 1; }
        else if pnl > 0.0 { te.1 += 1; }
        te.3 += pnl;
        if let Some(roc) = roc_opt {
            te.4 += roc;
            te.5 += 1;
        }

        // Credit/width ratio (defined-risk strategies only)
        let cw = calculate_credit_width_ratio(t.credit_received, &t.legs, t.spread_type());
        if cw > 0.0 {
            cw_ratio_sum   += cw;
            cw_ratio_count += 1;
        }

        // IV crush: entry_iv - iv_at_close (both stored as whole % e.g. 30.0 = 30%)
        if let (Some(entry_iv), Some(close_iv)) = (t.implied_volatility, t.iv_at_close) {
            iv_crush_sum   += entry_iv - close_iv; // already in % pts — no multiply needed
            iv_crush_count += 1;
        }

        // IVR bucket
        if let Some(ivr) = t.iv_rank {
            let bucket = if ivr < 25.0 { 0 } else if ivr < 50.0 { 1 } else if ivr < 75.0 { 2 } else { 3 };
            ivr_data[bucket].0 += 1;
            if pnl > 0.0 { ivr_data[bucket].1 += 1; }
            ivr_data[bucket].2 += pnl;
        }

        // VIX regime
        if let Some(vix) = t.vix_at_entry {
            let bucket = if vix < 15.0 { 0 } else if vix < 20.0 { 1 } else if vix < 30.0 { 2 } else if vix < 40.0 { 3 } else { 4 };
            vix_data[bucket].0 += 1;
            if pnl > 0.0 { vix_data[bucket].1 += 1; }
            vix_data[bucket].2 += pnl;
        }

        // L6: premium recapture
        if t.credit_received > 0.0 {
            if let Some(debit) = t.debit_paid {
                let recapture = (t.credit_received - debit) / t.credit_received * 100.0;
                recapture_sum   += recapture;
                recapture_count += 1;
            }
        }

        // L7: per-trade theta capture pct
        if let (Some(theta), Some(debit)) = (t.theta, t.debit_paid) {
            if theta.abs() > 0.0 {
                let hd = (exit_date_naive - t.trade_date.date_naive()).num_days().max(0) as f64;
                if hd > 0.0 {
                    let max_theta_pnl = theta.abs() * 100.0 * t.quantity as f64 * hd;
                    if max_theta_pnl > 0.0 {
                        let tc_pct = (pnl / max_theta_pnl * 100.0).clamp(-200.0, 200.0);
                        theta_capture_per_trade.push(tc_pct);
                    } else {
                        theta_capture_per_trade.push(0.0);
                    }
                } else {
                    theta_capture_per_trade.push(0.0);
                }
            } else {
                theta_capture_per_trade.push(0.0);
            }
            let _ = debit; // suppress unused warning
        }
        // trades with None theta or None debit_paid are excluded from the rolling window
        // to avoid distorting the average with meaningless zeros

        // Commission tracking
        if let Some(comm) = t.commission {
            if comm > 0.0 {
                total_commissions += comm;
                comm_count += 1;
            }
        }

        // Fill vs mid
        if let Some(fvm) = t.fill_vs_mid {
            fill_vs_mid_sum += fvm;
            fill_vs_mid_count += 1;
        }

        // L10: IVR entry histogram
        if let Some(ivr) = t.iv_rank {
            let b = if ivr < 25.0 { 0 } else if ivr < 50.0 { 1 } else if ivr < 75.0 { 2 } else { 3 };
            ivr_entry_data[b].0 += 1;
            if pnl > 0.0 { ivr_entry_data[b].1 += 1; }
        }

        // Balance history
        running += pnl;
        balance_history.push(running);
    }

    // Step 3: aggregates
    let win_rate = win_count / closed_count;
    let scratch_rate = scratch_count / closed_count;
    let avg_win = if win_count > 0.0 { gross_wins / win_count } else { 0.0 };
    let avg_loss = if loss_count > 0.0 { gross_losses / loss_count } else { 0.0 };
    let profit_factor = if gross_losses > 0.0 { gross_wins / gross_losses } else { 999.9 };
    let expected_value = win_rate * avg_win - (1.0 - win_rate) * avg_loss;

    // Step 4: Sharpe
    let daily_returns: Vec<f64> = daily_pnl_map.values()
        .map(|p| p / account_size)
        .collect();
    let sharpe_ratio = calculate_sharpe_ratio(&daily_returns, risk_free_rate_pct / 100.0);

    // Step 5: trade frequency
    let first_exit = sorted_closed.first().unwrap().exit_date.unwrap().date_naive();
    let last_exit  = sorted_closed.last().unwrap().exit_date.unwrap().date_naive();
    let span_days  = (last_exit - first_exit).num_days().max(1) as f64;
    let (trades_per_week, trades_per_month) = if sorted_closed.len() > 1 {
        (closed_count / (span_days / 7.0), closed_count / (span_days / 30.4375))
    } else {
        (0.0, 0.0)
    };

    let avg_held_days = held_sum / closed_count;

    // Step 6: strategy breakdown
    let mut strategy_breakdown: Vec<StrategyBreakdown> = strategy_map.into_iter().map(|(key, (trades, wins, scratches, total_pnl, roc_sum, roc_count))| {
        // Recover the StrategyType from its Debug string
        let strategy = sorted_closed.iter()
            .find(|t| format!("{:?}", t.strategy) == key)
            .map(|t| t.strategy.clone())
            .unwrap_or(StrategyType::ShortPutVertical);
        let tc = trades as f64;
        let avg_cw_ratio = strategy_cw_map.get(&key).and_then(|&(sum, count)| {
            if count > 0 { Some(sum / count as f64) } else { None }
        });
        let avg_entry_dte = strategy_dte_map.get(&key).and_then(|&(sum, count)| {
            if count > 0 { Some(sum / count as f64) } else { None }
        });
        StrategyBreakdown {
            strategy,
            trades,
            wins,
            scratches,
            total_pnl,
            avg_pnl: if tc > 0.0 { total_pnl / tc } else { 0.0 },
            avg_roc: if roc_count > 0 { roc_sum / roc_count as f64 } else { 0.0 },
            win_rate: if tc > 0.0 { wins as f64 / tc * 100.0 } else { 0.0 },
            scratch_rate: if tc > 0.0 { scratches as f64 / tc * 100.0 } else { 0.0 },
            avg_cw_ratio,
            avg_entry_dte,
        }
    }).collect();
    strategy_breakdown.sort_by(|a, b| b.trades.cmp(&a.trades));

    // Step 7a: ticker breakdown
    let mut ticker_breakdown: Vec<TickerBreakdown> = ticker_map.into_iter().map(|(ticker, (trades, wins, scratches, total_pnl, roc_sum, roc_count))| {
        let tc = trades as f64;
        TickerBreakdown {
            ticker,
            trades,
            wins,
            scratches,
            total_pnl,
            avg_pnl: if tc > 0.0 { total_pnl / tc } else { 0.0 },
            avg_roc: if roc_count > 0 { roc_sum / roc_count as f64 } else { 0.0 },
            win_rate: if tc > 0.0 { wins as f64 / tc * 100.0 } else { 0.0 },
            scratch_rate: if tc > 0.0 { scratches as f64 / tc * 100.0 } else { 0.0 },
        }
    }).collect();
    ticker_breakdown.sort_by(|a, b| b.trades.cmp(&a.trades));

    // Step 7: monthly P&L
    let mut monthly_pnl: Vec<MonthlyPnl> = monthly_map.into_iter().map(|((year, month), (pnl, trade_count))| {
        MonthlyPnl { year, month, pnl, trade_count }
    }).collect();
    monthly_pnl.sort_by(|a, b| (a.year, a.month).cmp(&(b.year, b.month)));

    let avg_annualized_roc = if ann_roc_count > 0 { ann_roc_sum / ann_roc_count as f64 } else { 0.0 };
    let sortino_ratio = calculate_sortino_ratio(&daily_returns, risk_free_rate_pct / 100.0);
    let total_pnl = gross_wins - gross_losses;
    let (_, max_drawdown_pct, _) = calculate_drawdown(&balance_history);
    let calmar_ratio = calculate_calmar_ratio(total_pnl, account_size, span_days, max_drawdown_pct);

    // IVR buckets
    let ivr_bucket_defs: [(&str, f64, f64); 4] = [
        ("IVR  0-25",  0.0,  25.0),
        ("IVR 25-50", 25.0,  50.0),
        ("IVR 50-75", 50.0,  75.0),
        ("IVR   75+", 75.0, 100.0),
    ];
    let ivr_buckets: Vec<IvrBucket> = ivr_bucket_defs.iter().zip(ivr_data.iter()).map(|((label, min, max), (trades, wins, pnl_sum))| {
        IvrBucket {
            label,
            min_ivr: *min,
            max_ivr: *max,
            trades: *trades,
            wins: *wins,
            win_rate: if *trades > 0 { *wins as f64 / *trades as f64 * 100.0 } else { 0.0 },
            avg_pnl: if *trades > 0 { pnl_sum / *trades as f64 } else { 0.0 },
        }
    }).collect();

    // VIX regimes
    let vix_regime_defs: [&str; 5] = ["Calm (<15)", "Normal (15-20)", "Elevated (20-30)", "High (30-40)", "Stress (40+)"];
    let vix_regimes: Vec<VixRegime> = vix_regime_defs.iter().zip(vix_data.iter()).map(|(label, (trades, wins, pnl_sum))| {
        VixRegime {
            label,
            trades: *trades,
            wins: *wins,
            win_rate: if *trades > 0 { *wins as f64 / *trades as f64 * 100.0 } else { 0.0 },
            avg_pnl: if *trades > 0 { pnl_sum / *trades as f64 } else { 0.0 },
        }
    }).collect();

    // M3: DTE-at-close buckets: 0-7, 7-14, 14-21, 21-30, 30+
    let dte_bucket_defs: [(&str, i32, i32); 5] = [
        ("0–7d",  0,  7),
        ("7–14d", 7, 14),
        ("14–21d",14, 21),
        ("21–30d",21, 30),
        ("30+d",  30, i32::MAX),
    ];
    let mut dte_data: [(usize, usize, f64); 5] = [(0, 0, 0.0); 5];
    for t in &sorted_closed {
        if let (Some(dte_close), Some(pnl)) = (t.dte_at_close, t.pnl) {
            let bi = dte_bucket_defs.iter().position(|&(_, lo, hi)| dte_close >= lo && dte_close < hi)
                .unwrap_or(4);
            dte_data[bi].0 += 1;
            if pnl > 0.0 { dte_data[bi].1 += 1; }
            dte_data[bi].2 += pnl;
        }
    }
    use crate::models::DteBucket;
    let dte_buckets: Vec<DteBucket> = dte_bucket_defs.iter().zip(dte_data.iter()).map(|((label, _, _), (trades, wins, pnl_sum))| {
        DteBucket {
            label,
            trades: *trades,
            wins: *wins,
            avg_pnl: if *trades > 0 { pnl_sum / *trades as f64 } else { 0.0 },
            win_rate: if *trades > 0 { *wins as f64 / *trades as f64 * 100.0 } else { 0.0 },
        }
    }).collect();

    // M8: rolling 30-trade win rate
    let rolling_win_rate: Vec<f64> = sorted_closed.iter().enumerate().map(|(i, _)| {
        let start = i.saturating_sub(rolling_window.saturating_sub(1));
        let window = &sorted_closed[start..=i];
        let wins = window.iter().filter(|t| t.pnl.unwrap_or(0.0) > 0.0).count();
        wins as f64 / window.len() as f64 * 100.0
    }).collect();

    // M9: running peak balance
    let mut peak = account_size;
    let peak_history: Vec<f64> = balance_history.iter().map(|&b| { peak = peak.max(b); peak }).collect();

    // L6: avg premium recapture
    let avg_premium_recapture = if recapture_count > 0 {
        Some(recapture_sum / recapture_count as f64)
    } else {
        None
    };

    // L7: rolling 30-trade theta capture
    let rolling_theta_capture: Vec<f64> = theta_capture_per_trade.iter().enumerate().map(|(i, _)| {
        let start = i.saturating_sub(rolling_window.saturating_sub(1));
        let window = &theta_capture_per_trade[start..=i];
        window.iter().sum::<f64>() / window.len() as f64
    }).collect();

    // L10: IVR entry buckets
    let ivr_entry_bucket_defs: [&str; 4] = ["IVR  <25", "IVR 25-50", "IVR 50-75", "IVR  75+"];
    let ivr_entry_buckets: Vec<IvrEntryBucket> = ivr_entry_bucket_defs.iter().zip(ivr_entry_data.iter()).map(|(label, (count, wins))| {
        IvrEntryBucket {
            label,
            count: *count,
            win_rate: if *count > 0 { *wins as f64 / *count as f64 * 100.0 } else { 0.0 },
        }
    }).collect();

    // Item 4: Time-in-Trade histogram (5 buckets by calendar days held)
    let held_bucket_defs: [(&'static str, i64, i64); 5] = [
        ("0–7d",   0,  7),
        ("7–14d",  7, 14),
        ("14–21d", 14, 21),
        ("21–30d", 21, 30),
        ("30+d",   30, i64::MAX),
    ];
    let mut held_data: [(usize, usize, f64); 5] = [(0, 0, 0.0); 5];
    for t in &sorted_closed {
        if let Some(pnl) = t.pnl {
            if let Some(exit) = t.exit_date {
                let days = (exit.date_naive() - t.trade_date.date_naive()).num_days().max(0);
                if let Some(bi) = held_bucket_defs.iter().position(|&(_, lo, hi)| days >= lo && days < hi) {
                    held_data[bi].0 += 1;
                    if pnl > 0.0 { held_data[bi].1 += 1; }
                    held_data[bi].2 += pnl;
                }
            }
        }
    }
    let held_buckets: Vec<HeldBucket> = held_bucket_defs.iter().zip(held_data.iter()).map(|((label, _, _), (trades, wins, pnl_sum))| {
        HeldBucket {
            label,
            trades: *trades,
            wins: *wins,
            win_rate: if *trades > 0 { *wins as f64 / *trades as f64 * 100.0 } else { 0.0 },
            avg_pnl: if *trades > 0 { pnl_sum / *trades as f64 } else { 0.0 },
        }
    }).collect();

    let avg_commission_per_trade = if comm_count > 0 { total_commissions / comm_count as f64 } else { 0.0 };
    let commission_pct_of_gross = if gross_wins > 0.0 { (total_commissions / gross_wins) * 100.0 } else { 0.0 };
    let avg_fill_vs_mid = if fill_vs_mid_count > 0 { Some(fill_vs_mid_sum / fill_vs_mid_count as f64) } else { None };

    // Item 4: P&L distribution histogram (6 buckets)
    use crate::models::PnlBucket;
    let pnl_bucket_defs: [(&'static str, f64, f64); 6] = [
        ("<  -$500",       f64::NEG_INFINITY, -500.0),
        ("-$500 to -$100", -500.0,            -100.0),
        ("-$100 to $0",    -100.0,               0.0),
        ("  $0 to $100",      0.0,             100.0),
        ("$100 to $500",    100.0,             500.0),
        (">   $500",         500.0, f64::INFINITY),
    ];
    let closed_count = sorted_closed.len();
    let mut pnl_counts = [0usize; 6];
    for t in &sorted_closed {
        if let Some(pnl) = t.pnl {
            if let Some(bi) = pnl_bucket_defs.iter().position(|&(_, lo, hi)| pnl >= lo && pnl < hi) {
                pnl_counts[bi] += 1;
            }
        }
    }
    let pnl_buckets: Vec<PnlBucket> = pnl_bucket_defs.iter().zip(pnl_counts.iter()).map(|((label, _, _), &count)| {
        PnlBucket {
            label,
            count,
            pct: if closed_count > 0 { count as f64 / closed_count as f64 * 100.0 } else { 0.0 },
        }
    }).collect();

    PerformanceStats {
        win_rate: win_rate * 100.0,
        scratch_rate: scratch_rate * 100.0,
        scratches: scratch_count as usize,
        avg_win,
        avg_loss,
        profit_factor,
        expected_value,
        sharpe_ratio,
        sortino_ratio,
        calmar_ratio,
        avg_annualized_roc,
        avg_dte_at_close: if dte_count > 0 { Some(dte_sum / dte_count as f64) } else { None },
        avg_pct_max_captured: if pct_max_count > 0 { Some(pct_max_sum / pct_max_count as f64) } else { None },
        avg_credit_width_ratio: if cw_ratio_count > 0 { Some(cw_ratio_sum / cw_ratio_count as f64) } else { None },
        avg_iv_crush: if iv_crush_count > 0 { Some(iv_crush_sum / iv_crush_count as f64) } else { None },
        trades_per_week,
        trades_per_month,
        avg_held_days,
        strategy_breakdown,
        ticker_breakdown,
        monthly_pnl,
        balance_history,
        ivr_buckets,
        vix_regimes,
        dte_buckets,
        rolling_win_rate,
        peak_history,
        avg_premium_recapture,
        rolling_theta_capture,
        ivr_entry_buckets,
        held_buckets,
        total_commissions,
        avg_commission_per_trade,
        commission_pct_of_gross,
        avg_fill_vs_mid,
        rolling_window_used: rolling_window,
        pnl_buckets,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// L8: Auto-match trades to playbooks
// ──────────────────────────────────────────────────────────────────────────────

/// Find playbook IDs that match the trade's strategy, IVR, DTE, and delta.
/// Returns a list of matching playbook IDs (may be empty, 1, or many).
pub fn find_matching_playbooks(
    trade: &Trade,
    playbooks: &[crate::models::PlaybookStrategy],
) -> Vec<i32> {
    let trade_badge = trade.strategy.badge();
    let mut matches = Vec::new();

    for pb in playbooks {
        // Must match spread type if playbook specifies one
        if let Some(ref st) = pb.spread_type {
            let pb_badge = crate::models::StrategyType::from_str(st).badge();
            if pb_badge != trade_badge { continue; }
        }

        // Check entry criteria if present
        if let Some(ref ec) = pb.entry_criteria {
            let mut pass = true;

            if let Some(ivr) = trade.iv_rank {
                if let Some(min) = ec.min_ivr { if ivr < min { pass = false; } }
                if let Some(max) = ec.max_ivr { if ivr > max { pass = false; } }
            }

            if let Some(dte) = trade.entry_dte {
                if let Some(min) = ec.min_dte { if dte < min { pass = false; } }
                if let Some(max) = ec.max_dte { if dte > max { pass = false; } }
            }

            if let Some(delta) = trade.delta {
                let d = if delta.abs() <= 1.0 { delta.abs() * 100.0 } else { delta.abs() };
                if let Some(min) = ec.min_delta { if d < min { pass = false; } }
                if let Some(max) = ec.max_delta { if d > max { pass = false; } }
            }

            if !pass { continue; }
        }

        matches.push(pb.id);
    }

    matches
}

// ──────────────────────────────────────────────────────────────────────────────
// L5: Payoff series for ASCII chart
// ──────────────────────────────────────────────────────────────────────────────

/// Build a payoff series for the ASCII chart: (price, pnl) pairs over the range
/// [underlying * 0.70, underlying * 1.30] in `steps` steps.
/// Also returns the expected move (±em) if IV data is available.
pub fn build_payoff_series(
    trade: &Trade,
    steps: usize,
) -> (Vec<(f64, f64)>, Option<f64>) {
    let underlying = match trade.underlying_price {
        Some(u) if u > 0.0 => u,
        _ => return (vec![], None),
    };

    let lo = underlying * 0.70;
    let hi = underlying * 1.30;
    let step_size = (hi - lo) / (steps.saturating_sub(1).max(1)) as f64;

    let series: Vec<(f64, f64)> = (0..steps).map(|i| {
        let price = lo + i as f64 * step_size;
        let pnl = calculate_payoff_at_price(&trade.legs, trade.credit_received, price)
            * trade.quantity as f64;
        (price, pnl)
    }).collect();

    // Expected move: underlying * (iv/100) * sqrt(dte/365)
    let iv = trade.implied_volatility.or_else(|| trade.iv_rank).unwrap_or(0.0);
    let dte = calculate_remaining_dte(&trade.expiration_date).max(0) as f64;
    let expected_move = if iv > 0.0 && dte > 0.0 {
        let iv_dec = if iv > 2.0 { iv / 100.0 } else { iv };
        Some(underlying * iv_dec * (dte / 365.0).sqrt())
    } else {
        None
    };

    (series, expected_move)
}

/// tastytrade "Opportunities in Extremes" — VIX -> max BPR % of net liq.
pub fn vix_max_heat(vix: f64) -> f64 {
    if      vix >= 40.0 { 50.0 }
    else if vix >= 30.0 { 40.0 }
    else if vix >= 20.0 { 35.0 }
    else if vix >= 15.0 { 30.0 }
    else                { 25.0 }
}
