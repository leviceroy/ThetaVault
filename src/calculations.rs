/// Calculations — exact port of OptionsTradingJournal/client/src/lib/trade-calculations.ts
///
/// All formulas are matched 100% to the TypeScript original.
use crate::models::{LegType, StrategyType, TradeLeg, Trade};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

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
    // Total = greater leg margin + lesser leg's premium
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
            let margin = if put_margin >= call_margin {
                put_margin + sc.premium
            } else {
                call_margin + sp.premium
            };
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
) -> Option<f64> {
    // Covered Call: stock margin (BPR) is the only valid capital basis.
    // If BPR not stored (old trades), return None — display "—" not "0%".
    if spread_type == "covered_call" {
        return bpr.map(|b| if b > 0.0 { (pnl / b) * 100.0 } else { 0.0 });
    }

    let mut capital_at_risk =
        calculate_max_loss_from_legs(legs, credit_received, quantity, spread_type);

    // Cash Secured Put: prefer BPR (20% margin = tastytrade platform basis).
    if spread_type == "cash_secured_put" {
        if let Some(b) = bpr { if b > 0.0 { capital_at_risk = b; } }
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
pub fn calculate_breakevens(legs: &[TradeLeg], spread_type: &str) -> Vec<f64> {
    if legs.is_empty() {
        return vec![];
    }

    // Net credit from legs
    let credit: f64 = legs.iter().map(|l| {
        if l.leg_type.is_short() { l.premium } else { -l.premium }
    }).sum();

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
        _ => {}
    }

    vec![]
}

/// P&L at expiration for one underlying price, at qty = 1 contract (in dollars).
/// Callers multiply by trade.quantity for total P&L.
/// Note: calendar/diagonal spreads are IV-dependent and not supported here.
pub fn calculate_payoff_at_price(legs: &[TradeLeg], credit_received: f64, price: f64) -> f64 {
    let intrinsic: f64 = legs.iter().map(|leg| {
        match leg.leg_type {
            LegType::ShortPut  => -(leg.strike - price).max(0.0),
            LegType::LongPut   =>  (leg.strike - price).max(0.0),
            LegType::ShortCall => -(price - leg.strike).max(0.0),
            LegType::LongCall  =>  (price - leg.strike).max(0.0),
        }
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
        "custom" => {
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
    let mut net_bwd      = 0.0_f64;

    // OTJ Dashboard metrics for open trades
    let mut total_open_bpr  = 0.0_f64;
    let mut undefined_bpr   = 0.0_f64;
    let mut defined_bpr     = 0.0_f64;
    let mut pop_sum         = 0.0_f64;
    let mut pop_count       = 0usize;

    // Build cumulative balance history for drawdown
    let mut balance_history: Vec<f64> = vec![0.0];
    let mut running = 0.0_f64;

    let all_refs: Vec<&Trade> = trades.iter().collect();

    for trade in trades {
        if let Some(pnl) = trade.pnl {
            realized_pnl += pnl;
            total_pnl    += pnl;
            closed_count += 1;

            if pnl > 0.0 { wins += 1; }
            if pnl > best_pnl  { best_pnl  = pnl; }
            if pnl < worst_pnl { worst_pnl = pnl; }

            if let Some(roc) = calculate_roc(pnl, &trade.legs, trade.credit_received, trade.quantity, trade.spread_type(), trade.bpr) {
                if roc.abs() > 0.001 {
                    roc_sum   += roc;
                    roc_count += 1;
                }
            }

            running += pnl;
            balance_history.push(running);
        } else {
            open_count += 1;

            // Net theta from open positions
            if let Some(th) = trade.theta {
                net_theta += th * 100.0 * trade.quantity as f64;
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
        unrealized_pnl,
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

fn delta_based_pop_fallback(trade: &Trade) -> f64 {
    match trade.strategy {
        StrategyType::IronCondor | StrategyType::IronButterfly => 68.0,
        StrategyType::Strangle | StrategyType::Straddle => 66.0,
        _ => {
            let delta_abs = trade.delta.map(|d| d.abs()).unwrap_or(0.30);
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
        let abs_delta = delta.abs() * 100.0;
        let check_delta = if abs_delta > 1.0 { abs_delta } else { abs_delta * 100.0 };
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

pub fn build_performance_stats(trades: &[Trade], account_size: f64) -> crate::models::PerformanceStats {
    use chrono::Datelike;
    use crate::models::{PerformanceStats, StrategyBreakdown, MonthlyPnl};

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
    let mut loss_count = 0.0_f64;

    let mut pct_max_sum = 0.0_f64;
    let mut pct_max_count = 0u32;

    let mut dte_sum = 0.0_f64;
    let mut dte_count = 0u32;

    let mut held_sum = 0.0_f64;

    // daily_pnl_map: NaiveDate -> total pnl on that date
    let mut daily_pnl_map: HashMap<chrono::NaiveDate, f64> = HashMap::new();

    // monthly_map: (year, month) -> (pnl, trade_count)
    let mut monthly_map: HashMap<(i32, u32), (f64, usize)> = HashMap::new();

    // strategy_map: StrategyType -> (trades, wins, total_pnl, roc_sum, roc_count)
    let mut strategy_map: HashMap<String, (usize, usize, f64, f64, u32)> = HashMap::new();

    // balance history: start at account_size, push running after each trade
    let mut balance_history: Vec<f64> = vec![account_size];
    let mut running = account_size;

    for t in &sorted_closed {
        let pnl = t.pnl.unwrap();
        let exit = t.exit_date.unwrap();
        let exit_date_naive = exit.date_naive();

        if pnl > 0.0 {
            gross_wins += pnl;
            win_count += 1.0;
        } else {
            gross_losses += pnl.abs();
            loss_count += 1.0;
        }

        // Avg % max captured (winners only)
        let max_profit = calculate_max_profit(t.credit_received, t.quantity);
        if max_profit > 0.0 && pnl > 0.0 {
            pct_max_sum += (pnl / max_profit) * 100.0;
            pct_max_count += 1;
        }

        // Per-trade ROC for strategy breakdown
        let roc_opt = calculate_roc(pnl, &t.legs, t.credit_received, t.quantity, &t.spread_type(), t.bpr);

        // DTE at close
        if let Some(dte) = t.dte_at_close {
            dte_sum += dte as f64;
            dte_count += 1;
        }

        // Held days
        let held_days = (exit_date_naive - t.trade_date.date_naive()).num_days().max(0);
        held_sum += held_days as f64;

        // Daily P&L
        *daily_pnl_map.entry(exit_date_naive).or_insert(0.0) += pnl;

        // Monthly P&L
        let month_key = (exit_date_naive.year(), exit_date_naive.month());
        let entry = monthly_map.entry(month_key).or_insert((0.0, 0));
        entry.0 += pnl;
        entry.1 += 1;

        // Strategy breakdown
        let strat_key = format!("{:?}", t.strategy);
        let se = strategy_map.entry(strat_key).or_insert((0, 0, 0.0, 0.0, 0));
        se.0 += 1;
        if pnl > 0.0 { se.1 += 1; }
        se.2 += pnl;
        if let Some(roc) = roc_opt {
            se.3 += roc;
            se.4 += 1;
        }

        // Balance history
        running += pnl;
        balance_history.push(running);
    }

    // Step 3: aggregates
    let win_rate = win_count / closed_count;
    let avg_win = if win_count > 0.0 { gross_wins / win_count } else { 0.0 };
    let avg_loss = if loss_count > 0.0 { gross_losses / loss_count } else { 0.0 };
    let profit_factor = if gross_losses > 0.0 { gross_wins / gross_losses } else { f64::INFINITY };
    let expected_value = win_rate * avg_win - (1.0 - win_rate) * avg_loss;

    // Step 4: Sharpe
    let daily_returns: Vec<f64> = daily_pnl_map.values()
        .map(|p| p / account_size)
        .collect();
    let sharpe_ratio = calculate_sharpe_ratio(&daily_returns, 0.045);

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
    let mut strategy_breakdown: Vec<StrategyBreakdown> = strategy_map.into_iter().map(|(key, (trades, wins, total_pnl, roc_sum, roc_count))| {
        // Recover the StrategyType from its Debug string
        let strategy = sorted_closed.iter()
            .find(|t| format!("{:?}", t.strategy) == key)
            .map(|t| t.strategy.clone())
            .unwrap_or(StrategyType::ShortPutVertical);
        let w = wins as f64;
        let tc = trades as f64;
        StrategyBreakdown {
            strategy,
            trades,
            wins,
            total_pnl,
            avg_pnl: if tc > 0.0 { total_pnl / tc } else { 0.0 },
            avg_roc: if roc_count > 0 { roc_sum / roc_count as f64 } else { 0.0 },
            win_rate: if tc > 0.0 { w / tc * 100.0 } else { 0.0 },
        }
    }).collect();
    strategy_breakdown.sort_by(|a, b| b.trades.cmp(&a.trades));

    // Step 7: monthly P&L
    let mut monthly_pnl: Vec<MonthlyPnl> = monthly_map.into_iter().map(|((year, month), (pnl, trade_count))| {
        MonthlyPnl { year, month, pnl, trade_count }
    }).collect();
    monthly_pnl.sort_by(|a, b| (a.year, a.month).cmp(&(b.year, b.month)));

    PerformanceStats {
        avg_win,
        avg_loss,
        profit_factor,
        expected_value,
        sharpe_ratio,
        avg_dte_at_close: if dte_count > 0 { Some(dte_sum / dte_count as f64) } else { None },
        avg_pct_max_captured: if pct_max_count > 0 { Some(pct_max_sum / pct_max_count as f64) } else { None },
        trades_per_week,
        trades_per_month,
        avg_held_days,
        strategy_breakdown,
        monthly_pnl,
        balance_history,
    }
}
