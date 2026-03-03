use crate::models::{Trade, StrategyType};
use crate::calculations::calculate_remaining_dte;
use std::collections::HashMap;
use chrono::Utc;

/// Urgency level — lower number sorts first (most urgent).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Critical = 0,  // DEFENSE / expiration today
    High     = 1,  // WARNING / earnings imminent
    Medium   = 2,  // MANAGE  / 21 DTE
    Low      = 3,  // CLOSE   / profit target
    Info     = 4,  // ROLL / SIZING advisory
    Ok       = 5,  // No action needed
}

/// The action category.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertKind {
    Defense,  // Tested position — needs immediate attention
    Warning,  // Earnings risk while short premium
    Manage,   // 21 DTE management trigger
    Close,    // Profit target hit / GTC reminder
    Roll,     // Roll for credit at 21 DTE
    Sizing,   // BPR oversized
    Ok,       // All clear
}

impl AlertKind {
    pub fn badge(&self) -> &'static str {
        match self {
            AlertKind::Defense => "DEFENSE",
            AlertKind::Warning => "WARNING",
            AlertKind::Manage  => "MANAGE ",
            AlertKind::Close   => "CLOSE  ",
            AlertKind::Roll    => "ROLL   ",
            AlertKind::Sizing  => "SIZING ",
            AlertKind::Ok      => "OK     ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradeAlert {
    pub trade_id: i32,
    pub ticker:   String,
    pub strategy_badge: String,    // "SPV", "IC", etc.
    pub kind:     AlertKind,
    pub severity: AlertSeverity,
    pub headline: String,          // e.g. "$AAPL SPV — Tested (spot $182 < strike $185)"
    pub detail:   Option<String>,  // e.g. "Buy back or roll the short put"
}

/// True for strategies where you collect credit and time works for you.
fn is_short_premium(strategy: &StrategyType) -> bool {
    matches!(strategy,
        StrategyType::ShortPutVertical
        | StrategyType::ShortCallVertical
        | StrategyType::IronCondor
        | StrategyType::IronButterfly
        | StrategyType::Strangle
        | StrategyType::Straddle
        | StrategyType::CashSecuredPut
        | StrategyType::CoveredCall
        | StrategyType::ShortDiagonalSpread
    )
}

/// Compute all alerts for the current open positions.
/// Sorted by severity (Critical first).
pub fn compute_alerts(
    trades:       &[Trade],
    live_prices:  &HashMap<String, f64>,
    account_size: f64,
    current_vix:  Option<f64>,
) -> Vec<TradeAlert> {
    let today = Utc::now().date_naive();
    let mut alerts: Vec<TradeAlert> = Vec::new();

    for trade in trades.iter().filter(|t| t.is_open()) {
        let dte       = calculate_remaining_dte(&trade.expiration_date);
        let live_spot = live_prices.get(&trade.ticker).copied();
        let strategy  = &trade.strategy;
        let badge     = strategy.badge().to_string();
        let entry_dte = trade.entry_dte.unwrap_or(45);

        // ── DEFENSE: Tested position ─────────────────────────────────────────
        let is_tested = trade.is_tested || {
            if let Some(spot) = live_spot {
                let short_put_tested  = matches!(strategy,
                    StrategyType::ShortPutVertical | StrategyType::IronCondor
                    | StrategyType::IronButterfly  | StrategyType::Strangle
                    | StrategyType::Straddle       | StrategyType::CashSecuredPut)
                    && trade.short_strike > 0.0 && spot < trade.short_strike;
                let short_call_tested = matches!(strategy,
                    StrategyType::ShortCallVertical | StrategyType::IronCondor
                    | StrategyType::IronButterfly   | StrategyType::Strangle
                    | StrategyType::Straddle        | StrategyType::CoveredCall)
                    && trade.short_strike > 0.0 && spot > trade.short_strike;
                short_put_tested || short_call_tested
            } else {
                false
            }
        };

        if is_tested {
            let spot_str = live_spot
                .map(|s| format!(" (spot ${:.0})", s))
                .unwrap_or_default();
            alerts.push(TradeAlert {
                trade_id: trade.id,
                ticker:   trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind:     AlertKind::Defense,
                severity: AlertSeverity::Critical,
                headline: format!("${} {} — Tested{}. Short strike ${:.0}",
                    trade.ticker, badge, spot_str, trade.short_strike),
                detail: Some("Roll down/out for credit or buy back to cut loss.".to_string()),
            });
            continue; // Defense is exclusive — skip lesser alerts for this trade
        }

        // ── DEFENSE: Expiration today ─────────────────────────────────────────
        if dte == 0 {
            alerts.push(TradeAlert {
                trade_id: trade.id,
                ticker:   trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind:     AlertKind::Defense,
                severity: AlertSeverity::Critical,
                headline: format!("${} {} — EXPIRES TODAY. Close immediately.", trade.ticker, badge),
                detail: Some("Do not let short options expire ITM. Close now.".to_string()),
            });
            continue;
        }

        // ── WARNING: Earnings within 48h with short premium ──────────────────
        if is_short_premium(strategy) {
            if let Some(earnings) = trade.next_earnings {
                let days_to_earnings = (earnings - today).num_days();
                if days_to_earnings >= 0 && days_to_earnings <= 2 {
                    alerts.push(TradeAlert {
                        trade_id: trade.id,
                        ticker:   trade.ticker.clone(),
                        strategy_badge: badge.clone(),
                        kind:     AlertKind::Warning,
                        severity: AlertSeverity::High,
                        headline: format!("${} {} — Earnings in {}d while short premium. CLOSE before report.",
                            trade.ticker, badge, days_to_earnings),
                        detail: Some("Never hold short premium through earnings. The IV crush doesn't offset gap risk.".to_string()),
                    });
                    continue;
                }
                // Earnings 3-5 days out — early warning
                if days_to_earnings <= 5 {
                    alerts.push(TradeAlert {
                        trade_id: trade.id,
                        ticker:   trade.ticker.clone(),
                        strategy_badge: badge.clone(),
                        kind:     AlertKind::Warning,
                        severity: AlertSeverity::High,
                        headline: format!("${} {} — Earnings in {}d. Plan your exit.",
                            trade.ticker, badge, days_to_earnings),
                        detail: Some("Close or roll before earnings if you're short premium.".to_string()),
                    });
                }
            }
        }

        // ── CLOSE: strategy-specific profit target (tastytrade rules) ────────
        let target_pct = trade.target_profit_pct.unwrap_or_else(|| trade.strategy.default_profit_target_pct());
        let days_held = (today - trade.trade_date.date_naive()).num_days().max(0);
        let time_elapsed_pct = if entry_dte > 0 {
            (days_held as f64 / entry_dte as f64) * 100.0
        } else {
            0.0
        };

        // GTC close price hint
        let gtc_price = (trade.credit_received * (1.0 - target_pct / 100.0) * 100.0).round() / 100.0;

        if time_elapsed_pct >= target_pct && is_short_premium(strategy) {
            alerts.push(TradeAlert {
                trade_id: trade.id,
                ticker:   trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind:     AlertKind::Close,
                severity: AlertSeverity::Low,
                headline: format!("${} {} — ~{:.0}% time elapsed (target {:.0}% profit). Close at ${:.2}/contract.",
                    trade.ticker, badge, time_elapsed_pct, target_pct, gtc_price),
                detail: Some(format!("GTC close order: debit ≤ ${:.2}. Don't wait for max profit.", gtc_price)),
            });
            continue;
        }

        // ── MANAGE: 21 DTE ───────────────────────────────────────────────────
        if dte <= 21 && dte > 0 {
            let mgmt = trade.management_rule.as_deref().unwrap_or("Roll or close");
            let (kind, detail) = if trade.credit_received > 0.0 {
                (AlertKind::Roll,
                 format!("Roll out 30-45 days for additional credit. Rule: {}", mgmt))
            } else {
                (AlertKind::Manage,
                 format!("Evaluate: close, roll, or hold. Rule: {}", mgmt))
            };
            let severity = if kind == AlertKind::Roll { AlertSeverity::Info } else { AlertSeverity::Medium };
            alerts.push(TradeAlert {
                trade_id: trade.id,
                ticker:   trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind,
                severity,
                headline: format!("${} {} — {} DTE. Management trigger.", trade.ticker, badge, dte),
                detail: Some(detail),
            });
            continue;
        }

        // ── SIZING: BPR > 5% of account ──────────────────────────────────────
        if let Some(bpr) = trade.bpr {
            let bpr_total = bpr * trade.quantity as f64;
            let pct = bpr_total / account_size * 100.0;
            if pct > 5.0 {
                alerts.push(TradeAlert {
                    trade_id: trade.id,
                    ticker:   trade.ticker.clone(),
                    strategy_badge: badge.clone(),
                    kind:     AlertKind::Sizing,
                    severity: AlertSeverity::Info,
                    headline: format!("${} {} — BPR ${:.0} is {:.1}% of account (>5%).",
                        trade.ticker, badge, bpr_total, pct),
                    detail: Some("Consider reducing size or hedging to maintain portfolio diversity.".to_string()),
                });
                continue;
            }
        }
    }

    // Sort by severity (Critical=0 first), then ticker alphabetically
    alerts.sort_by(|a, b| {
        a.severity.cmp(&b.severity).then_with(|| a.ticker.cmp(&b.ticker))
    });

    // VIX header advisory (global, not per-trade)
    if let Some(vix) = current_vix {
        if vix < 15.0 {
            alerts.insert(0, TradeAlert {
                trade_id: -1,
                ticker:   "VIX".to_string(),
                strategy_badge: "—".to_string(),
                kind:     AlertKind::Sizing,
                severity: AlertSeverity::Info,
                headline: format!("VIX {:.1} — Low volatility. Trade smaller size, prefer defined-risk.", vix),
                detail: Some("Low VIX = cheap options. Prefer defined-risk spreads. Expect mean reversion.".to_string()),
            });
        } else if vix > 30.0 {
            alerts.insert(0, TradeAlert {
                trade_id: -1,
                ticker:   "VIX".to_string(),
                strategy_badge: "—".to_string(),
                kind:     AlertKind::Warning,
                severity: AlertSeverity::High,
                headline: format!("VIX {:.1} — Elevated fear. Sell premium but reduce size 50%.", vix),
                detail: Some("High VIX = rich premium but wide moves. Keep size < 2% BPR per trade.".to_string()),
            });
        }
    }

    if alerts.is_empty() {
        alerts.push(TradeAlert {
            trade_id: -1,
            ticker:   "—".to_string(),
            strategy_badge: "—".to_string(),
            kind:     AlertKind::Ok,
            severity: AlertSeverity::Ok,
            headline: "All clear — no actions required today.".to_string(),
            detail:   None,
        });
    }

    alerts
}
