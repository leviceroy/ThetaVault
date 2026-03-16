use crate::models::{Trade, StrategyType};
use crate::calculations::{calculate_remaining_dte, calculate_payoff_at_price};
use std::collections::{HashMap, HashSet};
use chrono::Utc;

// ─────────────────────────────────────────────────────────────────────────────
// Severity & Kind
// ─────────────────────────────────────────────────────────────────────────────

/// Urgency level — lower number sorts first (most urgent).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Critical = 0,  // DEFENSE / expiration today
    High     = 1,  // WARNING / earnings imminent
    Medium   = 2,  // MANAGE  / 21 DTE / IVR crush
    Low      = 3,  // CLOSE   / profit target
    Info     = 4,  // ROLL / SIZING advisory
    Ok       = 5,  // No action needed
}

/// The action category shown to the user.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlertKind {
    Defense,        // Tested / expires today
    MaxLoss,        // Down 2× max profit — tastytrade hard stop
    GammaRisk,      // Short gamma blowup risk in final week (DTE ≤ 7)
    Warning,        // Earnings risk while short premium
    DeltaExtreme,   // Portfolio BWD > ±500 — heavy directional bias
    UndefinedDrift, // Undefined risk % drifted from target
    Drawdown,       // Account down X% from peak — circuit breaker
    RollChain,      // Rolled 3+ times — consider closing
    Manage,         // 21 DTE management / IVR crush
    Close,          // Profit target hit
    Roll,           // Roll for credit at 21 DTE
    Sizing,         // BPR oversized (per ticker or sector)
    Ok,             // All clear
}

impl AlertKind {
    pub fn badge(&self) -> &'static str {
        match self {
            AlertKind::Defense        => "DEFENSE",
            AlertKind::MaxLoss        => "MAXLOSS",
            AlertKind::GammaRisk      => "GAMMA  ",
            AlertKind::Warning        => "WARNING",
            AlertKind::DeltaExtreme   => "ΔEXTREM",
            AlertKind::UndefinedDrift => "DRIFT  ",
            AlertKind::Drawdown       => "DRAWDWN",
            AlertKind::RollChain      => "ROLLS  ",
            AlertKind::Manage         => "MANAGE ",
            AlertKind::Close          => "CLOSE  ",
            AlertKind::Roll           => "ROLL   ",
            AlertKind::Sizing         => "SIZING ",
            AlertKind::Ok             => "OK     ",
        }
    }

    /// Canonical display order for grouping.
    pub fn order(&self) -> u8 {
        match self {
            AlertKind::Defense        => 0,
            AlertKind::MaxLoss        => 1,
            AlertKind::GammaRisk      => 2,
            AlertKind::Warning        => 3,
            AlertKind::DeltaExtreme   => 4,
            AlertKind::UndefinedDrift => 5,
            AlertKind::Drawdown       => 6,
            AlertKind::RollChain      => 7,
            AlertKind::Manage         => 8,
            AlertKind::Close          => 9,
            AlertKind::Roll           => 10,
            AlertKind::Sizing         => 11,
            AlertKind::Ok             => 12,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Alert & Row types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TradeAlert {
    pub trade_id:       i32,
    pub ticker:         String,
    pub strategy_badge: String,
    pub kind:           AlertKind,
    pub severity:       AlertSeverity,
    pub headline:       String,
    pub detail:         Option<String>,
}

/// A row in the collapsible Actions list — either a group header or an alert.
#[derive(Debug, Clone)]
pub enum ActionRow {
    GroupHeader { kind: AlertKind, count: usize },
    Alert(TradeAlert),
}

/// Build the flat list of visible rows for the Actions tab, respecting collapse state.
/// Groups are emitted in canonical kind order; collapsed groups show only the header.
pub fn build_action_rows(
    alerts:    &[TradeAlert],
    collapsed: &HashSet<AlertKind>,
) -> Vec<ActionRow> {
    // Canonical kind order
    let order = [
        AlertKind::Defense,
        AlertKind::MaxLoss,
        AlertKind::GammaRisk,
        AlertKind::Warning,
        AlertKind::DeltaExtreme,
        AlertKind::UndefinedDrift,
        AlertKind::Drawdown,
        AlertKind::RollChain,
        AlertKind::Manage,
        AlertKind::Close,
        AlertKind::Roll,
        AlertKind::Sizing,
        AlertKind::Ok,
    ];

    let mut rows = Vec::new();
    for kind in &order {
        let group: Vec<&TradeAlert> = alerts.iter().filter(|a| &a.kind == kind).collect();
        if group.is_empty() {
            continue;
        }
        rows.push(ActionRow::GroupHeader { kind: kind.clone(), count: group.len() });
        if !collapsed.contains(kind) {
            for alert in group {
                rows.push(ActionRow::Alert(alert.clone()));
            }
        }
    }
    rows
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy helpers
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// Sector map (hardcoded; covers common option-active underlyings)
// Unknown tickers → "Other" which is excluded from concentration alerts.
// ─────────────────────────────────────────────────────────────────────────────

fn get_sector(ticker: &str) -> &'static str {
    match ticker {
        // Semiconductors (The "One Trade" Group)
        "NVDA" | "AMD" | "SMCI" | "INTC" | "MU" | "AVGO" | "QCOM"
        | "TXN" | "AMAT" | "LRCX" | "KLAC" | "MRVL" | "ADI" | "TSM" => "Semis",
        // Technology (Software/Services)
        "AAPL" | "MSFT" | "GOOG" | "GOOGL" | "META"
        | "CRM" | "ORCL" | "IBM" | "SNOW" | "PLTR" | "SYM" 
        | "DELL" | "HPQ" => "Tech",
        // Consumer Discretionary
        "AMZN" | "TSLA" | "TGT" | "HD" | "LOW" | "NKE" | "SBUX"
        | "MCD" | "CMG" | "BKNG" | "MAR" | "HLT" | "YUM" | "DPZ" => "Consumer Disc",
        // Financials
        "XLF" | "GS" | "JPM" | "BAC" | "MS" | "WFC" | "C"
        | "BRK" | "V" | "MA" | "AXP" | "BLK" | "SCHW" | "COF" => "Financials",
        // Energy
        "OXY" | "CVX" | "XOM" | "XLE" | "SLB" | "COP"
        | "HAL" | "MPC" | "PSX" | "VLO" | "DVN" => "Energy",
        // Healthcare
        "UNH" | "JNJ" | "MRK" | "PFE" | "ABBV" | "LLY" | "BMY"
        | "MDT" | "TMO" | "ABT" | "DHR" | "ISRG" | "AMGN" | "GILD" => "Healthcare",
        // Broad Index ETFs — diversified, no concentration alert
        "SPY" | "QQQ" | "IWM" | "DIA" | "VTI" | "VNQ" | "EEM" => "Index ETF",
        // Commodities
        "GLD" | "SLV" | "GDX" | "USO" | "UNG" | "GDXJ" | "SIL" => "Commodities",
        // Utilities
        "NEE" | "DUK" | "XLU" | "SO" | "D" | "EXC" | "AEP" => "Utilities",
        // Industrials
        "BA" | "CAT" | "GE" | "RTX" | "LMT" | "XLI" | "HON"
        | "UPS" | "FDX" | "DE" | "EMR" | "NOC" => "Industrials",
        // Consumer Staples
        "PG" | "KO" | "PEP" | "WMT" | "COST" | "XLP"
        | "CL" | "GIS" | "K" | "KHC" | "MO" | "PM" => "Consumer Staples",
        // Communication Services
        "T" | "VZ" | "CMCSA" | "NFLX" | "DIS" | "PARA" | "WBD" => "Comm Services",
        // Clean Energy
        "BE" | "ENPH" | "FSLR" | "PLUG" | "SEDG" | "NOVA" => "Clean Energy",
        // REITs
        "O" | "SPG" | "PLD" | "EQR" | "AMT" | "CCI" | "WELL" => "REITs",
        _ => "Other",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main alert computation
// ─────────────────────────────────────────────────────────────────────────────

/// Compute all alerts for open positions.
/// Sorted by severity (Critical first), then ticker alphabetically.
pub fn compute_alerts(
    trades:                   &[Trade],
    live_prices:              &HashMap<String, f64>,
    account_size:             f64,
    current_vix:              Option<f64>,
    net_bwd:                  f64,
    undefined_drift:          f64,
    target_undef_pct:         f64,
    max_drawdown_pct:         f64,
    drawdown_circuit_breaker: f64,
) -> Vec<TradeAlert> {
    let today = Utc::now().date_naive();
    let mut alerts: Vec<TradeAlert> = Vec::new();

    // ── Pre-compute per-ticker total BPR for sizing alerts ────────────────────
    let mut ticker_bpr: HashMap<String, f64> = HashMap::new();
    for t in trades.iter().filter(|t| t.is_open()) {
        if let Some(bpr) = t.bpr {
            *ticker_bpr.entry(t.ticker.clone()).or_default() += bpr * t.quantity as f64;
        }
    }
    let mut sizing_emitted: HashSet<String> = HashSet::new();

    // ── Pre-compute total open BPR for sector concentration ───────────────────
    let total_open_bpr: f64 = ticker_bpr.values().sum();

    // ── Per-trade alerts ──────────────────────────────────────────────────────
    for trade in trades.iter().filter(|t| t.is_open()) {
        let dte       = calculate_remaining_dte(&trade.expiration_date);
        let live_spot = live_prices.get(&trade.ticker).copied();
        let strategy  = &trade.strategy;
        let badge     = strategy.badge().to_string();
        let entry_dte = trade.entry_dte.unwrap_or(45);

        // ── 1. DEFENSE: Tested position ───────────────────────────────────────
        let short_put_tested = matches!(strategy,
            StrategyType::ShortPutVertical | StrategyType::IronCondor
            | StrategyType::IronButterfly  | StrategyType::Strangle
            | StrategyType::Straddle       | StrategyType::CashSecuredPut)
            && trade.short_strike > 0.0
            && live_spot.map_or(false, |s| s < trade.short_strike);

        // For two-sided strategies (IC, strangle, etc.) the call-side short strike
        // lives in trade.legs — trade.short_strike is PUT-side only.
        let short_call_strike: f64 = match strategy {
            StrategyType::IronCondor | StrategyType::IronButterfly
            | StrategyType::Strangle | StrategyType::Straddle => {
                trade.legs.iter()
                    .find(|l| l.leg_type == crate::models::LegType::ShortCall)
                    .map(|l| l.strike)
                    .unwrap_or(trade.short_strike)
            }
            _ => trade.short_strike,
        };

        let short_call_tested = matches!(strategy,
            StrategyType::ShortCallVertical | StrategyType::IronCondor
            | StrategyType::IronButterfly   | StrategyType::Strangle
            | StrategyType::Straddle        | StrategyType::CoveredCall)
            && short_call_strike > 0.0
            && live_spot.map_or(false, |s| s > short_call_strike);

        let is_tested = trade.is_tested || short_put_tested || short_call_tested;

        if is_tested {
            if let Some(spot) = live_spot {
                let side          = if short_put_tested { "put" } else { "call" };
                let rel           = if short_put_tested { "<" }    else { ">" };
                let tested_strike = if short_put_tested { trade.short_strike } else { short_call_strike };
                let distance_pct  = (spot - tested_strike).abs() / tested_strike * 100.0;
                let zone = if distance_pct > 5.0 { "CRITICAL" }
                           else if distance_pct > 2.0 { "DANGER" }
                           else { "WARNING zone" };
                alerts.push(TradeAlert {
                    trade_id:       trade.id,
                    ticker:         trade.ticker.clone(),
                    strategy_badge: badge.clone(),
                    kind:           AlertKind::Defense,
                    severity:       AlertSeverity::Critical,
                    headline: format!(
                        "${} {} {} is tested (Spot ${:.0} {} Strike ${:.0}). {}: {:.1}% ITM.",
                        trade.ticker, badge, side,
                        spot, rel, tested_strike,
                        zone, distance_pct
                    ),
                    detail: Some("Roll down/out for credit or buy back to cut loss.".to_string()),
                });
            } else {
                // No live price — use stored flag
                alerts.push(TradeAlert {
                    trade_id:       trade.id,
                    ticker:         trade.ticker.clone(),
                    strategy_badge: badge.clone(),
                    kind:           AlertKind::Defense,
                    severity:       AlertSeverity::Critical,
                    headline: format!(
                        "${} {} is tested. Short strike ${:.0}.",
                        trade.ticker, badge, trade.short_strike
                    ),
                    detail: Some("Roll down/out for credit or buy back to cut loss.".to_string()),
                });
            }
            continue; // Defense is exclusive — skip lesser alerts for this trade
        }

        // ── 2. DEFENSE: Expiration today ──────────────────────────────────────
        if dte == 0 {
            alerts.push(TradeAlert {
                trade_id:       trade.id,
                ticker:         trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind:           AlertKind::Defense,
                severity:       AlertSeverity::Critical,
                headline: format!("${} {} expires TODAY. Close immediately.", trade.ticker, badge),
                detail: Some("Do not let short options expire ITM. Close now.".to_string()),
            });
            continue;
        }

        // ── 3. MAXLOSS: Down 2× max profit (tastytrade hard stop) ────────────
        if is_short_premium(strategy) && trade.credit_received > 0.0 && !trade.legs.is_empty() {
            if let Some(spot) = live_spot {
                let payoff = calculate_payoff_at_price(&trade.legs, trade.credit_received, spot);
                let two_x_loss = -(2.0 * trade.credit_received * 100.0);
                if payoff < two_x_loss {
                    let loss_dollars = -payoff * trade.quantity as f64;
                    alerts.push(TradeAlert {
                        trade_id:       trade.id,
                        ticker:         trade.ticker.clone(),
                        strategy_badge: badge.clone(),
                        kind:           AlertKind::MaxLoss,
                        severity:       AlertSeverity::High,
                        headline: format!(
                            "${} {} is down ~${:.0} — exceeded 2× max profit. CLOSE NOW.",
                            trade.ticker, badge, loss_dollars
                        ),
                        detail: Some(
                            "Tastytrade hard stop: never let a position exceed 2× the initial credit. Cut the loss.".to_string()
                        ),
                    });
                    continue;
                }
            }
        }

        // ── 4. GAMMA RISK: Final-week short gamma exposure ───────────────────
        if dte <= 7 && dte > 0 && is_short_premium(strategy) {
            if let Some(g) = trade.gamma {
                if g.abs() > 0.02 {
                    alerts.push(TradeAlert {
                        trade_id:       trade.id,
                        ticker:         trade.ticker.clone(),
                        strategy_badge: badge.clone(),
                        kind:           AlertKind::GammaRisk,
                        severity:       AlertSeverity::High,
                        headline: format!(
                            "${} {} — {} DTE, gamma {:.3}. Short gamma risk accelerating.",
                            trade.ticker, badge, dte, g.abs()
                        ),
                        detail: Some(
                            "Close before expiration weekend. Gamma blowups happen fast in the final week.".to_string()
                        ),
                    });
                    continue;
                }
            } else {
                // No gamma stored — fire a general final-week warning
                alerts.push(TradeAlert {
                    trade_id:       trade.id,
                    ticker:         trade.ticker.clone(),
                    strategy_badge: badge.clone(),
                    kind:           AlertKind::GammaRisk,
                    severity:       AlertSeverity::High,
                    headline: format!(
                        "${} {} — {} DTE. Approaching expiration, gamma risk elevated.",
                        trade.ticker, badge, dte
                    ),
                    detail: Some(
                        "Under 7 DTE: gamma accelerates rapidly. Close or let expire if well OTM.".to_string()
                    ),
                });
                continue;
            }
        }

        // ── 5. WARNING: Earnings within 48h with short premium ────────────────
        if is_short_premium(strategy) {
            if let Some(earnings) = trade.next_earnings {
                let days_to_earnings = (earnings - today).num_days();
                // Only fire for future or same-day earnings (never negative days)
                if days_to_earnings >= 0 && days_to_earnings <= 2 {
                    let hours = days_to_earnings * 24;
                    let time_str = if days_to_earnings == 0 {
                        "today".to_string()
                    } else {
                        format!("in ~{}h", hours)
                    };
                    alerts.push(TradeAlert {
                        trade_id:       trade.id,
                        ticker:         trade.ticker.clone(),
                        strategy_badge: badge.clone(),
                        kind:           AlertKind::Warning,
                        severity:       AlertSeverity::High,
                        headline: format!(
                            "${} {} has earnings {} (you're short premium). CLOSE before report.",
                            trade.ticker, badge, time_str
                        ),
                        detail: Some("Never hold short premium through earnings. IV crush ≠ gap risk.".to_string()),
                    });
                    continue;
                }
                // Earnings 3-5 days out — early planning alert (guard: >= 0 to skip past earnings)
                if days_to_earnings >= 3 && days_to_earnings <= 5 {
                    alerts.push(TradeAlert {
                        trade_id:       trade.id,
                        ticker:         trade.ticker.clone(),
                        strategy_badge: badge.clone(),
                        kind:           AlertKind::Warning,
                        severity:       AlertSeverity::High,
                        headline: format!(
                            "${} {} — earnings in {}d. Plan your exit.",
                            trade.ticker, badge, days_to_earnings
                        ),
                        detail: Some("Close or roll before earnings if you're short premium.".to_string()),
                    });
                }
            }
        }

        // ── Shared: days held + estimated % of max profit ─────────────────────
        let days_held = (today - trade.trade_date.date_naive()).num_days().max(0);
        let est_pct_max = if entry_dte > 0 {
            let days_elapsed = (entry_dte as i64 - dte).max(0);
            (days_elapsed as f64 / entry_dte as f64) * 100.0
        } else {
            0.0
        };

        // ── 4. CLOSE: Profit target reached ──────────────────────────────────
        let target_pct = trade.target_profit_pct
            .unwrap_or_else(|| trade.strategy.default_profit_target_pct());
        let gtc_price = (trade.credit_received * (1.0 - target_pct / 100.0) * 100.0)
            .round() / 100.0;

        if est_pct_max >= target_pct && is_short_premium(strategy) && trade.credit_received > 0.0 {
            alerts.push(TradeAlert {
                trade_id:       trade.id,
                ticker:         trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind:           AlertKind::Close,
                severity:       AlertSeverity::Low,
                headline: format!(
                    "${} {} is at est. {:.0}% of max profit.",
                    trade.ticker, badge, est_pct_max
                ),
                detail: Some(format!(
                    "Target is {:.0}%. GTC close ≤ ${:.2}/contract.",
                    target_pct, gtc_price
                )),
            });
            continue;
        }

        // ── 5. IVR Crush: high-IV entry stalling after 14 days ───────────────
        if let Some(ivr) = trade.iv_rank {
            if ivr >= 25.0 && days_held >= 14 && est_pct_max < 25.0 && is_short_premium(strategy) {
                alerts.push(TradeAlert {
                    trade_id:       trade.id,
                    ticker:         trade.ticker.clone(),
                    strategy_badge: badge.clone(),
                    kind:           AlertKind::Manage,
                    severity:       AlertSeverity::Medium,
                    headline: format!(
                        "${} {} — IV crush stall: entered IVR {:.0}, est. {:.0}% of max after {} days.",
                        trade.ticker, badge, ivr, est_pct_max, days_held
                    ),
                    detail: Some(
                        "IV crushed but PnL not materializing — price likely moved against you. Evaluate closing.".to_string()
                    ),
                });
                continue;
            }
        }

        // ── 6. MANAGE: 21 DTE ────────────────────────────────────────────────
        if dte <= 21 && dte > 0 {
            let mgmt = trade.management_rule.as_deref().unwrap_or("Roll or close");
            let (kind, detail) = if trade.credit_received > 0.0 {
                (AlertKind::Roll,
                 format!("Roll for credit 30-45 days out. Rule: {}", mgmt))
            } else {
                (AlertKind::Manage,
                 format!("Evaluate: close, roll, or hold. Rule: {}", mgmt))
            };
            let severity = if kind == AlertKind::Roll { AlertSeverity::Info } else { AlertSeverity::Medium };
            alerts.push(TradeAlert {
                trade_id:       trade.id,
                ticker:         trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind,
                severity,
                headline: format!("${} {} is at {} DTE.", trade.ticker, badge, dte),
                detail:   Some(detail),
            });
            continue;
        }

        // ── 7. SIZING: Per-ticker BPR > 10% of account ───────────────────────
        if !sizing_emitted.contains(&trade.ticker) {
            if let Some(&total_bpr) = ticker_bpr.get(&trade.ticker) {
                let pct = total_bpr / account_size * 100.0;
                if pct > 10.0 {
                    sizing_emitted.insert(trade.ticker.clone());
                    alerts.push(TradeAlert {
                        trade_id:       trade.id,
                        ticker:         trade.ticker.clone(),
                        strategy_badge: "—".to_string(),
                        kind:           AlertKind::Sizing,
                        severity:       AlertSeverity::Info,
                        headline: format!(
                            "${} total position is {:.1}% of account (${:.0} BPR). Oversized.",
                            trade.ticker, pct, total_bpr
                        ),
                        detail: Some(
                            "Stay under 10% single-name BPR to avoid concentrated losses.".to_string()
                        ),
                    });
                }
            }
        }

        // ── 8. ROLL CHAIN: Rolled 3+ times ───────────────────────────────────
        if trade.roll_count >= 3 {
            alerts.push(TradeAlert {
                trade_id:       trade.id,
                ticker:         trade.ticker.clone(),
                strategy_badge: badge.clone(),
                kind:           AlertKind::RollChain,
                severity:       AlertSeverity::Medium,
                headline: format!(
                    "${} {} has been rolled {} times. Evaluate closing vs rolling again.",
                    trade.ticker, badge, trade.roll_count
                ),
                detail: Some(
                    "Rolling repeatedly can chase a loser. Assess whether the thesis still holds \
                     or take the loss and redeploy capital.".to_string()
                ),
            });
        }
    }

    // ── Sector concentration (portfolio-level) ────────────────────────────────
    if total_open_bpr > 0.0 {
        let mut sector_bpr: HashMap<&str, f64> = HashMap::new();
        for t in trades.iter().filter(|t| t.is_open()) {
            if let Some(bpr) = t.bpr {
                *sector_bpr.entry(get_sector(&t.ticker)).or_default() += bpr * t.quantity as f64;
            }
        }
        for (sector, sector_total) in &sector_bpr {
            // Skip broad index ETFs — they don't represent single-name concentration risk
            if *sector == "Index ETF" {
                continue;
            }
            let pct = sector_total / total_open_bpr * 100.0;
            // The "Oh Sh*t" Filter: >25% in a correlated group
            if pct > 25.0 {
                alerts.push(TradeAlert {
                    trade_id:       -1,
                    ticker:         format!("SECTOR:{}", sector),
                    strategy_badge: "—".to_string(),
                    kind:           AlertKind::Sizing,
                    severity:       AlertSeverity::High,
                    headline: format!(
                        "CORRELATION ALERT: {} sector at {:.0}% of BPR.",
                        sector, pct
                    ),
                    detail: Some(
                        "Correlated tickers (e.g. NVDA, AMD) act as one large trade. Diversify to protect the book.".to_string()
                    ),
                });
            }
        }
    }

    // ── Portfolio BWD extreme ─────────────────────────────────────────────────
    if net_bwd.abs() > 500.0 {
        let dir = if net_bwd > 0.0 { "bullish" } else { "bearish" };
        alerts.push(TradeAlert {
            trade_id:       -1,
            ticker:         "PORTFOLIO".to_string(),
            strategy_badge: "—".to_string(),
            kind:           AlertKind::DeltaExtreme,
            severity:       AlertSeverity::High,
            headline: format!(
                "Net BWD {:+.0} — heavy {} directional bias. Rebalance.",
                net_bwd, dir
            ),
            detail: Some(
                "Beta-weighted delta > ±500 means your book has material directional risk. \
                 Consider adding the opposite side or reducing size.".to_string()
            ),
        });
    }

    // ── Undefined risk drift ──────────────────────────────────────────────────
    if undefined_drift > 10.0 || undefined_drift < -15.0 {
        let actual = target_undef_pct + undefined_drift;
        let dir = if undefined_drift > 0.0 { "above" } else { "below" };
        alerts.push(TradeAlert {
            trade_id:       -1,
            ticker:         "PORTFOLIO".to_string(),
            strategy_badge: "—".to_string(),
            kind:           AlertKind::UndefinedDrift,
            severity:       AlertSeverity::Medium,
            headline: format!(
                "Undefined risk at {:.0}% vs target {:.0}% — {dir} by {:.0}%. Rebalance.",
                actual, target_undef_pct, undefined_drift.abs()
            ),
            detail: Some(
                "Add defined-risk spreads to bring undefined exposure back to target, \
                 or close naked positions.".to_string()
            ),
        });
    }

    // ── Drawdown circuit breaker ─────────────────────────────────────────────
    let cb_threshold = if drawdown_circuit_breaker > 0.0 { drawdown_circuit_breaker } else { 5.0 };
    if max_drawdown_pct > cb_threshold {
        alerts.push(TradeAlert {
            trade_id:       -1,
            ticker:         "PORTFOLIO".to_string(),
            strategy_badge: "—".to_string(),
            kind:           AlertKind::Drawdown,
            severity:       AlertSeverity::High,
            headline: format!(
                "Account down {:.1}% from peak. Reduce size and reassess.",
                max_drawdown_pct
            ),
            detail: Some(format!(
                "Drawdown circuit breaker: >{:.0}% from peak. Cut position size by 50%, \
                 close losers, and wait for high-probability setups.",
                cb_threshold
            )),
        });
    }

    // ── Sort: severity first, then ticker alphabetically ─────────────────────
    alerts.sort_by(|a, b| {
        a.severity.cmp(&b.severity).then_with(|| a.ticker.cmp(&b.ticker))
    });

    // ── VIX global advisory (inserted at front) ───────────────────────────────
    if let Some(vix) = current_vix {
        if vix < 15.0 {
            alerts.insert(0, TradeAlert {
                trade_id:       -1,
                ticker:         "VIX".to_string(),
                strategy_badge: "—".to_string(),
                kind:           AlertKind::Sizing,
                severity:       AlertSeverity::Info,
                headline: format!(
                    "VIX {:.1} — low volatility. Prefer defined-risk, trade smaller.", vix
                ),
                detail: Some(
                    "Low VIX = cheap options. Spreads > naked. Expect mean reversion.".to_string()
                ),
            });
        } else if vix > 30.0 {
            alerts.insert(0, TradeAlert {
                trade_id:       -1,
                ticker:         "VIX".to_string(),
                strategy_badge: "—".to_string(),
                kind:           AlertKind::Warning,
                severity:       AlertSeverity::High,
                headline: format!(
                    "VIX {:.1} — elevated fear. Sell premium but cut size 50%.", vix
                ),
                detail: Some(
                    "High VIX = rich premium but wide moves. Keep BPR < 2% per trade.".to_string()
                ),
            });
        }
    }

    // ── All-clear fallback ────────────────────────────────────────────────────
    if alerts.is_empty() {
        alerts.push(TradeAlert {
            trade_id:       -1,
            ticker:         "—".to_string(),
            strategy_badge: "—".to_string(),
            kind:           AlertKind::Ok,
            severity:       AlertSeverity::Ok,
            headline:       "All clear — no actions required today.".to_string(),
            detail:         None,
        });
    }

    alerts
}
