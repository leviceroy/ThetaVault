// Copyright (c) 2025 Chris Wenk. All rights reserved.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};

// ──────────────────────────────────────────────────────────────────────────────
// Strategy Types  (match OptionsTradingJournal exactly — snake_case strings)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StrategyType {
    ShortPutVertical,
    ShortCallVertical,
    IronCondor,
    IronButterfly,
    Strangle,
    Straddle,
    CalendarSpread,
    CashSecuredPut,
    CoveredCall,
    Pmcc,
    LongDiagonalSpread,
    ShortDiagonalSpread,
    LongCallVertical,
    LongPutVertical,
    PutZebra,
    CallZebra,
    Custom,
    PutBrokenWingButterfly,
    CallBrokenWingButterfly,
    JadeLizard,
    PutButterfly,
    CallButterfly,
}

impl StrategyType {
    /// Human-readable label matching OTJ STRATEGY_LABELS
    pub fn label(&self) -> &'static str {
        match self {
            StrategyType::ShortPutVertical    => "Short Put Vertical",
            StrategyType::ShortCallVertical   => "Short Call Vertical",
            StrategyType::IronCondor          => "Iron Condor",
            StrategyType::IronButterfly       => "Iron Butterfly",
            StrategyType::Strangle            => "Strangle",
            StrategyType::Straddle            => "Straddle",
            StrategyType::CalendarSpread      => "Calendar Spread",
            StrategyType::CashSecuredPut      => "Cash Secured Put",
            StrategyType::CoveredCall         => "Covered Call",
            StrategyType::Pmcc                => "Poor Man's Covered Call",
            StrategyType::LongDiagonalSpread  => "Long Diagonal Spread",
            StrategyType::ShortDiagonalSpread => "Short Diagonal Spread",
            StrategyType::LongCallVertical    => "Long Call Vertical",
            StrategyType::LongPutVertical     => "Long Put Vertical",
            StrategyType::PutZebra            => "Put ZEBRA",
            StrategyType::CallZebra           => "Call ZEBRA",
            StrategyType::Custom              => "Custom / Ratio Spread",
            StrategyType::PutBrokenWingButterfly  => "Put Broken Wing Butterfly",
            StrategyType::CallBrokenWingButterfly => "Call Broken Wing Butterfly",
            StrategyType::JadeLizard              => "Jade Lizard",
            StrategyType::PutButterfly            => "Put Butterfly",
            StrategyType::CallButterfly           => "Call Butterfly",
        }
    }

    /// Short badge matching OTJ STRATEGY_BADGES
    pub fn badge(&self) -> &'static str {
        match self {
            StrategyType::ShortPutVertical    => "SPV",
            StrategyType::ShortCallVertical   => "SCV",
            StrategyType::IronCondor          => "IC",
            StrategyType::IronButterfly       => "IB",
            StrategyType::Strangle            => "STR",
            StrategyType::Straddle            => "STD",
            StrategyType::CalendarSpread      => "CAL",
            StrategyType::CashSecuredPut      => "CSP",
            StrategyType::CoveredCall         => "CC",
            StrategyType::Pmcc                => "PMCC",
            StrategyType::LongDiagonalSpread  => "LDS",
            StrategyType::ShortDiagonalSpread => "SDS",
            StrategyType::LongCallVertical    => "LCV",
            StrategyType::LongPutVertical     => "LPV",
            StrategyType::PutZebra            => "PZBR",
            StrategyType::CallZebra           => "CZBR",
            StrategyType::Custom              => "CUST",
            StrategyType::PutBrokenWingButterfly  => "PBWB",
            StrategyType::CallBrokenWingButterfly => "CBWB",
            StrategyType::JadeLizard              => "JL",
            StrategyType::PutButterfly            => "PBF",
            StrategyType::CallButterfly           => "CBF",
        }
    }

    /// Parse from snake_case string (for SQLite reads)
    pub fn from_str(s: &str) -> StrategyType {
        match s {
            "short_put_vertical"    => StrategyType::ShortPutVertical,
            "short_call_vertical"   => StrategyType::ShortCallVertical,
            "iron_condor"           => StrategyType::IronCondor,
            "iron_butterfly"        => StrategyType::IronButterfly,
            "strangle"              => StrategyType::Strangle,
            "straddle"              => StrategyType::Straddle,
            "calendar_spread"       => StrategyType::CalendarSpread,
            "cash_secured_put"      => StrategyType::CashSecuredPut,
            "covered_call"          => StrategyType::CoveredCall,
            "pmcc"                  => StrategyType::Pmcc,
            "long_diagonal_spread"  => StrategyType::LongDiagonalSpread,
            "short_diagonal_spread" => StrategyType::ShortDiagonalSpread,
            "long_call_vertical"    => StrategyType::LongCallVertical,
            "long_put_vertical"     => StrategyType::LongPutVertical,
            // Standalone long legs imported before CSP/CC→Vertical upgrade existed
            "long_put"              => StrategyType::LongPutVertical,
            "long_call"             => StrategyType::LongCallVertical,
            "pzbr"                           => StrategyType::PutZebra,
            "czbr"                           => StrategyType::CallZebra,
            "put_broken_wing_butterfly"       => StrategyType::PutBrokenWingButterfly,
            "call_broken_wing_butterfly"      => StrategyType::CallBrokenWingButterfly,
            "jade_lizard"                     => StrategyType::JadeLizard,
            "put_butterfly"                   => StrategyType::PutButterfly,
            "call_butterfly"                  => StrategyType::CallButterfly,
            _                                 => StrategyType::Custom,
        }
    }

    /// Serialize to snake_case string (for SQLite writes)
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyType::ShortPutVertical    => "short_put_vertical",
            StrategyType::ShortCallVertical   => "short_call_vertical",
            StrategyType::IronCondor          => "iron_condor",
            StrategyType::IronButterfly       => "iron_butterfly",
            StrategyType::Strangle            => "strangle",
            StrategyType::Straddle            => "straddle",
            StrategyType::CalendarSpread      => "calendar_spread",
            StrategyType::CashSecuredPut      => "cash_secured_put",
            StrategyType::CoveredCall         => "covered_call",
            StrategyType::Pmcc                => "pmcc",
            StrategyType::LongDiagonalSpread  => "long_diagonal_spread",
            StrategyType::ShortDiagonalSpread => "short_diagonal_spread",
            StrategyType::LongCallVertical    => "long_call_vertical",
            StrategyType::LongPutVertical     => "long_put_vertical",
            StrategyType::PutZebra            => "pzbr",
            StrategyType::CallZebra           => "czbr",
            StrategyType::Custom              => "custom",
            StrategyType::PutBrokenWingButterfly  => "put_broken_wing_butterfly",
            StrategyType::CallBrokenWingButterfly => "call_broken_wing_butterfly",
            StrategyType::JadeLizard              => "jade_lizard",
            StrategyType::PutButterfly            => "put_butterfly",
            StrategyType::CallButterfly           => "call_butterfly",
        }
    }

    /// Whether this strategy has defined (capped) risk.
    /// Tastylive sizing: defined risk = 0.05-2% of net liq; undefined = 3-7%.
    pub fn is_defined_risk(&self) -> bool {
        matches!(self,
            StrategyType::IronCondor
            | StrategyType::IronButterfly
            | StrategyType::ShortPutVertical
            | StrategyType::ShortCallVertical
            | StrategyType::LongPutVertical
            | StrategyType::LongCallVertical
            | StrategyType::CashSecuredPut
            // CC: risk capped at stock purchase price; tastytrade treats as defined/low-risk
            | StrategyType::CoveredCall
            | StrategyType::CalendarSpread
            | StrategyType::Pmcc
            | StrategyType::LongDiagonalSpread
            | StrategyType::ShortDiagonalSpread
            | StrategyType::PutBrokenWingButterfly
            | StrategyType::CallBrokenWingButterfly
            // ZEBRA = synthetic long/short with defined downside via put spread
            | StrategyType::PutZebra
            | StrategyType::CallZebra
            // Jade Lizard: call side is spread-defined; put side = CSP; overall defined when credit > call width
            | StrategyType::JadeLizard
            // Put/Call Butterfly: debit spread with defined loss = debit paid
            | StrategyType::PutButterfly
            | StrategyType::CallButterfly
        )
    }

    /// Default profit-take target (%) when no per-trade override is set.
    /// Matches Tom Sosnoff / tastytrade rules per strategy type.
    pub fn default_profit_target_pct(&self) -> f64 {
        match self {
            StrategyType::CashSecuredPut | StrategyType::CoveredCall => 85.0,
            StrategyType::CalendarSpread | StrategyType::IronButterfly => 25.0,
            StrategyType::PutButterfly | StrategyType::CallButterfly => 25.0,
            _ => 50.0,
        }
    }

    /// Recommended entry DTE range (min, max) per tastytrade guidelines.
    /// Returns None for strategies where DTE is irrelevant (0DTE handled by (0,0)).
    pub fn recommended_entry_dte(&self) -> Option<(i32, i32)> {
        match self {
            StrategyType::IronCondor | StrategyType::IronButterfly => Some((40, 55)),
            StrategyType::CashSecuredPut | StrategyType::CoveredCall => Some((30, 45)),
            StrategyType::ShortPutVertical | StrategyType::ShortCallVertical => Some((21, 45)),
            StrategyType::Strangle | StrategyType::Straddle => Some((25, 45)),
            StrategyType::CalendarSpread | StrategyType::Pmcc => Some((30, 60)),
            StrategyType::LongCallVertical | StrategyType::LongPutVertical => Some((21, 45)),
            StrategyType::JadeLizard => Some((21, 45)),
            StrategyType::PutBrokenWingButterfly | StrategyType::CallBrokenWingButterfly => Some((30, 45)),
            StrategyType::PutButterfly | StrategyType::CallButterfly => Some((15, 45)),
            _ => None,
        }
    }

    /// Recommended maximum spread width per tastytrade guidelines.
    /// Returns None for strategies where width is not applicable.
    pub fn recommended_max_width(&self) -> Option<f64> {
        match self {
            StrategyType::ShortPutVertical | StrategyType::ShortCallVertical => Some(10.0),
            StrategyType::IronCondor => Some(10.0),   // per-side
            StrategyType::IronButterfly => Some(5.0),
            StrategyType::LongCallVertical | StrategyType::LongPutVertical => Some(10.0),
            _ => None,
        }
    }
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Leg Types  (matching OTJ TradeLeg exactly)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum LegType {
    #[serde(rename = "short_put")]
    ShortPut,
    #[serde(rename = "long_put")]
    LongPut,
    #[serde(rename = "short_call")]
    ShortCall,
    #[serde(rename = "long_call")]
    LongCall,
}

impl LegType {
    pub fn is_short(&self) -> bool {
        matches!(self, LegType::ShortPut | LegType::ShortCall)
    }
    pub fn is_call(&self) -> bool {
        matches!(self, LegType::ShortCall | LegType::LongCall)
    }
    pub fn is_put(&self) -> bool {
        matches!(self, LegType::ShortPut | LegType::LongPut)
    }

    /// Parse from human-readable label (for SELECT field round-trips)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "short_put"  | "Short Put"   => Some(LegType::ShortPut),
            "long_put"   | "Long Put"    => Some(LegType::LongPut),
            "short_call" | "Short Call"  => Some(LegType::ShortCall),
            "long_call"  | "Long Call"   => Some(LegType::LongCall),
            _ => None,
        }
    }

    /// Human-readable label used in SELECT options
    pub fn label(&self) -> &'static str {
        match self {
            LegType::ShortPut  => "Short Put",
            LegType::LongPut   => "Long Put",
            LegType::ShortCall => "Short Call",
            LegType::LongCall  => "Long Call",
        }
    }

    /// All four option labels in the canonical SELECT order
    pub fn all_options() -> Vec<String> {
        vec![
            "Short Put".to_string(),
            "Long Put".to_string(),
            "Short Call".to_string(),
            "Long Call".to_string(),
        ]
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Strategy leg templates  (exact port of OTJ shared/schema.ts legTemplates)
// ──────────────────────────────────────────────────────────────────────────────

/// Returns the canonical leg types for a given strategy (in display order).
/// Custom returns an empty vec — caller manages legs freely.
pub fn strategy_leg_template(strategy: &StrategyType) -> Vec<LegType> {
    match strategy {
        StrategyType::ShortPutVertical    => vec![LegType::ShortPut,  LegType::LongPut],
        StrategyType::ShortCallVertical   => vec![LegType::ShortCall, LegType::LongCall],
        StrategyType::IronCondor          => vec![LegType::ShortPut,  LegType::LongPut,  LegType::ShortCall, LegType::LongCall],
        StrategyType::IronButterfly       => vec![LegType::ShortPut,  LegType::LongPut,  LegType::ShortCall, LegType::LongCall],
        StrategyType::Strangle            => vec![LegType::ShortPut,  LegType::ShortCall],
        StrategyType::Straddle            => vec![LegType::ShortPut,  LegType::ShortCall],
        StrategyType::CalendarSpread      => vec![LegType::ShortCall, LegType::LongCall],
        StrategyType::CashSecuredPut      => vec![LegType::ShortPut],
        StrategyType::CoveredCall         => vec![LegType::ShortCall],
        StrategyType::Pmcc                => vec![LegType::LongCall,  LegType::ShortCall],
        StrategyType::LongDiagonalSpread  => vec![LegType::LongCall,  LegType::ShortCall],
        StrategyType::ShortDiagonalSpread => vec![LegType::ShortCall, LegType::LongCall],
        StrategyType::LongCallVertical    => vec![LegType::LongCall,  LegType::ShortCall],
        StrategyType::LongPutVertical     => vec![LegType::LongPut,   LegType::ShortPut],
        StrategyType::PutZebra            => vec![],  // user builds legs freely
        StrategyType::CallZebra           => vec![],
        StrategyType::Custom              => vec![],
        // ShortPut = ATM anchor short; two LongPuts = ATM long (higher) + wing (lower)
        StrategyType::PutBrokenWingButterfly => vec![LegType::ShortPut, LegType::LongPut, LegType::LongPut],
        // CBWB mirrors PBWB on the call side: LongCall (anchor) + ShortCall + LongCall (outer wing)
        StrategyType::CallBrokenWingButterfly => vec![LegType::LongCall, LegType::ShortCall, LegType::LongCall],
        // Jade Lizard: short put (OTM) + short call (OTM) + long call (higher strike)
        StrategyType::JadeLizard => vec![LegType::ShortPut, LegType::ShortCall, LegType::LongCall],
        // Put Butterfly: symmetrical — upper long put + 2 short puts + lower long put
        StrategyType::PutButterfly => vec![LegType::LongPut, LegType::ShortPut, LegType::LongPut],
        // Call Butterfly: symmetrical — lower long call + 2 short calls + upper long call
        StrategyType::CallButterfly => vec![LegType::LongCall, LegType::ShortCall, LegType::LongCall],
    }
}

/// Merge existing legs into a new strategy template — exact port of OTJ mergeLegsForStrategyChange().
/// Returns (merged_legs, removed_legs).
/// - Custom: keeps all existing legs unchanged.
/// - Standard: preserves matching leg types, creates empty for missing, drops extras.
pub fn merge_legs_for_strategy_change(
    existing_legs: &[TradeLeg],
    new_strategy: &StrategyType,
) -> (Vec<TradeLeg>, Vec<TradeLeg>) {
    if *new_strategy == StrategyType::Custom
        || *new_strategy == StrategyType::PutZebra
        || *new_strategy == StrategyType::CallZebra
    {
        return (existing_legs.to_vec(), vec![]);
    }

    let template = strategy_leg_template(new_strategy);
    let mut merged: Vec<TradeLeg> = Vec::new();
    let mut matched_indices: Vec<usize> = Vec::new();

    for leg_type in &template {
        // Prefer first unmatched existing leg of the same type
        let found = existing_legs.iter().enumerate().find(|(idx, l)| {
            &l.leg_type == leg_type && !matched_indices.contains(idx)
        });
        if let Some((idx, leg)) = found {
            matched_indices.push(idx);
            merged.push(leg.clone());
        } else {
            merged.push(TradeLeg {
                leg_type: leg_type.clone(),
                strike: 0.0,
                premium: 0.0,
                close_premium: None,
                expiration_date: None,
                quantity: None,
            });
        }
    }

    let removed: Vec<TradeLeg> = existing_legs.iter().enumerate()
        .filter(|(idx, _)| !matched_indices.contains(idx))
        .map(|(_, l)| l.clone())
        .collect();

    (merged, removed)
}

// ──────────────────────────────────────────────────────────────────────────────
// Trade Leg  (matches OTJ TradeLeg interface exactly)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeLeg {
    #[serde(rename = "type")]
    pub leg_type: LegType,
    pub strike: f64,
    pub premium: f64,
    #[serde(rename = "closePremium")]
    pub close_premium: Option<f64>,
    #[serde(rename = "expirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,  // ISO date string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i32>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Trade  (matches OTJ trades table schema exactly)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub id: i32,
    pub ticker: String,
    pub strategy: StrategyType,        // snake_case string in DB

    pub quantity: i32,

    // Strike prices (for quick access without parsing legs)
    pub short_strike: f64,
    pub long_strike: f64,

    // Premiums per-leg (entry)
    pub short_premium: f64,
    pub long_premium: f64,

    /// Net credit per spread at entry (short_premium - long_premium)
    pub credit_received: f64,

    // Timestamps
    pub entry_date: DateTime<Utc>,
    pub exit_date: Option<DateTime<Utc>>,
    pub expiration_date: DateTime<Utc>,
    pub trade_date: DateTime<Utc>,
    pub back_month_expiration: Option<DateTime<Utc>>,

    // P&L
    pub pnl: Option<f64>,
    pub debit_paid: Option<f64>,        // net debit to close (positive = paid to close)

    // Greeks at entry
    pub delta: Option<f64>,
    pub theta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub pop: Option<f64>,               // probability of profit 0-100

    // Underlying / IV data
    pub underlying_price: Option<f64>,          // at entry
    pub underlying_price_at_close: Option<f64>, // at exit
    pub iv_rank: Option<f64>,
    pub iv_percentile: Option<f64>,  // IV Percentile (0-100) — different from IV Rank; TOS shows both
    pub vix_at_entry: Option<f64>,
    pub implied_volatility: Option<f64>,

    // Trade metadata
    pub commission: Option<f64>,
    pub entry_reason: Option<String>,
    pub exit_reason: Option<String>,
    pub management_rule: Option<String>,
    pub target_profit_pct: Option<f64>,

    // Computed fields
    pub spread_width: Option<f64>,
    pub bpr: Option<f64>,               // Buying Power Reduction
    pub sector: Option<String>,         // GICS sector from Yahoo Finance
    pub entry_dte: Option<i32>,         // DTE at entry
    pub dte_at_close: Option<i32>,      // DTE at close

    // Close-side Greeks (captured when closing the trade)
    pub iv_at_close: Option<f64>,       // IV% at close — measures IV crush contribution
    pub delta_at_close: Option<f64>,    // Delta at close — directional exposure at exit
    pub theta_at_close: Option<f64>,    // Theta at close
    pub gamma_at_close: Option<f64>,    // Gamma at close
    pub vega_at_close: Option<f64>,     // Vega at close

    // Ex-dividend date (for CC early assignment risk alert)
    pub ex_dividend_date: Option<NaiveDate>,

    // Roll tracking
    pub roll_count: i32,                // Number of times this position has been rolled

    // Relationships
    pub playbook_id: Option<i32>,
    pub rolled_from_id: Option<i32>,

    // Status flags
    pub is_earnings_play: bool,
    pub is_tested: bool,

    // Earnings date (next expected earnings for the underlying)
    pub next_earnings: Option<NaiveDate>,

    // Grading
    pub trade_grade: Option<String>,    // A, B, C, D, F
    pub grade_notes: Option<String>,

    // Structured legs (JSON array in DB)
    pub legs: Vec<TradeLeg>,

    // Tags
    pub tags: Vec<String>,

    // Notes
    pub notes: Option<String>,

    // Execution quality (ISC-27, ISC-28)
    pub bid_ask_spread_at_entry: Option<f64>,  // bid-ask width at time of entry
    pub fill_vs_mid: Option<f64>,              // fill price minus mid-market (negative = paid through mid)

    // Assignment tracking (ISC-29)
    pub was_assigned: bool,                    // true if option was assigned to/exercised against stock
    pub assigned_shares: Option<i32>,          // number of shares assigned (100 per contract)
    pub cost_basis: Option<f64>,               // cost basis per share on assigned stock
    pub close_notes: Option<String>,           // exit thesis / close notes

    // M5: profit target tracking
    pub closed_at_target: bool,               // true if closed at or beyond profit target
}

impl Trade {
    /// True if the trade is still open
    pub fn is_open(&self) -> bool {
        self.exit_date.is_none() && self.pnl.is_none()
    }

    /// Spread type string (snake_case)
    pub fn spread_type(&self) -> &'static str {
        self.strategy.as_str()
    }

    /// True if this was a 0DTE trade (entered on expiration day)
    pub fn is_0dte(&self) -> bool {
        self.entry_dte == Some(0)
            || self.trade_date.date_naive() == self.expiration_date.date_naive()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Playbook Strategy Entry Criteria
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntryCriteria {
    #[serde(rename = "minIVR")]
    pub min_ivr: Option<f64>,
    #[serde(rename = "maxIVR")]
    pub max_ivr: Option<f64>,
    #[serde(rename = "minDelta")]
    pub min_delta: Option<f64>,
    #[serde(rename = "maxDelta")]
    pub max_delta: Option<f64>,
    #[serde(rename = "minDTE")]
    pub min_dte: Option<i32>,
    #[serde(rename = "maxDTE")]
    pub max_dte: Option<i32>,
    #[serde(rename = "maxAllocationPct")]
    pub max_allocation_pct: Option<f64>,
    #[serde(rename = "targetProfitPct")]
    pub target_profit_pct: Option<f64>,
    #[serde(rename = "managementRule")]
    pub management_rule: Option<String>,
    #[serde(rename = "minPOP")]
    pub min_pop: Option<f64>,
    #[serde(rename = "vixMin")]
    pub vix_min: Option<f64>,
    #[serde(rename = "vixMax")]
    pub vix_max: Option<f64>,
    #[serde(rename = "maxBprPct")]
    pub max_bpr_pct: Option<f64>,
    #[serde(rename = "notes")]
    pub notes: Option<String>,

    // Structured exit ladder (ISC-30, ISC-31, ISC-32)
    #[serde(rename = "stopLossPct")]
    pub stop_loss_pct: Option<f64>,      // e.g. 200.0 = close if loss exceeds 2× credit
    #[serde(rename = "profitTargetPct")]
    pub profit_target_pct: Option<f64>, // override target_profit_pct for this playbook ladder
    #[serde(rename = "dteExit")]
    pub dte_exit: Option<i32>,           // close position at this DTE regardless of P&L

    // Avoidance conditions (ISC-33)
    #[serde(rename = "whenToAvoid")]
    pub when_to_avoid: Option<String>,   // conditions under which NOT to trade this setup

    // Tier 4: credit quality filters
    #[serde(rename = "minCredit")]
    pub min_credit: Option<f64>,              // minimum credit to collect per contract (e.g. 0.30)
    #[serde(rename = "minCreditWidthRatio")]
    pub min_credit_width_ratio: Option<f64>,  // min credit as % of spread width (target 25–33%)
    #[serde(rename = "earningsBlackoutDays")]
    pub earnings_blackout_days: Option<i32>,  // don't enter within N days of earnings
}

// ──────────────────────────────────────────────────────────────────────────────
// Playbook Strategy
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaybookStrategy {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub spread_type: Option<String>,
    pub entry_criteria: Option<EntryCriteria>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Playbook Analytics (L10 + L11)
// ──────────────────────────────────────────────────────────────────────────────

/// Per-playbook analytics: matched vs unmatched win rate (L10) + violation frequency (L11)
#[derive(Debug, Clone)]
pub struct PlaybookAnalytics {
    pub playbook_id: i32,
    // L10: win rate for trades linked to this playbook vs all others
    pub matched_trades:    usize,
    pub matched_wins:      usize,
    pub matched_win_rate:  f64,
    pub unmatched_trades:  usize,
    pub unmatched_wins:    usize,
    pub unmatched_win_rate: f64,
    // L11: top violation fields by frequency (field+rule, count)
    pub top_violations: Vec<(String, usize)>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Portfolio Stats (displayed in Dashboard)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct PortfolioStats {
    // Core P&L
    pub total_pnl: f64,
    pub realized_pnl: f64,     // closed trades only
    pub win_rate: f64,

    // Trade counts
    pub open_trades: usize,
    pub closed_trades: usize,
    pub total_trades: usize,

    // Performance metrics
    pub avg_roc: f64,           // average ROC% on closed trades
    pub avg_pnl_per_trade: f64,
    pub best_trade_pnl: f64,
    pub worst_trade_pnl: f64,

    // Drawdown
    pub max_drawdown: f64,
    pub max_drawdown_pct: f64,

    // Streak
    pub current_streak: i64,    // positive=win streak, negative=loss streak
    pub max_win_streak: usize,
    pub max_loss_streak: usize,

    // Greeks
    pub net_beta_weighted_delta: f64,   // BWD = Σ delta × beta × (underlying/SPY) × qty × 100
    pub net_theta: f64,         // total daily theta on open positions
    pub spy_price: Option<f64>,         // reference SPY price used for BWD calc
    pub spx_price: Option<f64>,         // S&P 500 index price for VIX EM formula

    // OTJ Dashboard metrics
    pub account_size: f64,
    pub balance: f64,              // account_size + realized_pnl
    pub alloc_pct: f64,            // total_open_bpr / account_size × 100
    pub total_open_bpr: f64,
    pub undefined_risk_bpr: f64,   // BPR in undefined-risk strategies (CSP, CC, STR, STD)
    pub defined_risk_bpr: f64,     // BPR in defined-risk strategies
    pub undefined_risk_pct: f64,   // % of open BPR that is undefined
    pub defined_risk_pct: f64,     // % of open BPR that is defined
    pub target_undefined_pct: f64, // target allocation (default 75.0)
    pub drift: f64,                // undefined_risk_pct - target_undefined_pct
    pub avg_pop: f64,              // avg POP% across open trades
    pub vix: Option<f64>,          // current VIX (fetched at startup from Yahoo)
    pub net_vega: f64,             // total portfolio vega
    pub bp_available: f64,         // account_size - total_open_bpr
    pub unrealized_pnl: f64,       // estimated unrealized P&L (theta × days_held × 100 × qty)

    // M10: avg IVR across open positions
    pub avg_ivr_open: Option<f64>,

    // M5: upcoming expirations (DTE ≤ 21), sorted by DTE asc — (ticker, dte)
    pub next_critical_positions: Vec<(String, i32)>,

    // M1: avg P50 across open positions
    pub avg_p50_open: Option<f64>,

    // L2: theta/delta efficiency ratio
    pub theta_delta_ratio: Option<f64>,

    // tastytrade KPI: net_theta / account_size × 100  (target 0.1–0.3% daily)
    pub theta_netliq_ratio: Option<f64>,

    // Theta/BPR efficiency: net_theta / total_open_bpr × 100 (% return on capital per day)
    pub theta_bpr_ratio: Option<f64>,

    // L1: monthly P&L pace target + pace
    pub monthly_pnl_target: f64,
    pub monthly_pnl_pace: f64,

    // L3: strategy distribution for open trades (sorted by count desc)
    pub open_strategy_counts: Vec<(String, usize)>,

    // Item 13: portfolio stress test scenarios
    pub stress_test: Vec<StressPoint>,
    pub stress_priced_count: usize,  // open trades with underlying_price > 0 and legs
    pub stress_open_count: usize,    // total open trades

    // KPI 4: current drawdown from recent peak
    pub current_drawdown_pct: f64,

    // KPI 5: largest single position concentration (BPR as % of account)
    pub largest_position_bpr_pct: f64,
    pub largest_position_ticker: Option<String>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Performance Stats (displayed in Performance tab)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StrategyBreakdown {
    pub strategy: StrategyType,
    pub trades: usize,      // closed trades
    pub open_count: usize,  // open (active) trades
    pub wins: usize,
    pub scratches: usize,
    pub total_pnl: f64,
    pub avg_pnl: f64,
    pub avg_roc: f64,
    pub win_rate: f64,
    pub scratch_rate: f64,
    pub avg_cw_ratio: Option<f64>,   // avg credit/width ratio (defined-risk only)
    pub avg_entry_dte: Option<f64>,  // avg DTE at entry
}

#[derive(Debug, Clone)]
pub struct TickerBreakdown {
    pub ticker: String,
    pub trades: usize,
    pub wins: usize,
    pub scratches: usize,
    pub total_pnl: f64,
    pub avg_pnl: f64,
    pub avg_roc: f64,
    pub win_rate: f64,
    pub scratch_rate: f64,
    pub avg_ivr: Option<f64>,
    pub avg_entry_dte: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthlyPnl {
    pub year: i32,
    pub month: u32,
    pub pnl: f64,
    pub trade_count: usize,
    pub win_count: usize,
}

/// 0DTE performance by expiry day of week
#[derive(Debug, Clone)]
pub struct WeekdayStats {
    pub label: &'static str,   // "Mon", "Tue", "Wed", "Thu", "Fri"
    pub trade_count: usize,
    pub win_count: usize,
    pub total_pnl: f64,
    pub avg_pnl: f64,
}

/// Item 4: Time-in-Trade histogram bucket
#[derive(Debug, Clone)]
pub struct HeldBucket {
    pub label: &'static str,
    pub trades: usize,
    pub wins: usize,
    pub win_rate: f64,  // 0.0–100.0
    pub avg_pnl: f64,
}

/// Item 13: Portfolio stress test scenario
#[derive(Debug, Clone)]
pub struct StressPoint {
    pub spy_move_pct: f64,     // e.g. -20.0
    pub total_pnl: f64,        // sum across all open positions
    pub pct_of_account: f64,   // total_pnl / account_size * 100
    pub worst_ticker: String,  // ticker with most negative P&L at this move
    pub worst_pnl: f64,        // that position's P&L
}

/// L10: IVR entry frequency histogram bucket
#[derive(Debug, Clone)]
pub struct IvrEntryBucket {
    pub label: &'static str,
    pub count: usize,
    pub win_rate: f64,    // 0.0–100.0
}

/// L6: Per-sector trade count per month (parallel to perf.monthly_pnl)
#[derive(Debug, Clone, Serialize)]
pub struct SectorTrend {
    pub sector: String,
    pub monthly_counts: Vec<usize>,  // one entry per MonthlyPnl entry (same ordering)
    pub total_trades: usize,
}

/// Performance chart data payload — serialised via OSC 9998 to Tauri right panel
#[derive(Debug, Clone, Serialize)]
pub struct PerfChartPayload {
    pub account_size:       f64,
    pub balance_history:    Vec<f64>,
    pub unrealized_history: Vec<f64>,
    pub peak_history:       Vec<f64>,
    pub monthly_pnl:        Vec<MonthlyPnl>,
    pub rolling_win_rate:        Vec<f64>,
    pub rolling_theta_capture:   Vec<f64>,
    pub dte_roc_scatter:         Vec<(i32, f64, String)>,  // (dte, roc%, strategy label)
    pub bpr_history:             Vec<f64>,
    pub sector_trends:           Vec<SectorTrend>,
}

/// Item 4: P&L distribution histogram bucket
#[derive(Debug, Clone)]
pub struct PnlBucket {
    pub label: &'static str,
    pub count: usize,
    pub pct: f64,           // % of closed trades in this bucket
    pub normal_count: f64,  // expected count under normal distribution fit to the data
}

/// IVR bucket: win rate by IV Rank at entry
#[derive(Debug, Clone)]
pub struct IvrBucket {
    pub label: &'static str,    // e.g. "IVR 50-75"
    pub min_ivr: f64,
    pub max_ivr: f64,
    pub trades: usize,
    pub wins: usize,
    pub win_rate: f64,          // 0.0–100.0
    pub avg_pnl: f64,
}

/// DTE-at-close bucket: performance by how many DTE remained when trade was closed
#[derive(Debug, Clone)]
pub struct DteBucket {
    pub label: &'static str,    // e.g. "0-7d"
    pub trades: usize,
    pub wins: usize,
    pub avg_pnl: f64,
    pub win_rate: f64,          // 0.0–100.0
}

/// VIX regime bucket: performance by VIX environment at entry
#[derive(Debug, Clone)]
pub struct VixRegime {
    pub label: &'static str,    // e.g. "Normal (15-20)"
    pub trades: usize,
    pub wins: usize,
    pub win_rate: f64,          // 0.0–100.0
    pub avg_pnl: f64,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub win_rate: f64,
    pub scratch_rate: f64,
    pub scratches: usize,

    // Win/Loss quality
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub expected_value: f64,
    pub kelly_fraction: Option<f64>,  // Kelly Criterion optimal position size (% of account)
    pub avg_credit_per_dte: Option<f64>,  // avg (credit × 100 × qty / entry_dte) per closed trade
    pub avg_max_profit_per_day: Option<f64>, // avg (max_profit_dollars / entry_dte) per credit trade
    pub avg_theta_capture_quality: Option<f64>, // avg (actual_pnl / theoretical_theta_pnl) as %

    // Risk-adjusted return
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub avg_annualized_roc: f64,   // average ROC × (365 / days_held)

    // Exit quality
    pub avg_dte_at_close: Option<f64>,
    pub avg_pct_max_captured: Option<f64>,

    // Entry quality
    pub avg_credit_width_ratio: Option<f64>,   // avg (credit/width)×100 for defined-risk trades
    pub avg_iv_crush: Option<f64>,             // avg entry_iv - close_iv in % points

    // Trade frequency
    pub trades_per_week: f64,
    pub trades_per_month: f64,
    pub avg_held_days: f64,

    // Strategy breakdown (sorted by trade count desc)
    pub strategy_breakdown: Vec<StrategyBreakdown>,

    // Ticker breakdown (sorted by trade count desc)
    pub ticker_breakdown: Vec<TickerBreakdown>,

    // Monthly P&L (sorted chronologically)
    pub monthly_pnl: Vec<MonthlyPnl>,

    // Balance history: account_size then account_size + running_pnl per trade
    pub balance_history: Vec<f64>,

    // IVR-bucketed win rate (always 4 buckets: 0-25, 25-50, 50-75, 75+)
    pub ivr_buckets: Vec<IvrBucket>,

    // VIX regime performance (5 regimes: Calm, Normal, Elevated, High, Stress)
    pub vix_regimes: Vec<VixRegime>,

    // M3: P&L bucketed by DTE-at-close (5 buckets)
    pub dte_buckets: Vec<DteBucket>,

    // M8: rolling 30-trade win rate (one entry per closed trade)
    pub rolling_win_rate: Vec<f64>,

    // M9: running peak balance (same length as balance_history)
    pub peak_history: Vec<f64>,

    // L6: average premium recapture rate across closed trades
    pub avg_premium_recapture: Option<f64>,

    // L7: rolling 30-trade theta capture efficiency (%)
    pub rolling_theta_capture: Vec<f64>,

    // L10: IVR entry frequency histogram (4 buckets: <25, 25-50, 50-75, 75+)
    pub ivr_entry_buckets: Vec<IvrEntryBucket>,

    // Item 4: time-in-trade histogram (5 buckets: 0-7, 7-14, 14-21, 21-30, 30+)
    pub held_buckets: Vec<HeldBucket>,

    // Commission analysis
    pub total_commissions: f64,
    pub avg_commission_per_trade: f64,
    pub commission_pct_of_gross: f64,
    pub avg_fill_vs_mid: Option<f64>,

    // Rolling window configuration
    pub rolling_window_used: usize,

    // Item 4: P&L distribution histogram
    pub pnl_buckets: Vec<PnlBucket>,

    // M5: profit target hit rate
    pub target_hit_count: usize,
    pub target_hit_pct: f64,    // % of closed trades that reached their profit target
    pub closed_count: usize,

    // M9: DTE@entry vs ROC scatter points
    pub dte_roc_scatter: Vec<(i32, f64, StrategyType)>,  // (entry_dte, roc_pct, strategy)

    // M11: unrealized history — same as balance_history but final point adds open-position theta estimate
    pub unrealized_history: Vec<f64>,

    // L5: BPR per closed trade chronologically (position sizing consistency)
    pub bpr_history: Vec<f64>,

    // L6: Sector exposure by month (sparkline rows per sector)
    pub sector_trends: Vec<SectorTrend>,

    // 0DTE: monthly P&L for same-day expiry trades only
    pub monthly_0dte_pnl: Vec<MonthlyPnl>,
    // 0DTE: P&L breakdown by expiry day of week (Mon/Wed/Fri)
    pub dte_weekday_stats: Vec<WeekdayStats>,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            win_rate: 0.0, scratch_rate: 0.0, scratches: 0,
            avg_win: 0.0, avg_loss: 0.0, profit_factor: 0.0, expected_value: 0.0, kelly_fraction: None,
            avg_credit_per_dte: None, avg_max_profit_per_day: None, avg_theta_capture_quality: None,
            sharpe_ratio: 0.0, sortino_ratio: 0.0, calmar_ratio: 0.0, avg_annualized_roc: 0.0,
            avg_dte_at_close: None, avg_pct_max_captured: None,
            avg_credit_width_ratio: None, avg_iv_crush: None,
            trades_per_week: 0.0, trades_per_month: 0.0, avg_held_days: 0.0,
            strategy_breakdown: vec![], ticker_breakdown: vec![], monthly_pnl: vec![],
            balance_history: vec![], ivr_buckets: vec![], vix_regimes: vec![],
            dte_buckets: vec![], rolling_win_rate: vec![], peak_history: vec![],
            avg_premium_recapture: None, rolling_theta_capture: vec![],
            ivr_entry_buckets: vec![],
            held_buckets: vec![],
            total_commissions: 0.0, avg_commission_per_trade: 0.0, commission_pct_of_gross: 0.0,
            avg_fill_vs_mid: None,
            rolling_window_used: 30,
            pnl_buckets: vec![],
            target_hit_count: 0,
            target_hit_pct: 0.0,
            closed_count: 0,
            dte_roc_scatter: vec![],
            unrealized_history: vec![],
            bpr_history: vec![],
            sector_trends: vec![],
            monthly_0dte_pnl: vec![],
            dte_weekday_stats: vec![],
        }
    }
}
