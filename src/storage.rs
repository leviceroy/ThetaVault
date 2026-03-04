use rusqlite::{params, Connection, Result};
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
                iv_at_close=?45, delta_at_close=?46, roll_count=?47
             WHERE id=?48",
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
                id,                                     // 48
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
                iv_at_close, delta_at_close, roll_count
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
                ?45, ?46, ?47
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
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
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
               iv_at_close, delta_at_close, roll_count
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
        if count > 0 {
            return Ok(());
        }

        let thesis = "\
Overview:
Omnidirectional undefined-risk trade: long put spread funded by an extra short put, entered for a net credit. No risk to the upside; max profit is at the short strike. Also known as a put ratio spread.

Setup:
1. Buy an ATM or OTM put (long leg)
2. Sell two further OTM puts for a net credit

Max Profit: Width of Long Spread + Credit Received
Max Loss: Breakeven Price × 100 (undefined to the downside)
Profit Target: 50% of Credit Received or 25% of Long Spread Width
Breakeven: Short Put Strike – (Long Spread Width + Credit Received)

Greeks:
Delta: Long / Dynamic
Vega: Short (benefits from vol contraction)
Theta: Long (benefits from time decay)
Gamma: Dynamic

How The Trade Works:
Ideal: The stock moves toward the short strikes near expiration. Ratio spreads are omnidirectional — they profit from a stock price increase or a move down toward the short strikes. Max profit occurs when the long spread is fully ITM exactly at the short strike.

Not Ideal: The stock moves well below the short strikes and through our breakeven. This pushes us into the loss zone because the uncovered short put has undefined downside risk.

Defensive Tactics:
The naked short put is where the risk lives. Rolling it out in time for a credit adds extrinsic value and more time without adding risk. The long put spread will be near max value when we are seeing losses — it can be closed for an additional credit against the remaining short put.

Volatility:
If Volatility Expands: We may hold. An extrinsic value loss is possible if paired with a bearish move in price. Extrinsic value goes to zero by expiration regardless.

If Volatility Contracts: The spread can lose value when contraction pairs with a bullish move. Options: close for a profit, or buy an OTM put to convert to a symmetrical butterfly. If the put costs less than the original credit, we lock in a guaranteed profit and remove buying power risk.

Expiration:
If Partially ITM: We can likely sell the long put spread for a profit. Consider closing the whole trade.

If ITM: Close the long put spread to secure value, then roll or close the remaining short put.

If OTM: All strikes expire worthless. Keep the credit received on entry as profit.

Takeaways:
We need extrinsic value close to zero before realizing intrinsic value on the long spread. Moving ITM too early can produce extrinsic losses even when inside the max-profit zone. Hold ratio spreads closer to expiration to avoid passing through the profit zone before hitting the loss zone.

For earnings plays, use the weekly cycle — we need the stock to move toward our spread AND extrinsic value to be near zero to capture the long spread's intrinsic value.";

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
}

// ────────────────────────────────────────────────────────────────────────────
// Helper: parse DateTime from various string formats
// ────────────────────────────────────────────────────────────────────────────

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
