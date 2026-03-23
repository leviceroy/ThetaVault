// Copyright (c) 2025 Chris Wenk. All rights reserved.

use rusqlite::{params, Connection, OptionalExtension, Result};
use crate::models::{Trade, StrategyType, TradeLeg, PlaybookStrategy};
use chrono::{DateTime, NaiveDate, Utc};

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Self::init_db(&conn)?;
        Self::migrate_db(&conn)?;
        Ok(Storage { conn })
    }

    fn init_db(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trades (
                id                        INTEGER PRIMARY KEY AUTOINCREMENT,
                ticker                    TEXT NOT NULL,
                strategy                  TEXT NOT NULL,
                quantity                  INTEGER NOT NULL,

                -- Strike prices
                short_strike              REAL NOT NULL DEFAULT 0,
                long_strike               REAL NOT NULL DEFAULT 0,

                -- Premiums per leg (entry)
                short_premium             REAL NOT NULL DEFAULT 0,
                long_premium              REAL NOT NULL DEFAULT 0,

                -- Net credit at entry
                credit_received           REAL NOT NULL,

                -- Timestamps
                entry_date                TEXT NOT NULL,
                exit_date                 TEXT,
                expiration_date           TEXT NOT NULL,
                trade_date                TEXT NOT NULL,
                back_month_expiration     TEXT,

                -- P&L
                pnl                       REAL,
                debit_paid                REAL,

                -- Greeks at entry
                delta                     REAL,
                theta                     REAL,
                gamma                     REAL,
                vega                      REAL,
                pop                       REAL,

                -- Underlying / IV
                underlying_price          REAL,
                underlying_price_at_close REAL,
                iv_rank                   REAL,
                iv_percentile             REAL,
                vix_at_entry              REAL,
                implied_volatility        REAL,

                -- Trade metadata
                commission                REAL,
                entry_reason              TEXT,
                exit_reason               TEXT,
                management_rule           TEXT,
                target_profit_pct         REAL,

                -- Computed fields
                spread_width              REAL,
                bpr                       REAL,
                entry_dte                 INTEGER,
                dte_at_close              INTEGER,

                -- Relationships
                playbook_id               INTEGER,
                rolled_from_id            INTEGER,

                -- Status flags
                is_earnings_play          INTEGER NOT NULL DEFAULT 0,
                is_tested                 INTEGER NOT NULL DEFAULT 0,

                -- Grading
                trade_grade               TEXT,
                grade_notes               TEXT,

                -- Legs (JSON array)
                legs_json                 TEXT NOT NULL DEFAULT '[]',

                -- Tags
                tags                      TEXT NOT NULL DEFAULT '',

                -- Notes
                notes                     TEXT,

                -- Earnings date
                next_earnings             TEXT,

                -- Close-side Greeks
                iv_at_close               REAL,
                delta_at_close            REAL,
                roll_count                INTEGER NOT NULL DEFAULT 0,
                theta_at_close            REAL,
                gamma_at_close            REAL,
                vega_at_close             REAL,

                -- Execution quality
                bid_ask_spread_at_entry   REAL,
                fill_vs_mid               REAL,

                -- Assignment tracking
                was_assigned              INTEGER NOT NULL DEFAULT 0,
                assigned_shares           INTEGER,
                cost_basis                REAL,

                -- Unique constraint: prevent duplicate imports
                UNIQUE(trade_date, ticker, strategy, short_strike, long_strike, quantity)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playbook_strategies (
                id                  INTEGER PRIMARY KEY AUTOINCREMENT,
                name                TEXT NOT NULL,
                description         TEXT,
                spread_type         TEXT,
                entry_criteria_json TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    /// Add any missing columns to existing databases (forward-compatibility migration).
    fn migrate_db(conn: &Connection) -> Result<()> {
        let existing_cols: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(trades)")?;
            let cols: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .collect();
            cols
        };

        let migrations: &[(&str, &str)] = &[
            ("short_strike",              "ALTER TABLE trades ADD COLUMN short_strike REAL NOT NULL DEFAULT 0"),
            ("long_strike",               "ALTER TABLE trades ADD COLUMN long_strike REAL NOT NULL DEFAULT 0"),
            ("short_premium",             "ALTER TABLE trades ADD COLUMN short_premium REAL NOT NULL DEFAULT 0"),
            ("long_premium",              "ALTER TABLE trades ADD COLUMN long_premium REAL NOT NULL DEFAULT 0"),
            ("trade_date",                "ALTER TABLE trades ADD COLUMN trade_date TEXT NOT NULL DEFAULT ''"),
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
            ("next_earnings",             "ALTER TABLE trades ADD COLUMN next_earnings TEXT"),
            ("iv_at_close",              "ALTER TABLE trades ADD COLUMN iv_at_close REAL"),
            ("delta_at_close",           "ALTER TABLE trades ADD COLUMN delta_at_close REAL"),
            ("roll_count",               "ALTER TABLE trades ADD COLUMN roll_count INTEGER NOT NULL DEFAULT 0"),
            ("theta_at_close",           "ALTER TABLE trades ADD COLUMN theta_at_close REAL"),
            ("gamma_at_close",           "ALTER TABLE trades ADD COLUMN gamma_at_close REAL"),
            ("vega_at_close",            "ALTER TABLE trades ADD COLUMN vega_at_close REAL"),
            ("bid_ask_spread_at_entry",  "ALTER TABLE trades ADD COLUMN bid_ask_spread_at_entry REAL"),
            ("fill_vs_mid",              "ALTER TABLE trades ADD COLUMN fill_vs_mid REAL"),
            ("was_assigned",             "ALTER TABLE trades ADD COLUMN was_assigned INTEGER NOT NULL DEFAULT 0"),
            ("assigned_shares",          "ALTER TABLE trades ADD COLUMN assigned_shares INTEGER"),
            ("cost_basis",               "ALTER TABLE trades ADD COLUMN cost_basis REAL"),
            ("close_notes",              "ALTER TABLE trades ADD COLUMN close_notes TEXT"),
            ("sector",                   "ALTER TABLE trades ADD COLUMN sector TEXT"),
            ("closed_at_target",         "ALTER TABLE trades ADD COLUMN closed_at_target INTEGER NOT NULL DEFAULT 0"),
            ("iv_percentile",            "ALTER TABLE trades ADD COLUMN iv_percentile REAL"),
        ];

        for (col_name, sql) in migrations {
            if !existing_cols.iter().any(|c| c == col_name) {
                if let Err(e) = conn.execute(sql, []) {
                    eprintln!("Migration warning ({}): {}", col_name, e);
                }
            }
        }

        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // Trade CRUD
    // ────────────────────────────────────────────────────────────────────────

    pub fn clear_trades(&self) -> Result<()> {
        self.conn.execute("DELETE FROM trades", [])?;
        Ok(())
    }

    /// Update mutable fields of an existing trade.
    pub fn update_trade(&self, id: i32, t: &Trade) -> Result<()> {
        let legs_json = serde_json::to_string(&t.legs)
            .unwrap_or_else(|_| "[]".to_string());
        let tags_str  = t.tags.join(",");
        let strategy_str = t.strategy.as_str();

        self.conn.execute(
            "UPDATE trades SET
                ticker=?1, strategy=?2, quantity=?3,
                short_strike=?4, long_strike=?5,
                short_premium=?6, long_premium=?7,
                credit_received=?8,
                entry_date=?9, exit_date=?10,
                expiration_date=?11, trade_date=?12, back_month_expiration=?13,
                pnl=?14, debit_paid=?15,
                delta=?16, theta=?17, gamma=?18, vega=?19, pop=?20,
                underlying_price=?21, underlying_price_at_close=?22,
                iv_rank=?23, vix_at_entry=?24, implied_volatility=?25,
                commission=?26, entry_reason=?27, exit_reason=?28,
                management_rule=?29, target_profit_pct=?30,
                spread_width=?31, bpr=?32, entry_dte=?33, dte_at_close=?34,
                playbook_id=?35, rolled_from_id=?36,
                is_earnings_play=?37, is_tested=?38,
                trade_grade=?39, grade_notes=?40,
                legs_json=?41, tags=?42, notes=?43, next_earnings=?44,
                iv_at_close=?45, delta_at_close=?46, roll_count=?47,
                theta_at_close=?48, gamma_at_close=?49, vega_at_close=?50,
                bid_ask_spread_at_entry=?51, fill_vs_mid=?52,
                was_assigned=?53, assigned_shares=?54, cost_basis=?55,
                close_notes=?56, sector=?57, closed_at_target=?58,
                iv_percentile=?59
             WHERE id=?60",
            params![
                t.ticker,                               // 1
                strategy_str,                           // 2
                t.quantity,                             // 3
                t.short_strike,                         // 4
                t.long_strike,                          // 5
                t.short_premium,                        // 6
                t.long_premium,                         // 7
                t.credit_received,                      // 8
                t.entry_date.to_rfc3339(),              // 9
                t.exit_date.map(|d| d.to_rfc3339()),   // 10
                t.expiration_date.to_rfc3339(),         // 11
                t.trade_date.to_rfc3339(),              // 12
                t.back_month_expiration.map(|d| d.to_rfc3339()), // 13
                t.pnl,                                  // 14
                t.debit_paid,                           // 15
                t.delta,                                // 16
                t.theta,                                // 17
                t.gamma,                                // 18
                t.vega,                                 // 19
                t.pop,                                  // 20
                t.underlying_price,                     // 21
                t.underlying_price_at_close,            // 22
                t.iv_rank,                              // 23
                t.vix_at_entry,                         // 24
                t.implied_volatility,                   // 25
                t.commission,                           // 26
                t.entry_reason.as_deref(),              // 27
                t.exit_reason.as_deref(),               // 28
                t.management_rule.as_deref(),           // 29
                t.target_profit_pct,                    // 30
                t.spread_width,                         // 31
                t.bpr,                                  // 32
                t.entry_dte,                            // 33
                t.dte_at_close,                         // 34
                t.playbook_id,                          // 35
                t.rolled_from_id,                       // 36
                t.is_earnings_play as i32,              // 37
                t.is_tested as i32,                     // 38
                t.trade_grade.as_deref(),               // 39
                t.grade_notes.as_deref(),               // 40
                legs_json,                              // 41
                tags_str,                               // 42
                t.notes.as_deref(),                     // 43
                t.next_earnings.map(|d| d.format("%Y-%m-%d").to_string()), // 44
                t.iv_at_close,                          // 45
                t.delta_at_close,                       // 46
                t.roll_count,                           // 47
                t.theta_at_close,                       // 48
                t.gamma_at_close,                       // 49
                t.vega_at_close,                        // 50
                t.bid_ask_spread_at_entry,              // 51
                t.fill_vs_mid,                          // 52
                t.was_assigned as i32,                  // 53
                t.assigned_shares,                      // 54
                t.cost_basis,                           // 55
                t.close_notes.as_deref(),               // 56
                t.sector.as_deref(),                    // 57
                t.closed_at_target as i32,              // 58
                t.iv_percentile,                        // 59
                id,                                     // 60
            ],
        )?;
        Ok(())
    }

    /// Delete a single trade by id.
    pub fn delete_trade(&self, id: i32) -> Result<()> {
        self.conn.execute("DELETE FROM trades WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn insert_trade(&self, trade: &Trade) -> Result<i64> {
        let legs_json = serde_json::to_string(&trade.legs)
            .unwrap_or_else(|_| "[]".to_string());
        let tags_str  = trade.tags.join(",");

        // strategy stored as plain snake_case string (matches OTJ API format)
        let strategy_str = trade.strategy.as_str();

        self.conn.execute(
            "INSERT OR IGNORE INTO trades (
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
                legs_json, tags, notes, next_earnings,
                iv_at_close, delta_at_close, roll_count,
                theta_at_close, gamma_at_close, vega_at_close,
                bid_ask_spread_at_entry, fill_vs_mid,
                was_assigned, assigned_shares, cost_basis,
                close_notes, sector, closed_at_target, iv_percentile
            ) VALUES (
                ?1,  ?2,  ?3,
                ?4,  ?5,  ?6,  ?7,
                ?8,
                ?9,  ?10, ?11, ?12, ?13,
                ?14, ?15,
                ?16, ?17, ?18, ?19, ?20,
                ?21, ?22,
                ?23, ?24, ?25,
                ?26, ?27, ?28, ?29, ?30,
                ?31, ?32, ?33, ?34,
                ?35, ?36,
                ?37, ?38,
                ?39, ?40,
                ?41, ?42, ?43, ?44,
                ?45, ?46, ?47,
                ?48, ?49, ?50,
                ?51, ?52,
                ?53, ?54, ?55,
                ?56, ?57, ?58, ?59
            )",
            params![
                trade.ticker,               // 1
                strategy_str,               // 2
                trade.quantity,             // 3
                trade.short_strike,         // 4
                trade.long_strike,          // 5
                trade.short_premium,        // 6
                trade.long_premium,         // 7
                trade.credit_received,      // 8
                trade.entry_date.to_rfc3339(),  // 9
                trade.exit_date.map(|d| d.to_rfc3339()),  // 10
                trade.expiration_date.to_rfc3339(),  // 11
                trade.trade_date.to_rfc3339(),  // 12
                trade.back_month_expiration.map(|d| d.to_rfc3339()),  // 13
                trade.pnl,                  // 14
                trade.debit_paid,           // 15
                trade.delta,                // 16
                trade.theta,                // 17
                trade.gamma,                // 18
                trade.vega,                 // 19
                trade.pop,                  // 20
                trade.underlying_price,     // 21
                trade.underlying_price_at_close,  // 22
                trade.iv_rank,              // 23
                trade.vix_at_entry,         // 24
                trade.implied_volatility,   // 25
                trade.commission,           // 26
                trade.entry_reason.as_deref(),  // 27
                trade.exit_reason.as_deref(),   // 28
                trade.management_rule.as_deref(),  // 29
                trade.target_profit_pct,    // 30
                trade.spread_width,         // 31
                trade.bpr,                  // 32
                trade.entry_dte,            // 33
                trade.dte_at_close,         // 34
                trade.playbook_id,          // 35
                trade.rolled_from_id,       // 36
                trade.is_earnings_play as i32,  // 37
                trade.is_tested as i32,         // 38
                trade.trade_grade.as_deref(),   // 39
                trade.grade_notes.as_deref(),   // 40
                legs_json,                  // 41
                tags_str,                   // 42
                trade.notes.as_deref(),     // 43
                trade.next_earnings.map(|d| d.format("%Y-%m-%d").to_string()), // 44
                trade.iv_at_close,          // 45
                trade.delta_at_close,       // 46
                trade.roll_count,           // 47
                trade.theta_at_close,       // 48
                trade.gamma_at_close,       // 49
                trade.vega_at_close,        // 50
                trade.bid_ask_spread_at_entry, // 51
                trade.fill_vs_mid,          // 52
                trade.was_assigned as i32,  // 53
                trade.assigned_shares,      // 54
                trade.cost_basis,           // 55
                trade.close_notes.as_deref(),       // 56
                trade.sector.as_deref(),            // 57
                trade.closed_at_target as i32,      // 58
                trade.iv_percentile,                // 59
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn export_trades_csv(&self, path: &str) -> Result<()> {
        use std::io::{BufWriter, Write};
        let trades = self.get_all_trades()?;
        let file = std::fs::File::create(path).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let mut w = BufWriter::new(file);
        // Helper: quote a string if it contains commas/quotes/newlines
        fn csv_field(s: &str) -> String {
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        }
        let _ = writeln!(w, "id,ticker,strategy,trade_date,expiration_date,quantity,credit_received,debit_paid,pnl,bpr,entry_dte,dte_at_close,exit_date,exit_reason,delta,theta,gamma,vega,pop,underlying_price,iv_rank,vix_at_entry,implied_volatility,underlying_price_at_close,iv_at_close,commission,target_profit_pct,management_rule,trade_grade,grade_notes,notes,close_notes,tags,is_earnings_play,is_tested,bid_ask_spread_at_entry,fill_vs_mid,was_assigned,assigned_shares,cost_basis,roll_count,rolled_from_id,playbook_id,next_earnings");
        for t in &trades {
            let row = format!("{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                t.id,
                csv_field(&t.ticker),
                csv_field(t.strategy.as_str()),
                t.trade_date.format("%Y-%m-%d"),
                t.expiration_date.format("%Y-%m-%d"),
                t.quantity,
                t.credit_received,
                t.debit_paid.map_or(String::new(), |v| format!("{:.4}", v)),
                t.pnl.map_or(String::new(), |v| format!("{:.2}", v)),
                t.bpr.map_or(String::new(), |v| format!("{:.2}", v)),
                t.entry_dte.map_or(String::new(), |v| v.to_string()),
                t.dte_at_close.map_or(String::new(), |v| v.to_string()),
                t.exit_date.map_or(String::new(), |d| d.format("%Y-%m-%d").to_string()),
                csv_field(t.exit_reason.as_deref().unwrap_or("")),
                t.delta.map_or(String::new(), |v| format!("{:.4}", v)),
                t.theta.map_or(String::new(), |v| format!("{:.4}", v)),
                t.gamma.map_or(String::new(), |v| format!("{:.4}", v)),
                t.vega.map_or(String::new(), |v| format!("{:.4}", v)),
                t.pop.map_or(String::new(), |v| format!("{:.1}", v)),
                t.underlying_price.map_or(String::new(), |v| format!("{:.2}", v)),
                t.iv_rank.map_or(String::new(), |v| format!("{:.1}", v)),
                t.vix_at_entry.map_or(String::new(), |v| format!("{:.1}", v)),
                t.implied_volatility.map_or(String::new(), |v| format!("{:.1}", v)),
                t.underlying_price_at_close.map_or(String::new(), |v| format!("{:.2}", v)),
                t.iv_at_close.map_or(String::new(), |v| format!("{:.1}", v)),
                t.commission.map_or(String::new(), |v| format!("{:.2}", v)),
                t.target_profit_pct.map_or(String::new(), |v| format!("{:.1}", v)),
                csv_field(t.management_rule.as_deref().unwrap_or("")),
                csv_field(t.trade_grade.as_deref().unwrap_or("")),
                csv_field(t.grade_notes.as_deref().unwrap_or("")),
                csv_field(t.notes.as_deref().unwrap_or("")),
                csv_field(t.close_notes.as_deref().unwrap_or("")),
                csv_field(&t.tags.join(";")),
                if t.is_earnings_play { "1" } else { "0" },
                if t.is_tested { "1" } else { "0" },
                t.bid_ask_spread_at_entry.map_or(String::new(), |v| format!("{:.4}", v)),
                t.fill_vs_mid.map_or(String::new(), |v| format!("{:.4}", v)),
                if t.was_assigned { "1" } else { "0" },
                t.assigned_shares.map_or(String::new(), |v| v.to_string()),
                t.cost_basis.map_or(String::new(), |v| format!("{:.2}", v)),
                t.roll_count,
                t.rolled_from_id.map_or(String::new(), |v| v.to_string()),
                t.playbook_id.map_or(String::new(), |v| v.to_string()),
                t.next_earnings.map_or(String::new(), |d| d.format("%Y-%m-%d").to_string()),
            );
            let _ = writeln!(w, "{}", row);
        }
        Ok(())
    }

    pub fn get_all_trades(&self) -> Result<Vec<Trade>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, ticker, strategy, quantity,
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
                legs_json, tags, notes, next_earnings,
                iv_at_close, delta_at_close, roll_count,
                theta_at_close, gamma_at_close, vega_at_close,
                bid_ask_spread_at_entry, fill_vs_mid,
                was_assigned, assigned_shares, cost_basis,
                close_notes, sector, closed_at_target, iv_percentile
            FROM trades
            ORDER BY trade_date DESC, entry_date DESC"
        )?;

        let trade_iter = stmt.query_map([], |row| {
            // Parse strategy
            let strategy_str: String = row.get(2)?;
            let strategy = StrategyType::from_str(&strategy_str);

            // Parse timestamps
            let entry_date  = parse_dt(row.get::<_, String>(9)?)
                .unwrap_or_else(Utc::now);
            let exit_date   = row.get::<_, Option<String>>(10)?
                .and_then(|s| parse_dt(s));
            let exp_date    = parse_dt(row.get::<_, String>(11)?)
                .unwrap_or_else(Utc::now);
            let trade_date_str: String = row.get(12)?;
            let trade_date  = if trade_date_str.is_empty() {
                entry_date
            } else {
                parse_dt(trade_date_str).unwrap_or(entry_date)
            };
            let back_month  = row.get::<_, Option<String>>(13)?
                .and_then(|s| parse_dt(s));

            // Parse legs
            let legs_json: String = row.get(41)?;
            let legs: Vec<TradeLeg> = serde_json::from_str(&legs_json)
                .unwrap_or_default();

            // Parse tags
            let tags_str: Option<String> = row.get(42)?;
            let tags = tags_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(|t| t.to_string()).collect())
                .unwrap_or_default();

            // Parse next_earnings
            let next_earnings: Option<NaiveDate> = row.get::<_, Option<String>>(44)?
                .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

            Ok(Trade {
                id:                        row.get(0)?,
                ticker:                    row.get(1)?,
                strategy,
                quantity:                  row.get(3)?,
                short_strike:              row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                long_strike:               row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                short_premium:             row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
                long_premium:              row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
                credit_received:           row.get(8)?,
                entry_date,
                exit_date,
                expiration_date:           exp_date,
                trade_date,
                back_month_expiration:     back_month,
                pnl:                       row.get(14)?,
                debit_paid:                row.get(15)?,
                delta:                     row.get(16)?,
                theta:                     row.get(17)?,
                gamma:                     row.get(18)?,
                vega:                      row.get(19)?,
                pop:                       row.get(20)?,
                underlying_price:          row.get(21)?,
                underlying_price_at_close: row.get(22)?,
                iv_rank:                   row.get(23)?,
                vix_at_entry:              row.get(24)?,
                implied_volatility:        row.get(25)?,
                commission:                row.get(26)?,
                entry_reason:              row.get(27)?,
                exit_reason:               row.get(28)?,
                management_rule:           row.get(29)?,
                target_profit_pct:         row.get(30)?,
                spread_width:              row.get(31)?,
                bpr:                       row.get(32)?,
                entry_dte:                 row.get(33)?,
                dte_at_close:              row.get(34)?,
                playbook_id:               row.get(35)?,
                rolled_from_id:            row.get(36)?,
                is_earnings_play:          row.get::<_, Option<i32>>(37)?.unwrap_or(0) != 0,
                is_tested:                 row.get::<_, Option<i32>>(38)?.unwrap_or(0) != 0,
                trade_grade:               row.get(39)?,
                grade_notes:               row.get(40)?,
                legs,
                tags,
                notes:                     row.get(43)?,
                next_earnings,
                iv_at_close:               row.get(45)?,
                delta_at_close:            row.get(46)?,
                roll_count:                row.get::<_, Option<i32>>(47)?.unwrap_or(0),
                theta_at_close:            row.get(48)?,
                gamma_at_close:            row.get(49)?,
                vega_at_close:             row.get(50)?,
                bid_ask_spread_at_entry:   row.get(51)?,
                fill_vs_mid:               row.get(52)?,
                was_assigned:              row.get::<_, Option<i32>>(53)?.unwrap_or(0) != 0,
                assigned_shares:           row.get(54)?,
                cost_basis:                row.get(55)?,
                close_notes:               row.get(56)?,
                sector:                    row.get(57)?,
                closed_at_target:          row.get::<_, Option<i32>>(58)?.unwrap_or(0) != 0,
                iv_percentile:             row.get(59)?,
            })
        })?;

        let mut trades = Vec::new();
        for trade in trade_iter {
            match trade {
                Ok(t)  => trades.push(t),
                Err(e) => eprintln!("Error loading trade: {}", e),
            }
        }
        Ok(trades)
    }

    pub fn get_trade(&self, id: i32) -> Result<Option<Trade>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, ticker, strategy, quantity,
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
                legs_json, tags, notes, next_earnings,
                iv_at_close, delta_at_close, roll_count,
                theta_at_close, gamma_at_close, vega_at_close,
                bid_ask_spread_at_entry, fill_vs_mid,
                was_assigned, assigned_shares, cost_basis,
                close_notes, sector, closed_at_target, iv_percentile
            FROM trades WHERE id = ?1"
        )?;

        let rows = stmt.query_map(params![id], |row| {
            // Parse strategy
            let strategy_str: String = row.get(2)?;
            let strategy = StrategyType::from_str(&strategy_str);

            // Parse timestamps
            let entry_date  = parse_dt(row.get::<_, String>(9)?)
                .unwrap_or_else(Utc::now);
            let exit_date   = row.get::<_, Option<String>>(10)?
                .and_then(|s| parse_dt(s));
            let exp_date    = parse_dt(row.get::<_, String>(11)?)
                .unwrap_or_else(Utc::now);
            let trade_date_str: String = row.get(12)?;
            let trade_date  = if trade_date_str.is_empty() {
                entry_date
            } else {
                parse_dt(trade_date_str).unwrap_or(entry_date)
            };
            let back_month  = row.get::<_, Option<String>>(13)?
                .and_then(|s| parse_dt(s));

            // Parse legs
            let legs_json: String = row.get(41)?;
            let legs: Vec<TradeLeg> = serde_json::from_str(&legs_json)
                .unwrap_or_default();

            // Parse tags
            let tags_str: Option<String> = row.get(42)?;
            let tags = tags_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(|t| t.to_string()).collect())
                .unwrap_or_default();

            // Parse next_earnings
            let next_earnings: Option<NaiveDate> = row.get::<_, Option<String>>(44)?
                .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

            Ok(Trade {
                id:                        row.get(0)?,
                ticker:                    row.get(1)?,
                strategy,
                quantity:                  row.get(3)?,
                short_strike:              row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                long_strike:               row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                short_premium:             row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
                long_premium:              row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
                credit_received:           row.get(8)?,
                entry_date,
                exit_date,
                expiration_date:           exp_date,
                trade_date,
                back_month_expiration:     back_month,
                pnl:                       row.get(14)?,
                debit_paid:                row.get(15)?,
                delta:                     row.get(16)?,
                theta:                     row.get(17)?,
                gamma:                     row.get(18)?,
                vega:                      row.get(19)?,
                pop:                       row.get(20)?,
                underlying_price:          row.get(21)?,
                underlying_price_at_close: row.get(22)?,
                iv_rank:                   row.get(23)?,
                vix_at_entry:              row.get(24)?,
                implied_volatility:        row.get(25)?,
                commission:                row.get(26)?,
                entry_reason:              row.get(27)?,
                exit_reason:               row.get(28)?,
                management_rule:           row.get(29)?,
                target_profit_pct:         row.get(30)?,
                spread_width:              row.get(31)?,
                bpr:                       row.get(32)?,
                entry_dte:                 row.get(33)?,
                dte_at_close:              row.get(34)?,
                playbook_id:               row.get(35)?,
                rolled_from_id:            row.get(36)?,
                is_earnings_play:          row.get::<_, Option<i32>>(37)?.unwrap_or(0) != 0,
                is_tested:                 row.get::<_, Option<i32>>(38)?.unwrap_or(0) != 0,
                trade_grade:               row.get(39)?,
                grade_notes:               row.get(40)?,
                legs,
                tags,
                notes:                     row.get(43)?,
                next_earnings,
                iv_at_close:               row.get(45)?,
                delta_at_close:            row.get(46)?,
                roll_count:                row.get::<_, Option<i32>>(47)?.unwrap_or(0),
                theta_at_close:            row.get(48)?,
                gamma_at_close:            row.get(49)?,
                vega_at_close:             row.get(50)?,
                bid_ask_spread_at_entry:   row.get(51)?,
                fill_vs_mid:               row.get(52)?,
                was_assigned:              row.get::<_, Option<i32>>(53)?.unwrap_or(0) != 0,
                assigned_shares:           row.get(54)?,
                cost_basis:                row.get(55)?,
                close_notes:               row.get(56)?,
                sector:                    row.get(57)?,
                closed_at_target:          row.get::<_, Option<i32>>(58)?.unwrap_or(0) != 0,
                iv_percentile:             row.get(59)?,
            })
        })?;

        let mut trades = Vec::new();
        for trade in rows {
            if let Ok(t) = trade {
                trades.push(t);
            }
        }
        Ok(trades.into_iter().next())
    }

    /// Recursively fetch the roll chain for a trade (ancestors).
    /// Returns list ordered from oldest (original) to newest (current).
    pub fn get_roll_chain(&self, start_id: i32) -> Result<Vec<Trade>> {
        // 1. Walk backwards and collect every trade in the chain
        let mut chain: Vec<Trade> = Vec::new();
        let mut current_id = Some(start_id);
        
        // Safety: limit depth to avoid infinite loops
        for _ in 0..50 {
            if let Some(cid) = current_id {
                if let Ok(Some(t)) = self.get_trade(cid) {
                    let parent = t.rolled_from_id;
                    chain.push(t);
                    current_id = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        chain.reverse(); // Now oldest -> newest
        Ok(chain)
    }

    // ────────────────────────────────────────────────────────────────────────
    // Playbook CRUD
    // ────────────────────────────────────────────────────────────────────────

    pub fn clear_playbooks(&self) -> Result<()> {
        self.conn.execute("DELETE FROM playbook_strategies", [])?;
        Ok(())
    }

    pub fn insert_playbook(&self, pb: &PlaybookStrategy) -> Result<i64> {
        let criteria_json = serde_json::to_string(&pb.entry_criteria)
            .unwrap_or_else(|_| "null".to_string());
        self.conn.execute(
            "INSERT INTO playbook_strategies (name, description, spread_type, entry_criteria_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![pb.name, pb.description, pb.spread_type, criteria_json],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_playbook(&self, id: i32, pb: &PlaybookStrategy) -> Result<()> {
        let criteria_json = serde_json::to_string(&pb.entry_criteria)
            .unwrap_or_else(|_| "null".to_string());
        self.conn.execute(
            "UPDATE playbook_strategies
             SET name=?1, description=?2, spread_type=?3, entry_criteria_json=?4
             WHERE id=?5",
            params![pb.name, pb.description, pb.spread_type, criteria_json, id],
        )?;
        Ok(())
    }

    pub fn delete_playbook(&self, id: i32) -> Result<()> {
        self.conn.execute("DELETE FROM playbook_strategies WHERE id=?1", params![id])?;
        Ok(())
    }

    /// One-time migration: update Zebra playbooks from any spread_type → 'zebra'.
    /// Uses Rust-side name check for reliability regardless of case/spacing.
    pub fn migrate_zebra_type(&self) -> Result<()> {
        let pbs = self.get_all_playbooks()?;
        for pb in pbs {
            if pb.name.to_lowercase().contains("zebra")
                && pb.spread_type.as_deref() != Some("zebra")
            {
                self.conn.execute(
                    "UPDATE playbook_strategies SET spread_type = 'zebra' WHERE id = ?1",
                    params![pb.id],
                )?;
            }
        }
        Ok(())
    }

    /// Seed the Ratio Spread playbook if it does not already exist.
    pub fn ensure_ratio_spread_playbook(&self) -> Result<()> {
        let count: i64 = self.conn
            .query_row(
                "SELECT COUNT(*) FROM playbook_strategies WHERE name = 'Ratio Spread'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let thesis = build_ratio_spread_thesis();
        if count > 0 {
            // Upgrade old plain-prose format (pre-bullet) to bullet format
            let old_desc: Option<String> = self.conn.query_row(
                "SELECT description FROM playbook_strategies WHERE name = 'Ratio Spread'",
                [],
                |row| row.get(0),
            ).ok().flatten();
            let needs_upgrade = old_desc
                .as_deref()
                .map(|d| !d.contains("\n• "))
                .unwrap_or(false);
            if needs_upgrade {
                self.conn.execute(
                    "UPDATE playbook_strategies SET description = ?1 WHERE name = 'Ratio Spread'",
                    rusqlite::params![thesis],
                )?;
            }
            return Ok(());
        }

        use crate::models::{EntryCriteria, PlaybookStrategy};
        let ec = EntryCriteria {
            min_ivr:            Some(30.0),
            max_ivr:            Some(100.0),
            min_delta:          Some(0.20),
            max_delta:          Some(0.40),
            min_dte:            Some(15),
            max_dte:            Some(45),
            max_allocation_pct: Some(5.0),
            target_profit_pct:  Some(50.0),
            management_rule:    Some("profit_target_50".to_string()),
            min_pop:            None,
            vix_min:            None,
            vix_max:            None,
            max_bpr_pct:        None,
            stop_loss_pct:      None,
            profit_target_pct:  None,
            dte_exit:           None,
            when_to_avoid:      None,
            notes:              None,
        };
        let pb = PlaybookStrategy {
            id:             0,
            name:           "Ratio Spread".to_string(),
            description:    Some(thesis.to_string()),
            spread_type:    Some("custom".to_string()),
            entry_criteria: Some(ec),
        };
        self.insert_playbook(&pb)?;
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // Settings key-value store
    // ────────────────────────────────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn set_bpr(&self, trade_id: i32, bpr: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE trades SET bpr = ?1 WHERE id = ?2",
            rusqlite::params![bpr, trade_id],
        )?;
        Ok(())
    }

    pub fn get_all_playbooks(&self) -> Result<Vec<PlaybookStrategy>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, spread_type, entry_criteria_json
             FROM playbook_strategies
             ORDER BY id"
        )?;

        let pb_iter = stmt.query_map([], |row| {
            let criteria_json: String = row.get(4)?;
            Ok(PlaybookStrategy {
                id:             row.get(0)?,
                name:           row.get(1)?,
                description:    row.get(2)?,
                spread_type:    row.get(3)?,
                entry_criteria: serde_json::from_str(&criteria_json).ok().flatten(),
            })
        })?;

        let mut playbooks = Vec::new();
        for pb in pb_iter {
            playbooks.push(pb?);
        }
        Ok(playbooks)
    }

    /// Compute self-relative IVR for a trade: percentile of `current_iv` within
    /// min/max implied_volatility for the same ticker in our own database.
    /// Returns None if there is no data or if min == max (can't rank a single point).
    pub fn compute_ivr_for_ticker(&self, ticker: &str, current_iv: f64) -> Result<Option<f64>> {
        let mut stmt = self.conn.prepare(
            "SELECT MIN(implied_volatility), MAX(implied_volatility) \
             FROM trades \
             WHERE ticker = ?1 AND implied_volatility IS NOT NULL"
        )?;
        let row: Option<(Option<f64>, Option<f64>)> = stmt.query_row(
            rusqlite::params![ticker],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;
        match row {
            Some((Some(min_iv), Some(max_iv))) if max_iv > min_iv => {
                let ivr: f64 = ((current_iv - min_iv) / (max_iv - min_iv) * 100.0).clamp(0.0, 100.0);
                Ok(Some(ivr))
            }
            _ => Ok(None),
        }
    }

    /// Backfill iv_rank for all trades that have implied_volatility but no iv_rank.
    /// Returns the count of trades updated. Safe to call at every startup.
    pub fn backfill_ivr_all_trades(&self) -> Result<usize> {
        let candidates: Vec<(i32, String, f64)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, ticker, implied_volatility \
                 FROM trades \
                 WHERE implied_volatility IS NOT NULL AND iv_rank IS NULL"
            )?;
            let rows = stmt.query_map(rusqlite::params![], |r| {
                Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut count = 0usize;
        for (id, ticker, iv) in candidates {
            if let Ok(Some(ivr)) = self.compute_ivr_for_ticker(&ticker, iv) {
                self.conn.execute(
                    "UPDATE trades SET iv_rank = ?1 WHERE id = ?2",
                    rusqlite::params![ivr, id],
                )?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Seed the Put Broken Wing Butterfly playbook if it does not already exist.
    pub fn ensure_put_bwb_playbook(&self) -> Result<()> {
        let count: i64 = self.conn
            .query_row(
                "SELECT COUNT(*) FROM playbook_strategies WHERE name = 'Put Broken Wing Butterfly'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if count > 0 {
            // Upgrade spread_type from 'custom' to 'put_broken_wing_butterfly' if needed
            let _ = self.conn.execute(
                "UPDATE playbook_strategies SET spread_type = 'put_broken_wing_butterfly' WHERE name = 'Put Broken Wing Butterfly' AND (spread_type IS NULL OR spread_type = 'custom')",
                [],
            );
            return Ok(());
        }
        use crate::models::{EntryCriteria, PlaybookStrategy};
        let ec = EntryCriteria {
            min_ivr:            Some(30.0),
            max_ivr:            Some(100.0),
            min_delta:          None,
            max_delta:          None,
            min_dte:            Some(15),
            max_dte:            Some(45),
            max_allocation_pct: Some(5.0),
            target_profit_pct:  Some(50.0),
            management_rule:    Some("profit_target_50".to_string()),
            min_pop:            Some(60.0),
            vix_min:            None,
            vix_max:            None,
            max_bpr_pct:        None,
            stop_loss_pct:      None,
            profit_target_pct:  Some(50.0),
            dte_exit:           Some(21),
            when_to_avoid:      Some("Low IV environments; stocks without put skew".to_string()),
            notes:              Some("Enter for a net credit. Wider short spread than long spread is required. Check put skew before entry.".to_string()),
        };
        let pb = PlaybookStrategy {
            id:             0,
            name:           "Put Broken Wing Butterfly".to_string(),
            description:    Some(build_put_bwb_thesis().to_string()),
            spread_type:    Some("put_broken_wing_butterfly".to_string()),
            entry_criteria: Some(ec),
        };
        self.insert_playbook(&pb)?;
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Helper: parse DateTime from various string formats
// ────────────────────────────────────────────────────────────────────────────

fn build_ratio_spread_thesis() -> &'static str {
    "\
Overview:
• Omnidirectional undefined-risk trade: long put spread funded by an extra short put, entered for a net credit.
• No risk to the upside; max profit is at the short strike. Also known as a put ratio spread.

Setup:
• 1. Buy an ATM or OTM put (long leg)
• 2. Sell two further OTM puts for a net credit
• Max Profit: Width of Long Spread + Credit Received
• Max Loss: Breakeven Price × 100 (undefined to the downside)
• Profit Target: 50% of Credit Received or 25% of Long Spread Width
• Breakeven: Short Put Strike – (Long Spread Width + Credit Received)

Greeks:
• Delta: Long / Dynamic
• Vega: Short (benefits from vol contraction)
• Theta: Long (benefits from time decay)
• Gamma: Dynamic

How The Trade Works:
• Ideal: The stock moves toward the short strikes near expiration. Ratio spreads are omnidirectional — they profit from a stock price increase or a move down toward the short strikes. Max profit occurs when the long spread is fully ITM exactly at the short strike.
• Not Ideal: The stock moves well below the short strikes and through our breakeven. This pushes us into the loss zone because the uncovered short put has undefined downside risk.

Defensive Tactics:
• The naked short put is where the risk lives. Rolling it out in time for a credit adds extrinsic value and more time without adding risk.
• The long put spread will be near max value when we are seeing losses — it can be closed for an additional credit against the remaining short put.

Volatility:
• If Volatility Expands: We may hold. An extrinsic value loss is possible if paired with a bearish move in price. Extrinsic value goes to zero by expiration regardless.
• If Volatility Contracts: The spread can lose value when contraction pairs with a bullish move. Options: close for a profit, or buy an OTM put to convert to a symmetrical butterfly. If the put costs less than the original credit, we lock in a guaranteed profit and remove buying power risk.

Expiration:
• If Partially ITM: We can likely sell the long put spread for a profit. Consider closing the whole trade.
• If ITM: Close the long put spread to secure value, then roll or close the remaining short put.
• If OTM: All strikes expire worthless. Keep the credit received on entry as profit.

Takeaways:
• We need extrinsic value close to zero before realizing intrinsic value on the long spread. Moving ITM too early can produce extrinsic losses even when inside the max-profit zone. Hold ratio spreads closer to expiration to avoid passing through the profit zone before hitting the loss zone.
• For earnings plays, use the weekly cycle — we need the stock to move toward our spread AND extrinsic value to be near zero to capture the long spread's intrinsic value."
}

fn build_put_bwb_thesis() -> &'static str {
    "\
Omnidirectional, defined risk trade: long put spread with a wider OTM short put spread to finance the trade for a net credit. No risk to the upside; max profit at the short strikes.

I. CORE MECHANICS:
• Directional Assumption: Omnidirectional
• IV Environment: High (benefits from elevated premiums)
• Days to Expiration: 15 to 45
• Probability of Profit: 60% to 80%

II. SETUP:
• 1. Buy 1 ATM/OTM put (long anchor)
• 2. Sell 2 further OTM puts (short spread — the financing leg)
• 3. Buy 1 much further OTM put (long wing — defines the risk)
• Net Credit: The OTM short put spread is wider than the long put spread, generating a net credit.
• Example: With XYZ at $100 — Buy 100 put, Sell two 97 puts, Buy 91 put for a small credit.

III. FINANCIAL PROFILE:
• Max Profit: Width of Long Spread + Credit Received (achieved when stock pins at short strikes)
• Max Loss: Short Spread Width – Long Spread Width – Credit Received (defined, to the downside only)
• Profit Target: 50% of Credit Received or 25% of Long Spread Width
• Breakeven: Short Put Strike – (Long Spread Width + Credit Received)
• No risk to the upside if entered for a credit.

IV. THE GREEKS:
• Delta: Long / Dynamic (benefits from downward move toward short strikes)
• Vega: Short (benefits from IV contraction)
• Theta: Long (benefits from time decay)
• Gamma: Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS:
• Ideal: Stock moves toward the short strikes near expiration. The long put spread realizes max value, the short put spread expires worthless, and we keep the full credit. The spread can also profit on an upside move since it was entered for a credit.
• Not Ideal: Spread moves fully ITM — max loss. If stock moves too quickly toward the spread, extrinsic value loss can erode profits since the bulk of the potential P&L requires extrinsic value to be low.
• Remove Risk Early: If the spread moves further OTM, roll into a symmetrical butterfly for a debit less than the original credit — locks in a small guaranteed profit and removes initial risk.
• If Long Spread Is ITM Near Max Value: Sell it out to retain that value, then either hold the credit spread or adjust the trade into an iron condor.
• If Volatility Expands: Limited vega exposure due to defined risk, but could result in an extrinsic value marked loss.
• If Volatility Contracts: Easier to \"fly off\" risk by rolling to a symmetrical butterfly for a debit less than the credit received.
• If Partially ITM at Expiration: Sell for a profit if in the profit zone — long spread increases in value, short spread decreases.
• If ITM at Expiration: At max loss. Close to avoid assignment.
• If OTM at Expiration: All strikes expire worthless — keep the full credit as profit.

Takeaways:
• Put BWBs are ideal in products with put skew — skew lets us make them wider or collect a larger credit upfront.
• BWBs don't appreciate in value much until close to expiration when extrinsic value approaches zero. Primary goal: remove risk early by rolling to a symmetrical butterfly if the spread moves further OTM for a debit less than the original credit — lock in a small profit and eliminate initial risk."
}

fn parse_dt(s: String) -> Option<DateTime<Utc>> {
    // Try RFC3339 first (most common from the import scripts)
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Try YYYY-MM-DD (date-only)
    if let Ok(nd) = chrono::NaiveDate::parse_from_str(s.trim_end_matches('Z').split('T').next().unwrap_or(&s), "%Y-%m-%d") {
        let ndt = nd.and_hms_opt(0, 0, 0)?;
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
    }
    None
}
