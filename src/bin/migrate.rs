/// One-time migration utility: import OTJ full_export.json into trades.db.
/// Usage: cargo run --bin migrate -- [path_to_export.json] [trades.db]
use serde::Deserialize;
use chrono::{DateTime, Utc};
use theta_vault_rust::models::{Trade, StrategyType, TradeLeg, PlaybookStrategy, EntryCriteria};
use theta_vault_rust::storage::Storage;
use std::fs;

#[derive(Debug, Deserialize)]
struct ExportedTrade {
    ticker: String,
    #[serde(rename = "spreadType")]
    spread_type: String,
    quantity: i32,
    #[serde(rename = "shortStrike", default)]
    short_strike: f64,
    #[serde(rename = "longStrike", default)]
    long_strike: f64,
    #[serde(rename = "shortPremium", default)]
    short_premium: f64,
    #[serde(rename = "longPremium", default)]
    long_premium: f64,
    #[serde(rename = "creditReceived")]
    credit_received: f64,
    #[serde(rename = "entryTime")]
    entry_time: String,
    #[serde(rename = "exitTime")]
    exit_time: Option<String>,
    #[serde(rename = "expirationDate")]
    expiration_date: String,
    #[serde(rename = "tradeDate")]
    trade_date: String,
    pnl: Option<f64>,
    #[serde(rename = "debitPaid")]
    debit_paid: Option<f64>,
    delta: Option<f64>,
    theta: Option<f64>,
    gamma: Option<f64>,
    vega: Option<f64>,
    pop: Option<f64>,
    #[serde(rename = "underlyingPrice")]
    underlying_price: Option<f64>,
    #[serde(rename = "underlyingPriceAtClose")]
    underlying_price_at_close: Option<f64>,
    #[serde(rename = "ivRank")]
    iv_rank: Option<f64>,
    #[serde(rename = "vixAtEntry")]
    vix_at_entry: Option<f64>,
    #[serde(rename = "impliedVolatility")]
    implied_volatility: Option<f64>,
    commission: Option<f64>,
    #[serde(rename = "exitReason")]
    exit_reason: Option<String>,
    #[serde(rename = "managementRule")]
    management_rule: Option<String>,
    legs: Option<serde_json::Value>,
    tags: Option<Vec<String>>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExportedPlaybook {
    name: String,
    description: Option<String>,
    #[serde(rename = "spreadType")]
    spread_type: Option<String>,
    #[serde(rename = "entryCriteria")]
    entry_criteria: Option<EntryCriteria>,
}

#[derive(Debug, Deserialize)]
struct FullExport {
    trades: Vec<ExportedTrade>,
    playbooks: Vec<ExportedPlaybook>,
}

fn parse_date(s: &str) -> DateTime<Utc> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(nd) = chrono::NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d") {
        if let Some(ndt) = nd.and_hms_opt(0, 0, 0) {
            return DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
        }
    }
    Utc::now()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let json_path = args.get(1).cloned()
        .unwrap_or_else(|| "../OptionsTradingJournal/full_export.json".to_string());
    let db_path = args.get(2).cloned()
        .unwrap_or_else(|| "trades.db".to_string());

    println!("Reading {json_path}...");
    let data = fs::read_to_string(&json_path)?;
    let export: FullExport = serde_json::from_str(&data)?;

    let storage = Storage::new(&db_path)?;
    storage.clear_trades()?;
    storage.clear_playbooks()?;

    println!("Migrating {} trades...", export.trades.len());
    for et in &export.trades {
        let strategy = StrategyType::from_str(&et.spread_type);

        let legs: Vec<TradeLeg> = et.legs.as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let entry_date  = parse_date(&et.entry_time);
        let trade_date  = parse_date(&et.trade_date);
        let exit_date   = et.exit_time.as_ref().map(|s| parse_date(s));
        let exp_date    = parse_date(&et.expiration_date);

        let trade = Trade {
            id: 0,
            ticker: et.ticker.clone(),
            strategy,
            quantity: et.quantity,
            short_strike: et.short_strike,
            long_strike: et.long_strike,
            short_premium: et.short_premium,
            long_premium: et.long_premium,
            credit_received: et.credit_received,
            entry_date,
            exit_date,
            expiration_date: exp_date,
            trade_date,
            back_month_expiration: None,
            pnl: et.pnl,
            debit_paid: et.debit_paid,
            delta: et.delta,
            theta: et.theta,
            gamma: et.gamma,
            vega: et.vega,
            pop: et.pop,
            underlying_price: et.underlying_price,
            underlying_price_at_close: et.underlying_price_at_close,
            iv_rank: et.iv_rank,
            vix_at_entry: et.vix_at_entry,
            implied_volatility: et.implied_volatility,
            commission: et.commission,
            entry_reason: None,
            exit_reason: et.exit_reason.clone(),
            management_rule: et.management_rule.clone(),
            target_profit_pct: None,
            spread_width: None,
            bpr: None,
            entry_dte: None,
            dte_at_close: None,
            playbook_id: None,
            rolled_from_id: None,
            is_earnings_play: false,
            is_tested: false,
            trade_grade: None,
            grade_notes: None,
            legs,
            tags: et.tags.clone().unwrap_or_default(),
            notes: et.notes.clone(),
            next_earnings: None,
            iv_at_close: None,
            delta_at_close: None,
            roll_count: 0,
            theta_at_close: None,
            gamma_at_close: None,
            vega_at_close: None,
        };
        storage.insert_trade(&trade)?;
    }

    println!("Migrating {} playbooks...", export.playbooks.len());
    for ep in &export.playbooks {
        let pb = PlaybookStrategy {
            id: 0,
            name: ep.name.clone(),
            description: ep.description.clone(),
            spread_type: ep.spread_type.clone(),
            entry_criteria: ep.entry_criteria.clone(),
        };
        storage.insert_playbook(&pb)?;
    }

    println!("Done: migrated {} trades, {} playbooks into {}",
             export.trades.len(), export.playbooks.len(), db_path);
    Ok(())
}
