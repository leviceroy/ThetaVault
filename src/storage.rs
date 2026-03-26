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
        let _ = writeln!(w, "id,ticker,strategy,trade_date,expiration_date,quantity,credit_received,debit_paid,pnl,bpr,entry_dte,dte_at_close,exit_date,exit_reason,delta,theta,gamma,vega,pop,underlying_price,iv_rank,vix_at_entry,implied_volatility,underlying_price_at_close,iv_at_close,commission,target_profit_pct,management_rule,trade_grade,grade_notes,notes,close_notes,tags,is_earnings_play,is_tested,bid_ask_spread_at_entry,fill_vs_mid,was_assigned,assigned_shares,cost_basis,roll_count,rolled_from_id,playbook_id,next_earnings,sector,closed_at_target,iv_percentile,back_month_expiration");
        for t in &trades {
            let row = format!("{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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
                csv_field(t.sector.as_deref().unwrap_or("")),
                if t.closed_at_target { "1" } else { "0" },
                t.iv_percentile.map_or(String::new(), |v| format!("{:.1}", v)),
                t.back_month_expiration.map_or(String::new(), |d| d.format("%Y-%m-%d").to_string()),
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

    /// Migration: assign correct pzbr/czbr spread_type to Zebra playbooks.
    /// Handles old "zebra", "call_zebra", and untagged zebra-named playbooks.
    pub fn migrate_zebra_type(&self) -> Result<()> {
        let pbs = self.get_all_playbooks()?;
        for pb in pbs {
            let name_lc = pb.name.to_lowercase();
            let current = pb.spread_type.as_deref().unwrap_or("");
            let is_call_zebra = name_lc.contains("call zebra") || name_lc.contains("call_zebra") || current == "call_zebra" || current == "czbr";
            let is_any_zebra  = name_lc.contains("zebra") || current == "zebra" || current == "call_zebra" || current == "pzbr" || current == "czbr";
            if !is_any_zebra { continue; }
            let target = if is_call_zebra { "czbr" } else { "pzbr" };
            if current != target {
                self.conn.execute(
                    "UPDATE playbook_strategies SET spread_type = ?1 WHERE id = ?2",
                    params![target, pb.id],
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

    /// Seed all default playbook strategies that don't already exist.
    /// Called at startup so a fresh clone has a full playbook out of the box.
    pub fn ensure_default_playbooks(&self) -> Result<()> {
        use crate::models::EntryCriteria;

        struct Def {
            name:        &'static str,
            spread_type: &'static str,
            desc:        &'static str,
            ec:          Option<EntryCriteria>,
        }

        let defs: &[Def] = &[
            Def {
                name: "Short Put Vertical",
                spread_type: "short_put_vertical",
                desc: build_short_put_vertical_desc(),
                ec: Some(EntryCriteria {
                    min_ivr: Some(30.0), max_ivr: None,
                    min_delta: Some(16.0), max_delta: Some(30.0),
                    min_dte: Some(10), max_dte: Some(45),
                    max_allocation_pct: Some(2.0),
                    target_profit_pct: Some(50.0),
                    management_rule: Some("21 DTE EXIT".to_string()),
                    min_pop: Some(60.0), vix_min: None, vix_max: None,
                    max_bpr_pct: Some(5.0), notes: None,
                    ..Default::default()
                }),
            },
            Def {
                name: "Cash Secured Put",
                spread_type: "cash_secured_put",
                desc: build_cash_secured_put_desc(),
                ec: Some(EntryCriteria {
                    target_profit_pct: Some(85.0),
                    ..Default::default()
                }),
            },
            Def {
                name: "Covered Call",
                spread_type: "covered_call",
                desc: build_covered_call_desc(),
                ec: Some(EntryCriteria {
                    target_profit_pct: Some(85.0),
                    ..Default::default()
                }),
            },
            Def {
                name: "Iron Condor",
                spread_type: "iron_condor",
                desc: build_iron_condor_desc(),
                ec: Some(EntryCriteria {
                    min_ivr: Some(30.0), max_ivr: None,
                    min_delta: Some(16.0), max_delta: Some(30.0),
                    min_dte: Some(10), max_dte: Some(45),
                    max_allocation_pct: Some(2.0),
                    target_profit_pct: Some(50.0),
                    management_rule: Some("21 DTE EXIT".to_string()),
                    min_pop: Some(60.0), vix_min: None, vix_max: None,
                    max_bpr_pct: Some(5.0),
                    notes: Some("Low IV".to_string()),
                    ..Default::default()
                }),
            },
            Def {
                name: "Short Call Vertical",
                spread_type: "short_call_vertical",
                desc: build_short_call_vertical_desc(),
                ec: Some(EntryCriteria {
                    min_delta: Some(16.0), max_delta: Some(30.0),
                    min_dte: Some(10), max_dte: Some(45),
                    max_allocation_pct: Some(2.0),
                    target_profit_pct: Some(50.0),
                    management_rule: Some("21 DTE EXIT".to_string()),
                    min_pop: Some(65.0),
                    ..Default::default()
                }),
            },
            Def {
                name: "Calendar Spread",
                spread_type: "calendar_spread",
                desc: build_calendar_spread_desc(),
                ec: Some(EntryCriteria {
                    min_ivr: Some(25.0), max_ivr: Some(45.0),
                    min_delta: Some(10.0), max_delta: Some(20.0),
                    max_allocation_pct: Some(1.0),
                    target_profit_pct: Some(25.0),
                    management_rule: Some("Before Expiry".to_string()),
                    ..Default::default()
                }),
            },
            Def {
                name: "Short Strangle",
                spread_type: "strangle",
                desc: build_short_strangle_desc(),
                ec: None,
            },
            Def {
                name: "Iron Butterfly (Iron Fly)",
                spread_type: "iron_butterfly",
                desc: build_iron_fly_desc(),
                ec: Some(EntryCriteria {
                    target_profit_pct: Some(25.0),
                    ..Default::default()
                }),
            },
            Def {
                name: "Short Diagonal Spread",
                spread_type: "short_diagonal_spread",
                desc: build_short_diagonal_desc(),
                ec: None,
            },
            Def {
                name: "Put ZEBRA",
                spread_type: "pzbr",
                desc: build_put_zebra_desc(),
                ec: None,
            },
            Def {
                name: "Call ZEBRA",
                spread_type: "czbr",
                desc: build_call_zebra_desc(),
                ec: None,
            },
            Def {
                name: "Poor Man's Covered Call",
                spread_type: "pmcc",
                desc: build_pmcc_desc(),
                ec: None,
            },
            Def {
                name: "Long Diagonal Spread",
                spread_type: "long_diagonal_spread",
                desc: build_long_diagonal_desc(),
                ec: None,
            },
            Def {
                name: "Long Put Vertical Spread",
                spread_type: "long_put_vertical",
                desc: build_long_put_vertical_desc(),
                ec: Some(EntryCriteria {
                    min_dte: Some(30), max_dte: Some(45),
                    target_profit_pct: Some(50.0),
                    ..Default::default()
                }),
            },
            Def {
                name: "Long Call Vertical Spread",
                spread_type: "long_call_vertical",
                desc: build_long_call_vertical_desc(),
                ec: Some(EntryCriteria {
                    min_ivr: Some(30.0), max_ivr: Some(100.0),
                    min_delta: Some(16.0), max_delta: Some(30.0),
                    min_dte: Some(30), max_dte: Some(45),
                    max_allocation_pct: Some(1.0),
                    target_profit_pct: Some(50.0),
                    management_rule: Some("dte_exit_21".to_string()),
                    ..Default::default()
                }),
            },
        ];

        for def in defs {
            let count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM playbook_strategies WHERE name = ?1",
                params![def.name],
                |row| row.get(0),
            )?;
            if count > 0 { continue; }

            let criteria_json = def.ec.as_ref()
                .and_then(|ec| serde_json::to_string(ec).ok())
                .unwrap_or_else(|| "null".to_string());
            self.conn.execute(
                "INSERT INTO playbook_strategies (name, description, spread_type, entry_criteria_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![def.name, def.desc, def.spread_type, criteria_json],
            )?;
        }
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

fn build_short_put_vertical_desc() -> &'static str {
    "Neutral-Bullish defined risk credit trade betting against the stock moving below the short strike. Profits from the stock rising, staying flat, or decaying time.

I. CORE MECHANICS
\u{2022} Directional Assumption: Neutral-Bullish
\u{2022} IV Environment: High (Maximizes credit)
\u{2022} Ideal Expiration: 45 Days to Expiration (DTE)
\u{2022} Probability of Profit (POP): 60% to 80%

II. SETUP
1. Sell 1 OTM/ATM Put.
2. Buy 1 further OTM Put (defines max risk).
\u{2022} Goal: Collect ~1/3rd the width of the strikes in credit.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Credit Received.
\u{2022} Max Loss: Distance Between Strikes - Credit Received.
\u{2022} Breakeven: Short Put Strike - Credit Received.
\u{2022} Profit Target: 50% of Max Profit.

IV. THE GREEKS
\u{2022} Delta: Long (Profits as stock rises)
\u{2022} Theta: Long (Profits from time decay)
\u{2022} Vega: Short (Hurt by volatility expansion)
\u{2022} Gamma: Flat

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock rises or stays above the short strike, time passes, and volatility contracts.
\u{2022} Rolling Out: If the spread is tested, roll out to a farther expiration for a credit to add time, reduce max loss, and increase potential profit.
\u{2022} Early Profit: Close at 50% of max profit to secure gains and free up capital.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Spread expires worthless. Close early to remove gap risk.
\u{2022} If ITM: Close the trade before expiration to realize max loss and avoid assignment fees.
\u{2022} If Partially ITM: (Between strikes) Roll out or close. Never let a tested spread go through expiration to avoid unwanted share assignment."
}

fn build_cash_secured_put_desc() -> &'static str {
    "Bullish/Neutral strategy where we sell an OTM put and secure the potential assignment with cash. Used to generate income or acquire stock at a lower cost basis.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bullish / Neutral
\u{2022} IV Environment: High (Ideal for maximizing premium)
\u{2022} Ideal Expiration: 45 Days to Expiration (DTE)
\u{2022} Probability of Profit (POP): 60% to 80%

II. SETUP
1. Identify a stock you are willing to own at a lower price.
2. Sell 1 OTM Put at ~30 Delta (Standard) or lower for more safety.
3. Ensure cash (Strike x 100) is reserved in the account to \"secure\" the position.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Premium (Credit) Received.
\u{2022} Max Loss: (Short Strike - Credit Received) x 100 (Stock goes to zero).
\u{2022} Breakeven: Short Strike - Credit Received.
\u{2022} Profit Target: 50% of Max Profit.

IV. THE GREEKS
\u{2022} Delta: Long (Profits as stock rises)
\u{2022} Theta: Long (Profits from time decay)
\u{2022} Vega: Short (Hurt by volatility expansion)
\u{2022} Gamma: Flat / Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock stays above the short strike or rises. Put loses value through theta and is closed for a profit.
\u{2022} Managing Winners: Close at 50% of max profit or 21 DTE to reduce gamma risk and increase velocity of money.
\u{2022} Defensive Tactics: If the strike is tested, roll the position out in time for a net credit to buy time and reduce effective cost basis.
\u{2022} Transition: If assigned, you now own 100 shares. Transition to selling Covered Calls (The \"Wheel\").

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Put expires worthless. Keep the full premium and deploy another CSP if desired.
\u{2022} If ITM: Assigned 100 shares of stock at the strike price. Your effective entry price is the Breakeven."
}

fn build_covered_call_desc() -> &'static str {
    "Bullish stock position where we sell an ATM/OTM call against 100 long shares to reduce cost basis. The short call risk is \"covered\" by the long shares.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bullish
\u{2022} IV Environment: High (Ideal for maximizing premium)
\u{2022} Ideal Expiration: 45 Days to Expiration (DTE)
\u{2022} Probability of Profit (POP): 50% to 70%

II. SETUP
1. Buy 100 shares of the underlying stock.
2. Sell 1 ATM/OTM Call for every 100 shares owned.

III. FINANCIAL PROFILE
\u{2022} Max Profit: (Distance Between Stock Purchase & Short Call Strike) + Credit Received.
\u{2022} Max Loss: Stock Purchase Price - Credit Received (Stock going to zero).
\u{2022} Breakeven: Stock Purchase Price - Credit Received.
\u{2022} Profit Target: 50% of Max Profit.

IV. THE GREEKS
\u{2022} Delta: Long (Profit as stock rises)
\u{2022} Theta: Long (Profit from time decay)
\u{2022} Vega: Short (Hurt by volatility expansion)
\u{2022} Gamma: Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock moves up to the short call strike by expiration. Capture max extrinsic value + full stock gain.
\u{2022} Rolling for Credit: If the short call loses value, roll it out in time to add more extrinsic value and further reduce cost basis.
\u{2022} Rolling Down: If the stock drops, move the call strike down within the same cycle to collect more credit (avoid rolling below your breakeven).
\u{2022} Assignment Defense: If you want to keep the shares, roll the short call out and up BEFORE it goes ITM.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Call expires worthless. Deploy another call in a further expiration cycle to continue reducing basis.
\u{2022} If ITM: Stock is \"Called Away\" (exercised). The position is closed, and you realize max profit."
}

fn build_iron_condor_desc() -> &'static str {
    "Neutral, defined risk strategy consisting of an OTM put credit spread and OTM call credit spread. We profit from the underlying staying between our short strikes through expiration.

I. CORE MECHANICS
\u{2022} Directional Assumption: Neutral
\u{2022} IV Environment: High (Best for maximizing credit)
\u{2022} Ideal Expiration: 45 Days to Expiration (DTE)
\u{2022} Probability of Profit (POP): 60% to 80%

II. SETUP
1. Sell 1 OTM Put Spread (Short Put + Long Put).
2. Sell 1 OTM Call Spread (Short Call + Long Call).
\u{2022} Goal: Collect ~1/3rd the width of the strikes in total credit.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Total Credit Received.
\u{2022} Max Loss: Width of Widest Spread - Credit Received.
\u{2022} Breakeven (Lower): Short Put Strike - Total Credit.
\u{2022} Breakeven (Upper): Short Call Strike + Total Credit.
\u{2022} Profit Target: 50% of Max Profit.

IV. THE GREEKS
\u{2022} Delta: Flat (Neutral)
\u{2022} Theta: Long (Profit from time decay on both sides)
\u{2022} Vega: Short (Hurt by volatility expansion)
\u{2022} Gamma: Flat

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Underlying stays between short strikes as time passes, allowing both sides to decay simultaneously.
\u{2022} Rolling the Untested Side: If one side is tested, roll the other side closer to the price to collect more credit and reduce max loss.
\u{2022} Extending Duration: Roll the entire spread (or the tested side) out in time for a credit to buy more time.
\u{2022} Volatility Play: If IV contracts, close for a winner even if price is near a strike.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Both spreads expire worthless. Close early to remove gap risk and secure profit.
\u{2022} If ITM: Close the losing spread before expiration to avoid assignment fees and after-hours risk."
}

fn build_short_call_vertical_desc() -> &'static str {
    "Neutral-Bearish defined risk credit trade betting against the stock moving above the short strike. Profits from the stock falling, staying flat, or decaying time.

I. CORE MECHANICS
\u{2022} Directional Assumption: Neutral-Bearish
\u{2022} IV Environment: High (Maximizes credit)
\u{2022} Ideal Expiration: 45 Days to Expiration (DTE)
\u{2022} Probability of Profit (POP): 60% to 80%

II. SETUP
1. Sell 1 ATM/OTM Call.
2. Buy 1 further OTM Call (defines max risk).
\u{2022} Goal: Collect ~1/3rd the width of the strikes in credit.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Credit Received.
\u{2022} Max Loss: Distance Between Strikes - Credit Received.
\u{2022} Breakeven: Short Call Strike + Credit Received.
\u{2022} Profit Target: 50% of Max Profit.

IV. THE GREEKS
\u{2022} Delta: Short (Profits as stock falls)
\u{2022} Theta: Long (Profits from time decay)
\u{2022} Vega: Short (Hurt by volatility expansion)
\u{2022} Gamma: Flat

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock falls or stays below the short strike, time passes, and volatility contracts.
\u{2022} Rolling Out: If the spread is tested, roll out to a farther expiration for a credit to add time, reduce max loss, and increase potential profit.
\u{2022} Early Profit: Close at 50% of max profit to secure gains and free up capital.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Both options expire worthless. Close early to remove gap risk.
\u{2022} If ITM: Close the trade before expiration to realize max loss and avoid assignment.
\u{2022} If Partially ITM: (Between strikes) Either close the trade or roll out in time. Never let a tested spread go through expiration to avoid unwanted short stock position."
}

fn build_calendar_spread_desc() -> &'static str {
    "Neutral, defined risk trade where we bet on an increase in IV or the stock staying stagnant near our strikes so the short premium decays faster than the long premium.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bullish (for Call Calendars)
\u{2022} IV Environment: Low (Ideally IV expands)
\u{2022} Ideal Expiration: 45 Days (Short leg)
\u{2022} Probability of Profit (POP): N/A

II. SETUP
1. Buy a Call/Put in a long-term expiration cycle.
2. Sell a Call/Put in a near-term expiration cycle at the SAME strike.
\u{2022} Goal: Enter for a net debit.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Variable (Maximum when stock is at strike at front-month expiration).
\u{2022} Max Loss: Debit Paid.
\u{2022} Breakeven: Variable.
\u{2022} Profit Target: 10-25% of Debit Paid.

IV. THE GREEKS
\u{2022} Delta: Long (for Call Calendars)
\u{2022} Theta: Short (Note: Front month decays faster, but strategy captures extrinsic spread)
\u{2022} Vega: Long (Benefits from IV expansion)
\u{2022} Gamma: Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock trickles up to the strike over time. Long option expands while short option contracts.
\u{2022} Defensive Tactics: If the short option loses value, roll it out in time closer to the long option. This reduces net debit and max loss.
\u{2022} Volatility Play: If IV expands, the trade sees profit as long as it is not paired with a sharp move away from the strike.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Short option expires worthless. Hold long option or roll short option to a new cycle to reduce cost basis.
\u{2022} If ATM: Highest potential profit spot. Short option decays completely leaving remaining extrinsic in the long option.
\u{2022} If ITM: Risk of assignment. Short call converts to 100 short shares (Short put to long shares). Close or roll to avoid buying power spikes."
}

fn build_short_strangle_desc() -> &'static str {
    "Neutral, undefined risk strategy consisting of an OTM short put and an OTM short call. We want the stock to stay between our strikes through expiration so the options expire worthless and we keep the credit received up front as profit.

I. CORE MECHANICS
\u{2022} Directional Assumption: Neutral
\u{2022} IV Environment: High (Ideal for maximizing credit)
\u{2022} Ideal Expiration: 45 Days to Expiration (DTE)
\u{2022} Probability of Profit (POP): 60% to 80%

II. SETUP
1. Sell 1 OTM Put.
2. Sell 1 OTM Call.
\u{2022} Note: We do not aim for a specific target credit but trust the premium will be sufficient if the market is liquid.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Total Credit Received.
\u{2022} Max Loss: Unlimited.
\u{2022} Breakeven (Lower): Put Strike - Credit Received.
\u{2022} Breakeven (Upper): Call Strike + Credit Received.
\u{2022} Profit Target: 50% of Max Profit.

IV. THE GREEKS
\u{2022} Delta: Flat (Neutralized by two-way selling)
\u{2022} Theta: Positive (Time decay works on both sides)
\u{2022} Vega: Short (Hurt by volatility expansion)
\u{2022} Gamma: Short (Dangerous near expiration)

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: The stock stays between our strikes as time passes. This results in extrinsic value decay on both sides and the trade can be bought back for a profit over time.
\u{2022} Defensive Tactics: Strangles are undefined risk trades and can be adjusted very easily. If the stock moves towards or past one of our strikes, we can roll the other \"untested\" side closer to the \"tested\" side to pick up additional credit and reduce the delta of the position. We can also roll both strikes out in time to add more credit, or a combination of both.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM at Expiration: Both strikes will expire worthless and we will realize max profit.
\u{2022} If ITM at Expiration: We can roll the strikes out in time to add credit and duration to the trade, or close the trade if our assumption has changed."
}

fn build_iron_fly_desc() -> &'static str {
    "Neutral, defined risk strategy consisting of an ATM put credit spread and ATM call credit spread. We profit from the underlying staying between our breakeven prices through expiration.

I. CORE MECHANICS
\u{2022} Directional Assumption: Neutral
\u{2022} IV Environment: High (Ideal for maximizing credit)
\u{2022} Ideal Expiration: 45 Days to Expiration (DTE)
\u{2022} Probability of Profit (POP): 60% to 80%

II. SETUP
1. Sell an ATM Straddle (ATM Put + ATM Call).
2. Buy an OTM Put wing.
3. Buy an OTM Call wing.
\u{2022} Goal: Collect a large credit, ideally targeting a 1:1 risk/reward ratio.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Total Credit Received.
\u{2022} Max Loss: Widest Spread Width - Total Credit Received.
\u{2022} Breakeven (Lower): ATM Strike - Total Credit Received.
\u{2022} Breakeven (Upper): ATM Strike + Total Credit Received.
\u{2022} Profit Target: 25% of Max Profit.

IV. THE GREEKS
\u{2022} Delta: Flat (Neutral)
\u{2022} Theta: Long (Significant time decay at ATM strikes)
\u{2022} Vega: Short (Hurt by volatility expansion)
\u{2022} Gamma: Flat

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock stays between breakevens as time passes, allowing extrinsic value to collapse.
\u{2022} Early Management: Target 25% of max profit due to high gamma risk as expiration approaches.
\u{2022} Defensive Tactics: Management is limited compared to undefined risk. If the stock moves outside of your long strikes, there is little that can be done without increasing risk.

VI. EXPIRATION OUTCOMES
\u{2022} Assignment Risk: Strategy will always have at least one strike ITM at expiration. Close or roll prior to expiration to avoid assignment.
\u{2022} Partially ITM: Close for profit if the price is within the breakeven range.
\u{2022} ITM at Expiration: Close the trade to avoid assignment fees and move on."
}

fn build_short_diagonal_desc() -> &'static str {
    "Directional credit strategy where we sell a long-term OTM option and buy a short-term closer-to-the-money option for protection. A more aggressive version of a credit spread that uses time differential to increase credit.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bearish (Calls) / Bullish (Puts)
\u{2022} IV Environment: High (Ideally IV contracts)
\u{2022} Ideal Expiration: 60+ Days (Short leg) / 30 Days (Long leg)
\u{2022} Probability of Profit (POP): 60% to 75%

II. SETUP
1. Sell 1 OTM Call/Put in a back-month cycle.
2. Buy 1 OTM Call/Put in a front-month cycle at a strike closer to the money.
\u{2022} Goal: Enter for a net credit.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Net Credit Received.
\u{2022} Max Loss: Defined by strikes, but varies based on time remaining in back month.
\u{2022} Breakeven: Variable.
\u{2022} Profit Target: 50% of Credit Received.

IV. THE GREEKS
\u{2022} Delta: Short (Calls) / Long (Puts)
\u{2022} Theta: Positive (Captures accelerated back-month decay)
\u{2022} Vega: Short (Hurt by IV expansion)
\u{2022} Gamma: Flat

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Underlying stays away from short strike. Front month protection decays, and back month short decays even more.
\u{2022} Defensive Tactics: If tested, roll the front month out to match the back month, turning it into a standard vertical credit spread.

VI. EXPIRATION OUTCOMES
\u{2022} Front Month Expiry: If OTM, you are left with a naked short in the back month. Close or add new protection.
\u{2022} Back Month Expiry: Full profit realized if OTM."
}

fn build_put_zebra_desc() -> &'static str {
    "A bearish back-ratio spread where we are buying two ITM puts and selling one ATM put to remove all extrinsic value and achieve 100 negative deltas. Acts like a synthetic short stock position with limited risk.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bearish
\u{2022} IV Environment: Any
\u{2022} Ideal Expiration: Any (often 45-60 DTE)
\u{2022} Probability of Profit (POP): 50%

II. SETUP
1. Buy 2 ITM Puts (usually 70-80 Delta).
2. Sell 1 ATM Put (usually 50 Delta).
\u{2022} Goal: Zero extrinsic value on entry. The debit paid should equal the intrinsic value of the spread.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Unlimited (to the downside, until stock hits 0).
\u{2022} Max Loss: Debit Paid (Defined Risk).
\u{2022} Breakeven: Short Put Strike - Any Extrinsic Value Paid.
\u{2022} Profit Target: 25% of Debit Paid.

IV. THE GREEKS
\u{2022} Delta: Short / Dynamic (Targeting 100 negative deltas)
\u{2022} Theta: Flat (Neutralized by back-ratio)
\u{2022} Vega: Flat (Neutralized by back-ratio)
\u{2022} Gamma: Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock moves down. Position gains value at the same rate as 100 short shares without borrowing costs or unlimited risk.
\u{2022} Defensive Tactics: Roll the short put up a few strikes if the stock rallies. This will decrease the short delta exposure on a rally and reduce overall debit paid.
\u{2022} Stock Replacement: Great strategy for 100-delta short exposure with defined risk and no margin calls.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Realize max loss (debit paid upfront).
\u{2022} If ITM: Close trade for a profit.
\u{2022} If Partially ITM: Close or restructure in a later cycle. Avoid assignment by closing prior to expiration."
}

fn build_call_zebra_desc() -> &'static str {
    "A bullish back-ratio spread where we are buying two ITM calls and selling one ATM call to remove all extrinsic value and achieve 100 positive deltas. Acts like a synthetic long stock position with limited risk.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bullish
\u{2022} IV Environment: Any
\u{2022} Ideal Expiration: Any (often 45-60 DTE)
\u{2022} Probability of Profit (POP): 50%

II. SETUP
1. Buy 2 ITM Calls (usually 70-80 Delta).
2. Sell 1 ATM Call (usually 50 Delta).
\u{2022} Goal: Zero extrinsic value on entry. The debit paid should equal the intrinsic value of the spread.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Unlimited (to the upside).
\u{2022} Max Loss: Debit Paid (Defined Risk).
\u{2022} Breakeven: Short Call Strike + Any Extrinsic Value Paid.
\u{2022} Profit Target: 25% of Debit Paid.

IV. THE GREEKS
\u{2022} Delta: Long / Dynamic (Targeting 100 positive deltas)
\u{2022} Theta: Flat (Neutralized by back-ratio)
\u{2022} Vega: Flat (Neutralized by back-ratio)
\u{2022} Gamma: Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock moves up. Position gains value at the same rate as 100 shares of stock without theta decay drag.
\u{2022} Defensive Tactics: Roll the short call down to a lower strike if the stock sells off to reduce the total debit paid.
\u{2022} Stock Replacement: Great strategy for 100-delta exposure with limited risk and zero extrinsic cost.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Realize max loss (debit paid upfront).
\u{2022} If ITM: Close trade for a profit.
\u{2022} If Partially ITM: Close or restructure in a later cycle. Avoid assignment by closing prior to expiration."
}

fn build_pmcc_desc() -> &'static str {
    "A bullish synthetic covered call strategy that consists of an ITM long-term call to replicate 100 shares of stock, with an ATM/OTM short call in a near-term cycle to reduce cost basis.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bullish
\u{2022} IV Environment: Low (Ideally IV expands)
\u{2022} Ideal Expiration: 45 to 60 Days (Short leg)
\u{2022} Probability of Profit (POP): 50% to 60%

II. SETUP
1. Buy an ITM Call in a long-term expiration cycle (80+ Delta).
2. Sell an OTM Call in a near-term expiration cycle (30 Delta).
\u{2022} Golden Rule: Debit paid should not exceed 75% of the width between strikes.

III. FINANCIAL PROFILE
\u{2022} Max Profit: Distance Between Strikes - Debit Paid + Estimated Extrinsic in Long Option.
\u{2022} Max Loss: Debit Paid.
\u{2022} Breakeven: Long Call Strike + Debit Paid.
\u{2022} Profit Target: 50% of Estimated Max Profit.

IV. THE GREEKS
\u{2022} Delta: Long
\u{2022} Theta: Flat / Neutral
\u{2022} Vega: Long (Benefits from IV expansion)
\u{2022} Gamma: Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Stock moves up to the short call strike. Capture max extrinsic from short call + gain on long call.
\u{2022} Defensive Tactics: If short call loses value, roll it out in time to add extrinsic value. Can also move strike down in same cycle (but not below breakeven).
\u{2022} Volatility: If IV expands, extrinsic value moves against us temporarily but we can adjust short call if paired with selloff.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Short call expires worthless. Deploy another one in a further cycle.
\u{2022} If ITM: Max profit reached - close the trade."
}

fn build_long_diagonal_desc() -> &'static str {
    "Directional, defined risk strategy that combines a long-term ITM option with a short-term OTM option at a different strike. Effectively \"Poor Man's\" versions of covered calls or cash secured puts.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bullish (Calls) / Bearish (Puts)
\u{2022} IV Environment: Low (Ideally IV expands)
\u{2022} Ideal Expiration: LEAPS (Long leg) / 45 Days (Short leg)
\u{2022} Probability of Profit (POP): 50% to 60%

II. SETUP
1. Buy 1 ITM Call/Put in a long-term cycle (80+ Delta).
2. Sell 1 OTM Call/Put in a near-term cycle (30 Delta).
\u{2022} Golden Rule: Debit paid should be less than 75% of the width of the strikes (for Call Diagonals).

III. FINANCIAL PROFILE
\u{2022} Max Profit: Unlimited (theoretically) or Width of Strikes - Debit + Extrinsic in Long.
\u{2022} Max Loss: Debit Paid (Defined Risk).
\u{2022} Breakeven: Long Strike + Debit Paid.
\u{2022} Profit Target: 50% of Estimated Max Profit.

IV. THE GREEKS
\u{2022} Delta: Long (Calls) / Short (Puts)
\u{2022} Theta: Neutral / Positive (Front month decay offsets back month)
\u{2022} Vega: Long (Benefits from IV expansion)
\u{2022} Gamma: Dynamic

V. MANAGEMENT & DEFENSIVE TACTICS
\u{2022} Ideal Scenario: Underlying moves towards short strike. Capture max extrinsic from short leg while long leg gains intrinsic value.
\u{2022} Rolling: If short strike is tested, roll out in time for a credit to add more extrinsic value.
\u{2022} Volatility Play: If IV spikes, the long-term option gains significant value.

VI. EXPIRATION OUTCOMES
\u{2022} If OTM: Short leg expires worthless. Hold long leg or sell another short leg.
\u{2022} If ITM: Position reaches peak profit. Close the entire spread."
}

fn build_long_put_vertical_desc() -> &'static str {
    "Bearish, defined risk debit trade where we are betting on the stock moving below our short put strike price by the expiration of our contract.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bearish
\u{2022} IV Environment: Any
\u{2022} Days to Expiration: 45
\u{2022} Probability of Profit: 50% to 60%

II. SETUP
\u{2022} 1. Buy an ITM put
\u{2022} 2. Sell an ATM/OTM put
\u{2022} Max Profit: Distance Between Strikes - Debit Paid
\u{2022} Max Loss: Debit Paid
\u{2022} Profit Target: 50% of Max Profit
\u{2022} Breakeven: Long Put Strike - Debit Paid

III. GREEKS
\u{2022} Delta: Short
\u{2022} Vega: Flat
\u{2022} Theta: Flat
\u{2022} Gamma: Flat

IV. HOW THE TRADE WORKS
\u{2022} NOT IDEAL: The stock increases in value. The value of the long put spread would decrease, which means the spread will be less valuable to sell to close compared to the original purchase price, which would result in a loss.
\u{2022} IDEAL: The stock decreases in value. A long put spread is a directionally bearish position - so ideally the stock price decreases so that the long put strike increases in value to a greater degree than the short put.

V. VOLATILITY
\u{2022} IF VOLATILITY EXPANDS: Extrinsic value may have increased - but this is primarily a bearish trade and if the increase in IV is paired with a bearish move, we may see profitability and can close if we are happy with the exit price.
\u{2022} IF VOLATILITY CONTRACTS: Extrinsic value may have decreased, but this could be paired with a rally in the product. We may consider holding the position, or rolling the short option up closer to the long option, but not above the breakeven price.

VI. EXPIRATION
\u{2022} IF ITM AT EXPIRATION: The trade will be at max profit. We close the trade.
\u{2022} IF OTM AT EXPIRATION: The trade will be at max loss, as both options will have lost all of their value. We let the trade expire worthless.
\u{2022} IF PARTIALLY ITM AT EXPIRATION: We either roll out in time to extend the trade or close it. We avoid letting these trades go through expiration, because if the long put is ITM and the short put is OTM we can come back to the market the next trading session with 100 shares of short stock.

VII. TAKEAWAYS
\u{2022} 1. Vertical spreads have a less volatile P/L because of the long option that defines our risk. If we see profit on the short option, we will see losses on the long option and vice versa. For this reason, we should expect to be in spread trades longer than naked options to reach profit targets.
\u{2022} 2. With spreads, it's important to realize that options will be exercised if they are ITM and held through expiration. If one strike is ITM and the other moves OTM, close the trade prior to expiration to avoid unwanted shares."
}

fn build_long_call_vertical_desc() -> &'static str {
    "Bullish defined risk debit trade where we are betting on the stock moving above our short call strike price by the expiration of our contract.

I. CORE MECHANICS
\u{2022} Directional Assumption: Bullish
\u{2022} IV Environment: Any
\u{2022} Days to Expiration: 45
\u{2022} Probability of Profit: 40% to 60%

II. SETUP
\u{2022} 1. Buy an ITM call
\u{2022} 2. Sell an ATM/OTM call
\u{2022} Max Profit: Distance Between Strikes - Debit Paid
\u{2022} Max Loss: Debit Paid
\u{2022} Profit Target: 50% of Max Profit
\u{2022} Breakeven: Long Call Strike + Debit Paid

III. GREEKS
\u{2022} Delta: Long
\u{2022} Vega: Flat
\u{2022} Theta: Flat
\u{2022} Gamma: Flat

IV. HOW THE TRADE WORKS
\u{2022} IDEAL: The stock increases in value. A long call spread is a directionally bullish position - so ideally the stock price rises so that the long call strike increases in value to a greater degree than the short call, resulting in a profit.
\u{2022} NOT IDEAL: The stock decreases in value. The value of the long call spread would decrease, which means the spread will be less valuable to sell to close compared to the original purchase price, which would result in a loss.

V. VOLATILITY
\u{2022} IF VOLATILITY EXPANDS: We may hold the position - this may be paired with a sell-off in the stock price, but our risk is capped at the debit paid so we typically let the trade play out. However, we can close the trade if our assumption has changed.
\u{2022} IF VOLATILITY CONTRACTS: We may hold the position - if this is paired with a bullish move in the stock price, we may see profit in the spread and we can close if we're happy with the trade.

VI. EXPIRATION
\u{2022} IF OTM AT EXPIRATION: The trade will be at max loss, as both options will have lost all value. We let the trade expire worthless.
\u{2022} IF ITM AT EXPIRATION: Both options will be trading for intrinsic value, and the trade will be at max profit. To avoid assignment fees and the possibility of one of the options moving OTM, close the trade prior to expiration.
\u{2022} IF PARTIALLY ITM AT EXPIRATION: We either close the trade or roll out in time to extend it. We avoid letting these trades go through expiration because if the long call is ITM and the short call is OTM, we can come back to the market in the next trading session with 100 shares of stock.

VII. TAKEAWAYS
\u{2022} 1. Vertical spreads have a less volatile P/L because of the long option that defines our risk. For this reason, we should expect to be in spread trades longer than naked options to reach profit targets.
\u{2022} 2. Options will be exercised if they are ITM and held through expiration. If one strike is ITM and the other moves OTM, close the trade prior to expiration to avoid unwanted shares."
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
