// Copyright (c) 2025 Chris Wenk. All rights reserved.

use std::{error::Error, io};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal, widgets::{TableState, ListState}};
use theta_vault_rust::{models, storage, ui, calculations, yahoo};
use theta_vault_rust::app::{AppMode, EditField, FieldKind, FilterStatus, SortKey, VisualRowKind};
use std::collections::HashSet;
use chrono::Utc;

// ─────────────────────────────────────────────────────────────────────────────
// AppState
// ─────────────────────────────────────────────────────────────────────────────

pub struct AppState {
    // Data
    pub trades:             Vec<models::Trade>,
    pub playbooks:          Vec<models::PlaybookStrategy>,
    pub stats:              models::PortfolioStats,
    pub perf_stats:         models::PerformanceStats,
    pub playbook_analytics: Vec<models::PlaybookAnalytics>,

    // Account settings
    pub account_size:         f64,
    pub target_undefined_pct: f64,
    pub max_heat_pct:         f64,
    pub max_pos_bpr_pct:      f64,   // max BPR per position as % of account (default 5.0)
    pub monthly_pnl_target:   f64,
    pub drawdown_circuit_breaker_pct: f64, // drawdown % threshold for circuit breaker alert (default 5.0)
    pub default_profit_target_pct: f64,   // global default profit target % (0 = use per-strategy defaults)
    pub default_mgmt_dte:     i32,        // DTE at which to trigger management alerts (tastytrade default: 21)
    pub risk_free_rate_pct:   f64,        // risk-free rate % for Sharpe/Sortino (default 4.5)
    pub rolling_window:       usize,      // rolling window trade count (default 30)
    pub export_status:        Option<String>, // CSV export status message
    pub export_status_tick:   u8,         // countdown to clear export_status
    pub current_vix:          Option<f64>,
    pub beta_map:             std::collections::HashMap<String, f64>,
    pub spy_price:            Option<f64>,
    pub live_prices:          std::collections::HashMap<String, f64>,
    pub spy_monthly:          std::collections::HashMap<(i32, u32), f64>,
    pub alerts:               Vec<theta_vault_rust::actions::TradeAlert>,
    pub actions_list_state:   ListState,
    pub collapsed_action_kinds: HashSet<theta_vault_rust::actions::AlertKind>,
    pub pulse_on:             bool,

    // Runtime context
    pub under_tauri: bool,

    // KPI info popups
    pub dash_kpi_popup:          bool,
    pub dash_kpi_scroll:         u16,
    pub dash_kpi_max_scroll:     u16,
    pub perf_kpi_popup:          bool,
    pub perf_kpi_scroll:         u16,
    pub perf_kpi_max_scroll:     u16,
    pub journal_help_popup: bool,
    pub journal_help_scroll: u16,
    pub journal_help_max_scroll: u16,
    pub journal_help_page: u8,

    // Admin settings form
    pub admin_fields:    Vec<EditField>,
    pub admin_field_idx: usize,
    pub admin_scroll:    u16,

    // Navigation
    pub selected_tab:    usize,
    pub table_state:     TableState,
    pub playbook_state:  ListState,

    // Month/year grouping
    pub collapsed_years:  HashSet<i32>,
    pub collapsed_months: HashSet<(i32, u32)>,
    pub visual_rows:      Vec<VisualRowKind>,
    // Chain View mode
    pub journal_chain_view: bool,

    // Column visibility for trade table (21 columns — last is EM)
    pub col_visibility:  [bool; 22],
    pub show_col_picker: bool,

    // Playbook auto-match candidates (when multiple match on save)
    pub playbook_match_candidates: Vec<i32>,

    // Scrolling
    pub thesis_scroll:      u16,
    pub thesis_max_scroll:  u16,
    pub detail_scroll:      u16,
    pub detail_max_scroll:  u16,
    pub detail_total_lines: usize,
    pub dash_open_scroll:     usize,
    pub dash_open_max_scroll: usize,
    pub dash_risk_scroll:     usize,
    pub dash_risk_max_scroll: usize,
    pub dash_panel_focus:     u8,   // 0 = Risk Distribution, 1 = Open Positions
    pub perf_subtab:              usize,   // 0 = OVERVIEW, 1 = CHARTS, 2 = ANALYTICS
    pub perf_overview_scroll:     u16,
    pub perf_overview_max_scroll: u16,
    pub perf_analytics_scroll:    u16,
    pub perf_analytics_max_scroll: u16,
    pub perf_collapsed:   [bool; 14], // 0=Health 1=Returns 2=Advanced 3=Growth 4=Strategy 5=Ticker 6=Monthly 7=IVR 8=VIX 9=DTE 10=IVREntry 11=PnlDist 12=Held 13=Commission
    pub perf_section_cursor: usize,  // 0-based index into current subtab's navigable section list
    pub perf_scroll_dirty:   bool,   // true → scroll refresh block should sync scroll to cursor

    // Calendar popup state
    pub cal_year:      i32,
    pub cal_month:     u32,
    pub cal_day:       u32,
    pub cal_field_idx: usize,
    pub cal_is_edit:   bool,    // true = edit_fields, false = close_fields
    pub cal_from_mode: AppMode,

    // Mode flags
    pub show_detail: bool,
    pub selected_trade_chain: Vec<models::Trade>,
    pub app_mode:    AppMode,

    // Journal filter / sort
    pub filter_status: FilterStatus,
    pub filter_ticker: String,
    pub sort_key:      SortKey,
    pub sort_desc:     bool,

    // Edit mode
    pub edit_trade_id: Option<i32>,
    pub edit_fields:   Vec<EditField>,
    pub edit_field_idx: usize,
    pub edit_scroll:   u16,

    // Analyze mode (payoff chart)
    pub analyze_trade_id: Option<i32>,

    // Close mode
    pub close_trade_id: Option<i32>,
    pub close_fields:   Vec<EditField>,
    pub close_field_idx: usize,

    // Delete confirmation
    pub delete_trade_id: Option<i32>,

    // Playbook edit mode
    pub edit_playbook_id:        Option<i32>,
    pub playbook_edit_fields:    Vec<EditField>,
    pub playbook_edit_field_idx: usize,
    pub playbook_edit_scroll:    u16,

    // Thesis in-place editor buffer
    pub thesis_edit_buf: String,

    // Journal Note quick-entry (Actions tab, N key)
    pub journal_note_buf:      String,
    pub journal_note_trade_id: Option<i32>,
}

impl AppState {
    pub fn new(storage: &storage::Storage) -> Self {
        let mut trades = storage.get_all_trades().unwrap_or_default();
        // Backfill BPR for any trade where it was never calculated
        for t in &mut trades {
            if t.bpr.is_none() {
                let bpr = calculations::calculate_bpr(
                    &t.legs,
                    t.credit_received,
                    t.quantity,
                    t.underlying_price,
                    t.strategy.as_str(),
                );
                if bpr > 0.0 {
                    t.bpr = Some(bpr);
                    let _ = storage.set_bpr(t.id, bpr);
                }
            }
        }
        let playbooks = storage.get_all_playbooks().unwrap_or_default();
        let account_size = storage.get_setting("account_size")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(100_000.0);
        let target_undefined_pct = storage.get_setting("target_undefined_pct")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(75.0);
        let max_heat_pct = storage.get_setting("max_heat_pct")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(50.0);
        let max_pos_bpr_pct = storage.get_setting("max_pos_bpr_pct")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(5.0);
        let monthly_pnl_target = storage.get_setting("monthly_pnl_target")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let drawdown_circuit_breaker_pct = storage.get_setting("drawdown_circuit_breaker_pct")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(5.0);
        let default_profit_target_pct = storage.get_setting("default_profit_target_pct")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let default_mgmt_dte = storage.get_setting("default_mgmt_dte")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(21);
        let risk_free_rate_pct = storage.get_setting("risk_free_rate_pct")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(4.5);
        let rolling_window = storage.get_setting("rolling_window")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(30);
        let col_visibility_str = storage.get_setting("col_visibility")
            .unwrap_or_else(|| "1111111111111111111".to_string());
        let col_visibility = parse_col_visibility(&col_visibility_str);
        let current_vix: Option<f64> = None;
        let beta_map = std::collections::HashMap::new();
        let spy_price: Option<f64> = None;
        let live_prices = std::collections::HashMap::new();
        let stats = calculations::build_portfolio_stats(&trades, account_size, current_vix, &beta_map, spy_price, target_undefined_pct, monthly_pnl_target);
        let perf_stats = calculations::build_performance_stats(&trades, account_size, risk_free_rate_pct, rolling_window);
        let playbook_analytics = calculations::build_playbook_analytics(&trades, &playbooks);

        let mut table_state = TableState::default();
        if !trades.is_empty() {
            table_state.select(Some(0));
        }
        let mut playbook_state = ListState::default();
        if !playbooks.is_empty() {
            playbook_state.select(Some(0));
        }

        let alerts = theta_vault_rust::actions::compute_alerts(
            &trades, &live_prices, account_size, current_vix,
            stats.net_beta_weighted_delta, stats.drift, stats.target_undefined_pct,
            stats.max_drawdown_pct, drawdown_circuit_breaker_pct,
            default_profit_target_pct, default_mgmt_dte,
        );
        let mut actions_list_state = ListState::default();
        if !alerts.is_empty() {
            actions_list_state.select(Some(0));
        }

        // Pre-collapse all alert groups except the first non-empty one in canonical order
        let collapsed_action_kinds_init: HashSet<theta_vault_rust::actions::AlertKind> = {
            use theta_vault_rust::actions::AlertKind;
            let order = [
                AlertKind::Defense, AlertKind::MaxLoss, AlertKind::GammaRisk,
                AlertKind::Warning, AlertKind::DeltaExtreme, AlertKind::UndefinedDrift,
                AlertKind::Drawdown, AlertKind::RollChain, AlertKind::Manage,
                AlertKind::Close, AlertKind::Roll, AlertKind::Sizing, AlertKind::Ok,
            ];
            let first = order.iter()
                .find(|k| alerts.iter().any(|a| &a.kind == *k))
                .cloned();
            order.iter()
                .filter(|k| alerts.iter().any(|a| &a.kind == *k))
                .filter(|k| Some(*k) != first.as_ref())
                .cloned()
                .collect()
        };

        let mut app = AppState {
            trades,
            playbooks,
            stats,
            perf_stats,
            playbook_analytics,
            account_size,
            target_undefined_pct,
            max_heat_pct,
            max_pos_bpr_pct,
            monthly_pnl_target,
            drawdown_circuit_breaker_pct,
            default_profit_target_pct,
            default_mgmt_dte,
            risk_free_rate_pct,
            rolling_window,
            export_status: None,
            export_status_tick: 0,
            col_visibility,
            show_col_picker: false,
            playbook_match_candidates: Vec::new(),
            current_vix,
            beta_map,
            spy_price,
            live_prices,
            spy_monthly: std::collections::HashMap::new(),
            alerts,
            actions_list_state,
            collapsed_action_kinds: collapsed_action_kinds_init,
            pulse_on:             false,
            under_tauri: std::env::var("THETA_VAULT_TAURI").is_ok(),
            dash_kpi_popup:          false,
            dash_kpi_scroll:         0,
            dash_kpi_max_scroll:     0,
            perf_kpi_popup:          false,
            perf_kpi_scroll:         0,
            perf_kpi_max_scroll:     0,
            journal_help_popup: false,
            journal_help_scroll: 0,
            journal_help_max_scroll: 0,
            journal_help_page: 0,
            admin_fields:    Vec::new(),
            admin_field_idx: 0,
            admin_scroll:    0,
            selected_tab:     0,
            table_state,
            playbook_state,
            collapsed_years:  HashSet::new(),
            collapsed_months: HashSet::new(),
            visual_rows:      Vec::new(),
            journal_chain_view: false,
            thesis_scroll:      0,
            thesis_max_scroll:  u16::MAX,
            detail_scroll:      0,
            detail_max_scroll:  u16::MAX,
            detail_total_lines: 0,
            dash_open_scroll:     0,
            dash_open_max_scroll: usize::MAX,
            dash_risk_scroll:     0,
            dash_risk_max_scroll: usize::MAX,
            dash_panel_focus:     1,   // default: Open Positions focused (existing behavior)
            perf_subtab: 0,
            perf_overview_scroll: 0,
            perf_overview_max_scroll: u16::MAX,
            perf_analytics_scroll: 0,
            perf_analytics_max_scroll: u16::MAX,
            // Default: Health(0), Returns(1), Growth(3) expanded; rest collapsed
            perf_collapsed:   [false, false, false, false, false, true, true, true, true, true, true, true, true, true],
            perf_section_cursor: 0,
            perf_scroll_dirty:   false,
            cal_year:      0,
            cal_month:     0,
            cal_day:       0,
            cal_field_idx: 0,
            cal_is_edit:   false,
            cal_from_mode: AppMode::Normal,
            show_detail:      false,
            selected_trade_chain: Vec::new(),
            app_mode:         AppMode::Normal,
            filter_status:    FilterStatus::All,
            filter_ticker:    String::new(),
            sort_key:         SortKey::Date,
            sort_desc:        true,
            edit_trade_id:    None,
            edit_fields:      Vec::new(),
            edit_field_idx:   0,
            edit_scroll:      0,
            analyze_trade_id: None,
            close_trade_id:   None,
            close_fields:     Vec::new(),
            close_field_idx:  0,
            delete_trade_id:  None,
            edit_playbook_id:        None,
            playbook_edit_fields:    Vec::new(),
            playbook_edit_field_idx: 0,
            playbook_edit_scroll:    0,
            thesis_edit_buf:         String::new(),
            journal_note_buf:        String::new(),
            journal_note_trade_id:   None,
        };
        app.rebuild_visual_rows();
        app
    }

    pub fn reload(&mut self, storage: &storage::Storage) {
        self.trades      = storage.get_all_trades().unwrap_or_default();
        self.playbooks   = storage.get_all_playbooks().unwrap_or_default();
        self.stats              = calculations::build_portfolio_stats(&self.trades, self.account_size, self.current_vix, &self.beta_map, self.spy_price, self.target_undefined_pct, self.monthly_pnl_target);
        self.perf_stats         = calculations::build_performance_stats(&self.trades, self.account_size, self.risk_free_rate_pct, self.rolling_window);
        self.playbook_analytics = calculations::build_playbook_analytics(&self.trades, &self.playbooks);
        self.alerts = theta_vault_rust::actions::compute_alerts(
            &self.trades, &self.live_prices, self.account_size, self.current_vix,
            self.stats.net_beta_weighted_delta, self.stats.drift, self.stats.target_undefined_pct,
            self.stats.max_drawdown_pct, self.drawdown_circuit_breaker_pct,
            self.default_profit_target_pct, self.default_mgmt_dte,
        );
        if !self.alerts.is_empty() && self.actions_list_state.selected().is_none() {
            self.actions_list_state.select(Some(0));
        }
        // If Charts subtab is currently visible in Tauri, refresh the payload
        if self.selected_tab == 5 && self.perf_subtab == 1 {
            self.start_perf_charts();
        }

        let selected = self.table_state.selected().unwrap_or(0);
        if self.trades.is_empty() {
            self.table_state.select(None);
        } else {
            // After reload, try to keep the same trade selected by ID
            if let Some(tid) = self.edit_trade_id.or(self.close_trade_id) {
                let filtered = self.filtered_sorted_indices();
                let idx = filtered.iter().position(|&i| self.trades[i].id == tid)
                    .unwrap_or(0.min(filtered.len().saturating_sub(1)));
                self.table_state.select(Some(idx));
            } else {
                let filtered_len = self.filtered_sorted_indices().len();
                self.table_state.select(Some(selected.min(filtered_len.saturating_sub(1))));
            }
        }
        let pb_sel = self.playbook_state.selected().unwrap_or(0);
        if self.playbooks.is_empty() {
            self.playbook_state.select(None);
        } else {
            self.playbook_state.select(Some(pb_sel.min(self.playbooks.len() - 1)));
        }
        self.rebuild_visual_rows();
    }

    /// Rebuild the visual_rows vec from filter/sort/collapse state.
    /// Must be called any time filter, sort, collapsed_months, or collapsed_years changes.
    pub fn rebuild_visual_rows(&mut self) {
        if self.journal_chain_view {
            self.visual_rows = self.build_chain_visual_rows();
            return;
        }
        use chrono::Datelike;
        use std::collections::BTreeMap;

        let sorted = self.filtered_sorted_indices();

        // Phase 1: group into (year, month) buckets, preserving sort order within each bucket
        let mut buckets: BTreeMap<(i32, u32), Vec<usize>> = BTreeMap::new();
        for trade_idx in sorted {
            let t = &self.trades[trade_idx];
            let key = (t.trade_date.year(), t.trade_date.month());
            buckets.entry(key).or_default().push(trade_idx);
        }

        // Phase 2: emit rows in descending (year, month) order — most recent first
        let mut rows = Vec::new();
        let mut last_year: Option<i32> = None;

        for (&(year, month), trade_indices) in buckets.iter().rev() {
            // Year header (once per year)
            if last_year != Some(year) {
                rows.push(VisualRowKind::YearHeader { year });
                last_year = Some(year);
            }
            if self.collapsed_years.contains(&year) {
                continue;
            }

            // Month header (exactly once per month)
            rows.push(VisualRowKind::MonthHeader { year, month });

            // Trades within the month (unless month is collapsed)
            if !self.collapsed_months.contains(&(year, month)) {
                for &ti in trade_indices {
                    rows.push(VisualRowKind::Trade(ti));
                }
            }
        }
        self.visual_rows = rows;
    }

    /// Build chain-grouped visual rows (Chain View mode).
    fn build_chain_visual_rows(&self) -> Vec<VisualRowKind> {
        use std::collections::{BTreeMap, HashMap, HashSet as HSet};

        // Build forward map: parent_id -> child trade index
        let mut child_map: HashMap<i32, usize> = HashMap::new();
        for (i, t) in self.trades.iter().enumerate() {
            if let Some(parent_id) = t.rolled_from_id {
                child_map.insert(parent_id, i);
            }
        }

        // Set of all known trade IDs
        let trade_ids: HSet<i32> = self.trades.iter().map(|t| t.id).collect();

        // Collect root indices grouped by ticker (BTreeMap = alpha sort)
        let mut ticker_roots: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, t) in self.trades.iter().enumerate() {
            let is_root = t.rolled_from_id
                .map_or(true, |pid| !trade_ids.contains(&pid));
            if is_root {
                ticker_roots.entry(t.ticker.clone()).or_default().push(i);
            }
        }

        let mut rows = Vec::new();

        for (ticker, root_indices) in &ticker_roots {
            // Ticker filter
            if !self.filter_ticker.is_empty()
                && !ticker.to_uppercase().contains(&self.filter_ticker.to_uppercase())
            {
                continue;
            }

            // Build chains for this ticker
            let mut chains: Vec<Vec<usize>> = Vec::new();
            for &root_idx in root_indices {
                let mut chain = Vec::new();
                let mut cur = root_idx;
                loop {
                    chain.push(cur);
                    let cur_id = self.trades[cur].id;
                    if let Some(&next) = child_map.get(&cur_id) {
                        cur = next;
                    } else {
                        break;
                    }
                }
                chains.push(chain);
            }

            // Sort chains: most recent entry first
            chains.sort_by(|a, b| {
                self.trades[b[0]].trade_date.cmp(&self.trades[a[0]].trade_date)
            });

            // Apply status filter at chain level
            let filtered: Vec<&Vec<usize>> = chains.iter().filter(|chain| {
                let chain_trades: Vec<&models::Trade> = chain.iter().map(|&i| &self.trades[i]).collect();
                match self.filter_status {
                    FilterStatus::All     => true,
                    FilterStatus::Open    => chain_trades.iter().any(|t| t.is_open()),
                    FilterStatus::Closed  => chain_trades.iter().all(|t| !t.is_open() && t.exit_reason.as_deref() != Some("expired") && t.exit_reason.as_deref() != Some("rolled")),
                    FilterStatus::Rolled  => chain_trades.iter().any(|t| t.exit_reason.as_deref() == Some("rolled")),
                    FilterStatus::Expired => chain_trades.last().map_or(false, |t| t.exit_reason.as_deref() == Some("expired")),
                }
            }).collect();

            if filtered.is_empty() {
                continue;
            }

            // Ticker-level stats
            let open_count  = filtered.iter().filter(|c| c.iter().any(|&i| self.trades[i].is_open())).count();
            let closed_count = filtered.iter().filter(|c| c.iter().all(|&i| !self.trades[i].is_open())).count();
            let net_pnl: f64 = filtered.iter()
                .flat_map(|c| c.iter())
                .filter_map(|&i| self.trades[i].pnl)
                .sum();

            rows.push(VisualRowKind::TickerHeader {
                ticker: ticker.clone(),
                open_count,
                closed_count,
                net_pnl,
            });

            for chain in &filtered {
                let root_trade = &self.trades[chain[0]];
                let chain_owned: Vec<models::Trade> = chain.iter().map(|&i| self.trades[i].clone()).collect();
                let metrics = calculations::calculate_campaign_metrics(&chain_owned);
                let is_open = chain.iter().any(|&i| self.trades[i].is_open());
                let chain_pnl = metrics.total_pnl;

                rows.push(VisualRowKind::ChainHeader {
                    root_id: root_trade.id,
                    ticker: ticker.clone(),
                    strategy: root_trade.strategy.badge().to_string(),
                    roll_count: metrics.roll_count as i32,
                    net_credit: metrics.net_credit,
                    chain_pnl,
                    chain_commissions: metrics.total_commissions,
                    is_open,
                    entry_date: root_trade.trade_date,
                });

                for &ti in chain.iter() {
                    rows.push(VisualRowKind::Trade(ti));
                }
            }
        }

        rows
    }

    /// Returns indices into self.trades for the current filter/sort.
    pub fn filtered_sorted_indices(&self) -> Vec<usize> {
        let mut idxs: Vec<usize> = (0..self.trades.len())
            .filter(|&i| {
                let t = &self.trades[i];
                // Status filter
                let status_ok = match self.filter_status {
                    FilterStatus::All     => true,
                    FilterStatus::Open    => t.is_open(),
                    FilterStatus::Closed  => !t.is_open() && t.exit_reason.as_deref() != Some("expired") && t.exit_reason.as_deref() != Some("rolled"),
                    FilterStatus::Rolled  => t.exit_reason.as_deref() == Some("rolled"),
                    FilterStatus::Expired => t.exit_reason.as_deref() == Some("expired"),
                };
                // Ticker filter
                let ticker_ok = self.filter_ticker.is_empty()
                    || t.ticker.to_uppercase().contains(&self.filter_ticker.to_uppercase());
                status_ok && ticker_ok
            })
            .collect();

        idxs.sort_by(|&a, &b| {
            let ta = &self.trades[a];
            let tb = &self.trades[b];
            let ord = match self.sort_key {
                SortKey::Date   => ta.trade_date.cmp(&tb.trade_date),
                SortKey::Ticker => ta.ticker.cmp(&tb.ticker),
                SortKey::Pnl    => {
                    let pa = ta.pnl.unwrap_or(f64::NEG_INFINITY);
                    let pb = tb.pnl.unwrap_or(f64::NEG_INFINITY);
                    pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::Roc    => {
                    let ra = ta.pnl.and_then(|p| calculations::calculate_roc(p, &ta.legs, ta.credit_received, ta.quantity, ta.spread_type(), ta.bpr, ta.underlying_price)).unwrap_or(f64::NEG_INFINITY);
                    let rb = tb.pnl.and_then(|p| calculations::calculate_roc(p, &tb.legs, tb.credit_received, tb.quantity, tb.spread_type(), tb.bpr, tb.underlying_price)).unwrap_or(f64::NEG_INFINITY);
                    ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::Dte    => {
                    calculations::calculate_remaining_dte(&ta.expiration_date)
                        .cmp(&calculations::calculate_remaining_dte(&tb.expiration_date))
                }
                SortKey::Credit => ta.credit_received.partial_cmp(&tb.credit_received)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortKey::PctMax => {
                    let pa = ta.pnl.map(|p| calculations::calculate_pct_max_profit(p, ta.credit_received, ta.quantity)).unwrap_or(f64::NEG_INFINITY);
                    let pb = tb.pnl.map(|p| calculations::calculate_pct_max_profit(p, tb.credit_received, tb.quantity)).unwrap_or(f64::NEG_INFINITY);
                    pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
                }
            };
            if self.sort_desc { ord.reverse() } else { ord }
        });

        idxs
    }

    /// Get the Trade currently selected in the journal (None if on a group header).
    pub fn selected_trade(&self) -> Option<&models::Trade> {
        let idx = self.table_state.selected()?;
        match self.visual_rows.get(idx)? {
            VisualRowKind::Trade(ti) => self.trades.get(*ti),
            VisualRowKind::YearHeader { .. } | VisualRowKind::MonthHeader { .. }
            | VisualRowKind::TickerHeader { .. } | VisualRowKind::ChainHeader { .. } => None,
        }
    }

    /// Get a mutable clone of the selected trade.
    pub fn selected_trade_cloned(&self) -> Option<models::Trade> {
        self.selected_trade().cloned()
    }

    pub fn nav_down(&mut self) {
        match self.selected_tab {
            1 => {
                if self.app_mode == AppMode::EditTrade {
                    if self.edit_field_idx + 1 < self.edit_fields.len() {
                        self.edit_field_idx += 1;
                        self.sync_edit_scroll();
                    }
                } else if self.app_mode == AppMode::CloseTrade {
                    if self.close_field_idx + 1 < self.close_fields.len() {
                        self.close_field_idx += 1;
                    }
                } else if self.show_detail {
                    self.detail_scroll = self.detail_scroll.saturating_add(1).min(self.detail_max_scroll);
                } else {
                    let len = self.visual_rows.len();
                    if len == 0 { return; }
                    let i = self.table_state.selected().unwrap_or(0);
                    self.table_state.select(Some(if i + 1 >= len { 0 } else { i + 1 }));
                    self.detail_scroll = 0;
                }
            }
            2 => {
                let len = self.playbooks.len();
                if len == 0 { return; }
                let i = self.playbook_state.selected().unwrap_or(0);
                self.playbook_state.select(Some(if i + 1 >= len { 0 } else { i + 1 }));
                self.thesis_scroll = 0;
            }
            3 => {
                let rows = theta_vault_rust::actions::build_action_rows(
                    &self.alerts, &self.collapsed_action_kinds,
                );
                let len = rows.len();
                if len == 0 { return; }
                let i = self.actions_list_state.selected().unwrap_or(0);
                self.actions_list_state.select(Some(if i + 1 >= len { 0 } else { i + 1 }));
            }
            0 => {
                if self.dash_panel_focus == 0 {
                    if self.dash_risk_scroll < self.dash_risk_max_scroll {
                        self.dash_risk_scroll = self.dash_risk_scroll.saturating_add(1);
                    }
                } else if self.dash_open_scroll < self.dash_open_max_scroll {
                    self.dash_open_scroll = self.dash_open_scroll.saturating_add(1);
                }
            }
            5 => {
                let count = perf_section_count(self.perf_subtab);
                self.perf_section_cursor = (self.perf_section_cursor + 1) % count;
                self.perf_scroll_dirty = true;
            }
            _ => {}
        }
    }

    pub fn nav_up(&mut self) {
        match self.selected_tab {
            1 => {
                if self.app_mode == AppMode::EditTrade {
                    if self.edit_field_idx > 0 {
                        self.edit_field_idx -= 1;
                        self.sync_edit_scroll();
                    }
                } else if self.app_mode == AppMode::CloseTrade {
                    if self.close_field_idx > 0 {
                        self.close_field_idx -= 1;
                    }
                } else if self.show_detail {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                } else {
                    let len = self.visual_rows.len();
                    if len == 0 { return; }
                    let i = self.table_state.selected().unwrap_or(0);
                    self.table_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
                    self.detail_scroll = 0;
                }
            }
            2 => {
                let len = self.playbooks.len();
                if len == 0 { return; }
                let i = self.playbook_state.selected().unwrap_or(0);
                self.playbook_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
                self.thesis_scroll = 0;
            }
            3 => {
                let rows = theta_vault_rust::actions::build_action_rows(
                    &self.alerts, &self.collapsed_action_kinds,
                );
                let len = rows.len();
                if len == 0 { return; }
                let i = self.actions_list_state.selected().unwrap_or(0);
                self.actions_list_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
            }
            0 => {
                if self.dash_panel_focus == 0 {
                    self.dash_risk_scroll = self.dash_risk_scroll.saturating_sub(1);
                } else {
                    self.dash_open_scroll = self.dash_open_scroll.saturating_sub(1);
                }
            }
            5 => {
                let count = perf_section_count(self.perf_subtab);
                self.perf_section_cursor = if self.perf_section_cursor == 0 {
                    count - 1
                } else {
                    self.perf_section_cursor - 1
                };
                self.perf_scroll_dirty = true;
            }
            _ => {}
        }
    }

    /// While in detail mode, move selection to the next Trade row (wraps around).
    pub fn nav_detail_next_trade(&mut self) {
        let len = self.visual_rows.len();
        if len == 0 { return; }
        let current = self.table_state.selected().unwrap_or(0);
        // Forward pass
        for i in (current + 1)..len {
            if matches!(self.visual_rows[i], VisualRowKind::Trade(_)) {
                self.table_state.select(Some(i));
                self.detail_scroll = 0;
                return;
            }
        }
        // Wrap: search from beginning
        for i in 0..=current {
            if matches!(self.visual_rows[i], VisualRowKind::Trade(_)) {
                self.table_state.select(Some(i));
                self.detail_scroll = 0;
                return;
            }
        }
    }

    /// While in detail mode, move selection to the previous Trade row (wraps around).
    pub fn nav_detail_prev_trade(&mut self) {
        let len = self.visual_rows.len();
        if len == 0 { return; }
        let current = self.table_state.selected().unwrap_or(0);
        // Backward pass
        if current > 0 {
            for i in (0..current).rev() {
                if matches!(self.visual_rows[i], VisualRowKind::Trade(_)) {
                    self.table_state.select(Some(i));
                    self.detail_scroll = 0;
                    return;
                }
            }
        }
        // Wrap: search from end
        for i in (current..len).rev() {
            if matches!(self.visual_rows[i], VisualRowKind::Trade(_)) {
                self.table_state.select(Some(i));
                self.detail_scroll = 0;
                return;
            }
        }
    }

    pub fn scroll_right(&mut self) {
        if self.selected_tab == 2 && self.thesis_scroll < self.thesis_max_scroll {
            self.thesis_scroll = self.thesis_scroll.saturating_add(1);
        }
    }

    pub fn scroll_left(&mut self) {
        if self.selected_tab == 2 {
            self.thesis_scroll = self.thesis_scroll.saturating_sub(1);
        }
    }

    fn sync_edit_scroll(&mut self) {
        // Keep focused field visible in a ~10-line viewport
        let visible = 10u16;
        let idx = self.edit_field_idx as u16;
        if idx < self.edit_scroll {
            self.edit_scroll = idx;
        } else if idx >= self.edit_scroll + visible {
            self.edit_scroll = idx - visible + 1;
        }
    }

    // ── Edit mode helpers ────────────────────────────────────────────────────

    pub fn start_edit(&mut self, trade: &models::Trade) {
        self.edit_trade_id  = Some(trade.id);
        self.edit_fields    = build_edit_fields(trade);
        self.edit_field_idx = 0;
        self.edit_scroll    = 0;
        self.app_mode       = AppMode::EditTrade;
        self.show_detail    = false;
    }

    pub fn start_analyze(&mut self, trade: &models::Trade) {
        self.analyze_trade_id = Some(trade.id);
        self.app_mode         = AppMode::AnalyzeTrade;
        self.show_detail      = false;
        // Signal Tauri wrapper via stdout (same fd as Ratatui — guaranteed through PTY master).
        // Ignored when running standalone: xterm interprets unknown OSC 9999 as no-op.
        if let Ok(json) = serde_json::to_string(trade) {
            use std::io::Write;
            let seq = format!("\x1b]9999;THETAVAULT_CHART:{}\x07", json);
            let _ = std::io::stdout().lock().write_all(seq.as_bytes());
            let _ = std::io::stdout().lock().flush();
        }
    }

    /// Emit OSC 9998 payload so Tauri right panel renders performance charts as SVG.
    /// No-op when running standalone (xterm ignores unknown OSC sequences).
    pub fn start_perf_charts(&self) {
        if !self.under_tauri { return; }
        let payload = models::PerfChartPayload {
            account_size:       self.account_size,
            balance_history:    self.perf_stats.balance_history.clone(),
            unrealized_history: self.perf_stats.unrealized_history.clone(),
            peak_history:       self.perf_stats.peak_history.clone(),
            monthly_pnl:        self.perf_stats.monthly_pnl.clone(),
            rolling_win_rate:   self.perf_stats.rolling_win_rate.clone(),
            dte_roc_scatter:    self.perf_stats.dte_roc_scatter.iter()
                .map(|(d, r, s)| (*d, *r, s.label().to_string()))
                .collect(),
            bpr_history:        self.perf_stats.bpr_history.clone(),
            sector_trends:      self.perf_stats.sector_trends.clone(),
        };
        if let Ok(json) = serde_json::to_string(&payload) {
            use std::io::Write;
            let seq = format!("\x1b]9998;THETAVAULT_PERF:{}\x07", json);
            let _ = std::io::stdout().lock().write_all(seq.as_bytes());
            let _ = std::io::stdout().lock().flush();
        }
    }

    /// Send OSC 9998 close signal so Tauri hides the performance chart panel.
    pub fn stop_perf_charts(&self) {
        if !self.under_tauri { return; }
        use std::io::Write;
        let seq = "\x1b]9998;THETAVAULT_PERF_CLOSE\x07";
        let _ = std::io::stdout().lock().write_all(seq.as_bytes());
        let _ = std::io::stdout().lock().flush();
    }

    pub fn edit_key_char(&mut self, c: char) {
        if let Some(field) = self.edit_fields.get_mut(self.edit_field_idx) {
            match &field.kind {
                FieldKind::Bool => {
                    // toggle with space or y/n
                    if c == ' ' || c == 'y' || c == 'Y' {
                        field.value = "true".to_string();
                    } else if c == 'n' || c == 'N' {
                        field.value = "false".to_string();
                    }
                }
                FieldKind::Select(opts) => {
                    // +/- to cycle through options
                    if c == '+' || c == ' ' {
                        let cur = field.value.parse::<usize>().unwrap_or(0);
                        field.value = ((cur + 1) % opts.len()).to_string();
                    } else if c == '-' {
                        let cur = field.value.parse::<usize>().unwrap_or(0);
                        field.value = if cur == 0 { (opts.len() - 1).to_string() } else { (cur - 1).to_string() };
                    }
                }
                FieldKind::Number | FieldKind::Text | FieldKind::Multiline | FieldKind::Date => {
                    field.value.push(c);
                }
                FieldKind::Button(_) => {} // no-op: buttons are activated with Enter
            }
        }
    }

    // ── Leg-aware edit helpers ───────────────────────────────────────────────

    /// Rebuild the legs section when the Strategy SELECT changes.
    pub fn rebuild_legs_in_edit_fields(&mut self) {
        let new_strategy = get_strategy_from_fields(&self.edit_fields);
        let current_legs = extract_legs_from_edit_fields(&self.edit_fields);
        let (merged, _) = models::merge_legs_for_strategy_change(&current_legs, &new_strategy);

        // Find legs section: first field whose section_header contains "Legs"
        let legs_start = self.edit_fields.iter().position(|f| {
            f.section_header.as_deref().map_or(false, |h| h.contains("Legs"))
                || extract_leg_number(&f.label).is_some()
                || matches!(&f.kind, FieldKind::Button(_))
        });

        // End of legs section: first field (after start) that has a section_header starting a NEW section
        let after_legs = legs_start.and_then(|start| {
            self.edit_fields.iter().enumerate().skip(start + 1).find(|(_, f)| {
                f.section_header.as_deref().map_or(false, |h| !h.is_empty() && !h.contains("Legs"))
            }).map(|(i, _)| i)
        });

        if let (Some(ls), Some(al)) = (legs_start, after_legs) {
            let new_leg_fields = build_leg_fields_for_strategy(&merged, &new_strategy);
            self.edit_fields.drain(ls..al);
            for (i, fld) in new_leg_fields.into_iter().enumerate() {
                self.edit_fields.insert(ls + i, fld);
            }
            // Clamp focus
            if self.edit_field_idx >= self.edit_fields.len() {
                self.edit_field_idx = self.edit_fields.len().saturating_sub(1);
            }
            self.sync_edit_scroll();
        }
    }

    /// Ctrl+A: add a blank leg (Custom and PBWB strategies).
    pub fn add_leg_to_edit_fields(&mut self) {
        let strat = get_strategy_from_fields(&self.edit_fields);
        if strat != models::StrategyType::Custom && strat != models::StrategyType::PutBrokenWingButterfly { return; }

        let new_n = count_legs_in_fields(&self.edit_fields) + 1;

        // Insert before the "+ Add Leg" button (or before the Exit section)
        let insert_pos = self.edit_fields.iter().position(|f| matches!(&f.kind, FieldKind::Button(_)))
            .unwrap_or_else(|| {
                self.edit_fields.iter().position(|f| {
                    f.section_header.as_deref().map_or(false, |h| h.contains("Exit"))
                }).unwrap_or(self.edit_fields.len())
            });

        let new_fields = vec![
            EditField::select(&format!("Leg {} Type", new_n), "0", models::LegType::all_options()),
            EditField::number(&format!("Leg {} Strike", new_n), "0.00"),
            EditField::number(&format!("Leg {} Premium", new_n), "0.0000"),
            EditField::number(&format!("Leg {} Close", new_n), ""),
            EditField::date(&format!("Leg {} Expiry", new_n), ""),
            EditField::number(&format!("Leg {} Qty", new_n), "1"),
        ];
        for (i, fld) in new_fields.into_iter().enumerate() {
            self.edit_fields.insert(insert_pos + i, fld);
        }
        fix_legs_section(&mut self.edit_fields);
        self.edit_field_idx = insert_pos;
        self.sync_edit_scroll();
    }

    /// Ctrl+D: delete the leg whose field is currently focused (Custom and PBWB).
    pub fn delete_focused_leg_from_edit_fields(&mut self) {
        let strat = get_strategy_from_fields(&self.edit_fields);
        if strat != models::StrategyType::Custom && strat != models::StrategyType::PutBrokenWingButterfly { return; }

        let focused_label = self.edit_fields.get(self.edit_field_idx)
            .map(|f| f.label.clone())
            .unwrap_or_default();
        let leg_num = match extract_leg_number(&focused_label) {
            Some(n) => n,
            None => return, // not focused on a leg field
        };

        self.edit_fields.retain(|f| extract_leg_number(&f.label) != Some(leg_num));
        renumber_leg_fields(&mut self.edit_fields);
        fix_legs_section(&mut self.edit_fields);

        if self.edit_field_idx >= self.edit_fields.len() {
            self.edit_field_idx = self.edit_fields.len().saturating_sub(1);
        }
        self.sync_edit_scroll();
    }

    pub fn edit_key_backspace(&mut self) {
        if let Some(field) = self.edit_fields.get_mut(self.edit_field_idx) {
            if matches!(field.kind, FieldKind::Text | FieldKind::Number) {
                field.value.pop();
            }
        }
    }

    /// Apply edit fields back onto a Trade clone, returning it ready to save.
    pub fn build_updated_trade(&self, original: &models::Trade) -> models::Trade {
        let mut t = original.clone();
        apply_edit_fields_to_trade(&self.edit_fields, &mut t);
        // L8: auto-match playbook if none assigned
        if t.playbook_id.is_none() {
            let matches = calculations::find_matching_playbooks(&t, &self.playbooks);
            if matches.len() == 1 {
                t.playbook_id = Some(matches[0]);
            }
        }
        t
    }

    // ── Close mode helpers ───────────────────────────────────────────────────

    pub fn start_close(&mut self, trade: &models::Trade) {
        self.close_trade_id  = Some(trade.id);
        self.close_fields    = build_close_fields(trade);
        self.close_field_idx = 0;
        self.app_mode        = AppMode::CloseTrade;
        self.show_detail     = false;
    }

    pub fn close_key_char(&mut self, c: char) {
        if let Some(field) = self.close_fields.get_mut(self.close_field_idx) {
            match &field.kind {
                FieldKind::Select(opts) => {
                    if c == '+' || c == ' ' {
                        let cur = field.value.parse::<usize>().unwrap_or(0);
                        field.value = ((cur + 1) % opts.len()).to_string();
                    } else if c == '-' {
                        let cur = field.value.parse::<usize>().unwrap_or(0);
                        field.value = if cur == 0 { (opts.len() - 1).to_string() } else { (cur - 1).to_string() };
                    }
                }
                FieldKind::Button(_) => {}
                _ => { field.value.push(c); }
            }
        }
    }

    pub fn close_key_backspace(&mut self) {
        if let Some(field) = self.close_fields.get_mut(self.close_field_idx) {
            field.value.pop();
        }
    }

    // ── Calendar / date picker helpers ───────────────────────────────────────

    pub fn open_date_picker(&mut self, is_edit: bool) {
        let value = {
            let fields = if is_edit { &self.edit_fields } else { &self.close_fields };
            let idx    = if is_edit { self.edit_field_idx } else { self.close_field_idx };
            fields.get(idx).map(|f| f.value.clone()).unwrap_or_default()
        };
        let nd = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive());
        use chrono::Datelike;
        self.cal_year      = nd.year();
        self.cal_month     = nd.month();
        self.cal_day       = nd.day();
        self.cal_field_idx = if is_edit { self.edit_field_idx } else { self.close_field_idx };
        self.cal_is_edit   = is_edit;
        self.cal_from_mode = self.app_mode;
        self.app_mode      = AppMode::DatePicker;
    }

    pub fn cal_move_days(&mut self, delta: i64) {
        use chrono::{Datelike, Duration, NaiveDate};
        if let Some(nd) = NaiveDate::from_ymd_opt(self.cal_year, self.cal_month, self.cal_day) {
            let next = nd + Duration::days(delta);
            self.cal_year  = next.year();
            self.cal_month = next.month();
            self.cal_day   = next.day();
        }
    }

    pub fn cal_move_months(&mut self, delta: i32) {
        use chrono::{Datelike, NaiveDate};
        let total = self.cal_year * 12 + self.cal_month as i32 - 1 + delta;
        let year  = total / 12;
        let month = ((total % 12) + 1) as u32;
        // clamp day to valid range for the new month
        let max_day = NaiveDate::from_ymd_opt(year, month + 1, 1)
            .or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1))
            .and_then(|d| d.pred_opt())
            .map(|d| d.day())
            .unwrap_or(28);
        self.cal_year  = year;
        self.cal_month = month;
        self.cal_day   = self.cal_day.min(max_day);
    }

    pub fn cal_confirm_selection(&mut self) {
        let value = format!("{:04}-{:02}-{:02}", self.cal_year, self.cal_month, self.cal_day);
        let fields = if self.cal_is_edit { &mut self.edit_fields } else { &mut self.close_fields };
        if let Some(f) = fields.get_mut(self.cal_field_idx) {
            f.value = value;
        }
        self.app_mode = self.cal_from_mode;
    }

    /// Build a partially-updated Trade for closing (sets exit, pnl, legs).
    pub fn build_closed_trade(&self, original: &models::Trade) -> models::Trade {
        let mut t = original.clone();
        apply_close_fields_to_trade(&self.close_fields, &mut t);
        // L8: auto-match playbook if none assigned
        if t.playbook_id.is_none() {
            let matches = calculations::find_matching_playbooks(&t, &self.playbooks);
            if matches.len() == 1 {
                t.playbook_id = Some(matches[0]);
            }
        }
        t
    }

    pub fn cancel_mode(&mut self) {
        self.app_mode         = AppMode::Normal;
        self.edit_trade_id    = None;
        self.edit_fields      = Vec::new();
        self.analyze_trade_id = None;
        self.close_trade_id   = None;
        self.close_fields     = Vec::new();
        self.delete_trade_id  = None;
        self.edit_playbook_id        = None;
        self.playbook_edit_fields    = Vec::new();
        self.playbook_edit_field_idx = 0;
        self.playbook_edit_scroll    = 0;
        self.admin_fields    = Vec::new();
        self.admin_field_idx = 0;
        self.admin_scroll    = 0;
    }

    pub fn start_admin_settings(&mut self) {
        self.admin_fields    = build_admin_settings_fields(self.account_size, self.max_heat_pct, self.max_pos_bpr_pct, self.target_undefined_pct, self.monthly_pnl_target, self.drawdown_circuit_breaker_pct, self.default_profit_target_pct, self.default_mgmt_dte, self.risk_free_rate_pct, self.rolling_window);
        self.admin_field_idx = 0;
        self.admin_scroll    = 0;
        self.app_mode        = AppMode::AdminSettings;
    }

    fn sync_admin_scroll(&mut self) {
        let visible = 12u16;
        let idx = self.admin_field_idx as u16;
        if idx < self.admin_scroll {
            self.admin_scroll = idx;
        } else if idx >= self.admin_scroll + visible {
            self.admin_scroll = idx - visible + 1;
        }
    }

    fn sync_playbook_edit_scroll(&mut self) {
        let visible = 10u16;
        let idx = self.playbook_edit_field_idx as u16;
        if idx < self.playbook_edit_scroll {
            self.playbook_edit_scroll = idx;
        } else if idx >= self.playbook_edit_scroll + visible {
            self.playbook_edit_scroll = idx - visible + 1;
        }
    }

    pub fn start_new_playbook(&mut self) {
        let pb = models::PlaybookStrategy {
            id: 0,
            name: String::new(),
            description: None,
            spread_type: None,
            entry_criteria: None,
        };
        self.edit_playbook_id        = None;
        self.playbook_edit_fields    = build_playbook_edit_fields(&pb);
        self.playbook_edit_field_idx = 0;
        self.playbook_edit_scroll    = 0;
        self.app_mode                = AppMode::EditPlaybook;
    }

    pub fn start_edit_playbook(&mut self) {
        let idx = match self.playbook_state.selected() {
            Some(i) => i,
            None    => return,
        };
        let pb = match self.playbooks.get(idx) {
            Some(p) => p.clone(),
            None    => return,
        };
        self.edit_playbook_id        = Some(pb.id);
        self.playbook_edit_fields    = build_playbook_edit_fields(&pb);
        self.playbook_edit_field_idx = 0;
        self.playbook_edit_scroll    = 0;
        self.app_mode                = AppMode::EditPlaybook;
    }

    pub fn start_thesis_edit(&mut self) {
        if let Some(idx) = self.playbook_state.selected() {
            if let Some(pb) = self.playbooks.get(idx) {
                self.thesis_edit_buf = pb.description.clone().unwrap_or_default();
                self.thesis_scroll   = 0;
                self.app_mode        = AppMode::EditThesis;
            }
        }
    }

    pub fn save_thesis(&mut self, storage: &storage::Storage) {
        if let Some(idx) = self.playbook_state.selected() {
            if let Some(pb) = self.playbooks.get(idx).cloned() {
                let mut updated = pb.clone();
                updated.description = if self.thesis_edit_buf.is_empty() {
                    None
                } else {
                    Some(self.thesis_edit_buf.clone())
                };
                let _ = storage.update_playbook(pb.id, &updated);
                self.app_mode = AppMode::Normal;
                self.reload(storage);
            }
        }
    }

    pub fn build_playbook_from_edit_fields(&self) -> models::PlaybookStrategy {
        build_playbook_from_edit_fields_fn(&self.playbook_edit_fields)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy helpers
// ─────────────────────────────────────────────────────────────────────────────

fn all_strategy_types() -> Vec<models::StrategyType> {
    vec![
        models::StrategyType::ShortPutVertical,
        models::StrategyType::ShortCallVertical,
        models::StrategyType::IronCondor,
        models::StrategyType::IronButterfly,
        models::StrategyType::Strangle,
        models::StrategyType::Straddle,
        models::StrategyType::CalendarSpread,
        models::StrategyType::CashSecuredPut,
        models::StrategyType::CoveredCall,
        models::StrategyType::Pmcc,
        models::StrategyType::LongDiagonalSpread,
        models::StrategyType::ShortDiagonalSpread,
        models::StrategyType::LongCallVertical,
        models::StrategyType::LongPutVertical,
        models::StrategyType::Zebra,
        models::StrategyType::Custom,
        models::StrategyType::PutBrokenWingButterfly,
    ]
}

/// True for calendar/diagonal/pmcc/custom — strategies with per-leg expiry.
fn strategy_shows_per_leg_expiry(strategy: &models::StrategyType) -> bool {
    matches!(strategy,
        models::StrategyType::CalendarSpread |
        models::StrategyType::LongDiagonalSpread |
        models::StrategyType::ShortDiagonalSpread |
        models::StrategyType::Pmcc |
        models::StrategyType::Zebra |
        models::StrategyType::Custom)
}

/// True for custom / zebra / PBWB — per-leg quantity fields.
fn strategy_shows_per_leg_qty(strategy: &models::StrategyType) -> bool {
    matches!(strategy,
        models::StrategyType::Zebra |
        models::StrategyType::Custom |
        models::StrategyType::PutBrokenWingButterfly)
}

/// Read the Strategy SELECT field and return the StrategyType.
fn get_strategy_from_fields(fields: &[EditField]) -> models::StrategyType {
    fields.iter().find(|f| f.label == "Strategy").and_then(|f| {
        let idx = f.value.parse::<usize>().unwrap_or(0);
        all_strategy_types().into_iter().nth(idx)
    }).unwrap_or(models::StrategyType::Custom)
}

// ─────────────────────────────────────────────────────────────────────────────
// Leg field helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse "Leg N ..." label → N (1-indexed). Returns None for non-leg labels.
fn extract_leg_number(label: &str) -> Option<usize> {
    let rest = label.strip_prefix("Leg ")?;
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end].parse::<usize>().ok()
}

/// Count how many distinct leg numbers are in the field list.
fn count_legs_in_fields(fields: &[EditField]) -> usize {
    fields.iter().filter_map(|f| extract_leg_number(&f.label)).max().unwrap_or(0)
}

/// Extract Vec<TradeLeg> from the indexed "Leg N ..." edit fields.
fn extract_legs_from_edit_fields(fields: &[EditField]) -> Vec<models::TradeLeg> {
    let mut legs = Vec::new();
    let mut n = 1usize;
    loop {
        let type_label = format!("Leg {} Type", n);
        let type_field = match fields.iter().find(|f| f.label == type_label) {
            Some(f) => f,
            None => break,
        };
        let leg_type = if let FieldKind::Select(opts) = &type_field.kind {
            let idx = type_field.value.parse::<usize>().unwrap_or(0);
            let s = opts.get(idx).map(|s| s.as_str()).unwrap_or("Short Put");
            models::LegType::from_str(s).unwrap_or(models::LegType::ShortPut)
        } else {
            models::LegType::ShortPut
        };
        let strike = fields.iter().find(|f| f.label == format!("Leg {} Strike", n))
            .and_then(|f| f.value.parse::<f64>().ok()).unwrap_or(0.0);
        let premium = fields.iter().find(|f| f.label == format!("Leg {} Premium", n))
            .and_then(|f| f.value.parse::<f64>().ok()).unwrap_or(0.0);
        let close_premium = fields.iter().find(|f| f.label == format!("Leg {} Close", n))
            .and_then(|f| if f.value.is_empty() { None } else { f.value.parse::<f64>().ok() });
        let expiration_date = fields.iter().find(|f| f.label == format!("Leg {} Expiry", n))
            .and_then(|f| if f.value.is_empty() { None } else { Some(f.value.clone()) });
        let quantity = fields.iter().find(|f| f.label == format!("Leg {} Qty", n))
            .and_then(|f| f.value.parse::<i32>().ok());
        legs.push(models::TradeLeg { leg_type, strike, premium, close_premium, expiration_date, quantity });
        n += 1;
    }
    legs
}

/// Build indexed leg fields for a strategy — called by build_edit_fields and rebuild.
fn build_leg_fields_for_strategy(legs: &[models::TradeLeg], strategy: &models::StrategyType) -> Vec<EditField> {
    let mut f: Vec<EditField> = Vec::new();
    let show_expiry = strategy_shows_per_leg_expiry(strategy);
    let show_qty    = strategy_shows_per_leg_qty(strategy);
    let hdr = format!("── Legs ({} total) ─────────────────────────────────────────────", legs.len());

    for (i, leg) in legs.iter().enumerate() {
        let n = i + 1;
        let type_idx = match leg.leg_type {
            models::LegType::ShortPut  => 0,
            models::LegType::LongPut   => 1,
            models::LegType::ShortCall => 2,
            models::LegType::LongCall  => 3,
        };
        let mut type_field = EditField::select(
            &format!("Leg {} Type", n),
            &type_idx.to_string(),
            models::LegType::all_options(),
        );
        if i == 0 { type_field.section_header = Some(hdr.clone()); }
        f.push(type_field);
        f.push(EditField::number(&format!("Leg {} Strike", n),  &format!("{:.2}", leg.strike)));
        f.push(EditField::number(&format!("Leg {} Premium", n), &format!("{:.4}", leg.premium)));
        f.push(EditField::number(&format!("Leg {} Close", n),
            &leg.close_premium.map(|p| format!("{:.4}", p)).unwrap_or_default()));
        if show_expiry {
            let val = leg.expiration_date.as_deref().unwrap_or_default();
            let clean_val = val.split('T').next().unwrap_or(val);
            f.push(EditField::date(&format!("Leg {} Expiry", n), clean_val));
        }
        if show_qty {
            f.push(EditField::number(&format!("Leg {} Qty", n),
                &leg.quantity.map(|q| q.to_string()).unwrap_or_else(|| "1".to_string())));
        }
    }

    if *strategy == models::StrategyType::Custom
        || *strategy == models::StrategyType::PutBrokenWingButterfly
    {
        let mut btn = EditField {
            label:          "+ Add Leg".to_string(),
            value:          String::new(),
            kind:           FieldKind::Button("[+ Add Leg]  (Enter or Ctrl+A)".to_string()),
            section_header: None,
        };
        // If no legs yet, button carries the section header
        if legs.is_empty() {
            btn.section_header = Some(hdr);
        }
        f.push(btn);
    }
    f
}

/// Fix the legs section header to show the correct leg count.
/// Sets the header on the first leg Type field or the "+ Add Leg" button.
fn fix_legs_section(fields: &mut Vec<EditField>) {
    let count = count_legs_in_fields(fields);
    let hdr = format!("── Legs ({} total) ─────────────────────────────────────────────", count);
    let mut first_found = false;
    for field in fields.iter_mut() {
        let is_leg = extract_leg_number(&field.label).is_some() && field.label.ends_with("Type");
        let is_btn = field.label == "+ Add Leg";
        if is_leg || is_btn {
            if !first_found {
                field.section_header = Some(hdr.clone());
                first_found = true;
            } else {
                field.section_header = None;
            }
        }
    }
}

/// Renumber leg fields sequentially after a deletion.
fn renumber_leg_fields(fields: &mut Vec<EditField>) {
    // Collect unique leg numbers in sorted order
    let mut unique: Vec<usize> = fields.iter()
        .filter_map(|f| extract_leg_number(&f.label))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    unique.sort_unstable();

    for (new_idx, &old_num) in unique.iter().enumerate() {
        let new_num = new_idx + 1;
        if old_num == new_num { continue; }
        let old_prefix = format!("Leg {} ", old_num);
        for field in fields.iter_mut() {
            if let Some(rest) = field.label.strip_prefix(&old_prefix) {
                let rest = rest.to_string();
                field.label = format!("Leg {} {}", new_num, rest);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Build edit fields from a Trade
// ─────────────────────────────────────────────────────────────────────────────

fn build_edit_fields(t: &models::Trade) -> Vec<EditField> {
    let mut f: Vec<EditField> = Vec::new();

    // ── Core — Strategy SELECT is first
    let stypes  = all_strategy_types();
    let slabels: Vec<String> = stypes.iter().map(|s| s.label().to_string()).collect();
    let sidx    = stypes.iter().position(|s| s == &t.strategy).unwrap_or(0);
    f.push(EditField::select("Strategy", &sidx.to_string(), slabels)
        .with_section("── Core ─────────────────────────────────────────────────────────"));
    f.push(EditField::text("Ticker",      &t.ticker));
    f.push(EditField::number("Quantity",  &t.quantity.to_string()));
    f.push(EditField::date("Trade Date",  &t.trade_date.format("%Y-%m-%d").to_string()));
    f.push(EditField::date("Expiration",  &t.expiration_date.format("%Y-%m-%d").to_string()));
    f.push(EditField::date("Back Month Exp",
        &t.back_month_expiration.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));

    // ── Legs (strategy-aware: type SELECT, optional expiry/qty, add button)
    f.extend(build_leg_fields_for_strategy(&t.legs, &t.strategy));

    // ── Exit
    f.push(EditField::date("Exit Date",
        &t.exit_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default())
        .with_section("── Exit ──────────────────────────────────────────────────────────"));
    let exit_reasons = vec!["".to_string(), "closed".to_string(), "expired".to_string(), "rolled".to_string(), "stopped".to_string()];
    let exit_idx = exit_reasons.iter().position(|r| Some(r.as_str()) == t.exit_reason.as_deref()).unwrap_or(0);
    f.push(EditField::select("Exit Reason", &exit_idx.to_string(), exit_reasons)
        .with_section("  (Space/+ to cycle)"));

    // ── Greeks
    f.push(EditField::number("Delta",  &t.delta.map(|v| format!("{:.4}", v)).unwrap_or_default())
        .with_section("── Greeks ────────────────────────────────────────────────────────"));
    f.push(EditField::number("Theta",  &t.theta.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("Gamma",  &t.gamma.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("Vega",   &t.vega.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("POP %",  &t.pop.map(|v| format!("{:.1}", v)).unwrap_or_default()));

    // ── Entry Conditions
    f.push(EditField::number("Underlying $",
        &t.underlying_price.map(|v| format!("{:.2}", v)).unwrap_or_default())
        .with_section("── Entry Conditions ──────────────────────────────────────────────"));
    f.push(EditField::number("IVR %",             &t.iv_rank.map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("IV Pct %",          &t.iv_percentile.map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("VIX at Entry",      &t.vix_at_entry.map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("Impl Vol %",        &t.implied_volatility.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("Underlying @ Close",&t.underlying_price_at_close.map(|v| format!("{:.2}", v)).unwrap_or_default()));

    // ── Meta
    f.push(EditField::number("Commission",
        &t.commission.map(|v| format!("{:.2}", v)).unwrap_or_default())
        .with_section("── Trade Meta ────────────────────────────────────────────────────"));
    f.push(EditField::number("Target Profit %",
        &t.target_profit_pct.map(|v| format!("{:.0}", v)).unwrap_or_default()));

    let mgmt_opts = vec!["".to_string(), "50pct_profit".to_string(), "21_dte".to_string(), "bp_defend".to_string(), "manual".to_string()];
    let mgmt_idx = mgmt_opts.iter().position(|r| Some(r.as_str()) == t.management_rule.as_deref()).unwrap_or(0);
    f.push(EditField::select("Management Rule", &mgmt_idx.to_string(), mgmt_opts));

    let grade_opts = vec!["".to_string(), "A".to_string(), "B".to_string(), "C".to_string(), "D".to_string(), "F".to_string()];
    let grade_idx = grade_opts.iter().position(|r| Some(r.as_str()) == t.trade_grade.as_deref()).unwrap_or(0);
    f.push(EditField::select("Grade", &grade_idx.to_string(), grade_opts));

    f.push(EditField::text("Grade Notes",         &t.grade_notes.clone().unwrap_or_default()));
    f.push(EditField::text("Entry Reason",         &t.entry_reason.clone().unwrap_or_default()));
    f.push(EditField::text("Exit Reason (notes)",  &t.exit_reason.clone().unwrap_or_default()));
    f.push(EditField::date("Earnings Date",
        &t.next_earnings.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
    f.push(EditField::text("Tags (comma)",         &t.tags.join(",")));
    f.push(EditField::text("Notes",                &t.notes.clone().unwrap_or_default()));
    f.push(EditField::bool_field("Earnings Play",  t.is_earnings_play));
    f.push(EditField::bool_field("Is Tested",      t.is_tested));

    // ── Execution Quality
    f.push(EditField::number("Bid-Ask Spread",
        &t.bid_ask_spread_at_entry.map(|v| format!("{:.2}", v)).unwrap_or_default())
        .with_section("── Execution Quality ─────────────────────────────────────────────"));
    f.push(EditField::number("Fill vs Mid",
        &t.fill_vs_mid.map(|v| format!("{:.2}", v)).unwrap_or_default()));

    f
}

// ─────────────────────────────────────────────────────────────────────────────
// Apply edit fields back onto Trade
// ─────────────────────────────────────────────────────────────────────────────

fn field_opt_f64(fields: &[EditField], label: &str) -> Option<f64> {
    fields.iter().find(|f| f.label == label)
        .and_then(|f| if f.value.is_empty() { None } else { f.value.parse().ok() })
}

fn field_str(fields: &[EditField], label: &str) -> String {
    fields.iter().find(|f| f.label == label).map(|f| f.value.clone()).unwrap_or_default()
}

fn field_opt_str(fields: &[EditField], label: &str) -> Option<String> {
    fields.iter().find(|f| f.label == label)
        .and_then(|f| if f.value.is_empty() { None } else { Some(f.value.clone()) })
}

fn field_select_value(fields: &[EditField], label: &str) -> Option<String> {
    fields.iter().find(|f| f.label == label).and_then(|f| {
        if let FieldKind::Select(opts) = &f.kind {
            let idx = f.value.parse::<usize>().unwrap_or(0);
            opts.get(idx).and_then(|v| if v.is_empty() { None } else { Some(v.clone()) })
        } else {
            None
        }
    })
}

fn apply_edit_fields_to_trade(fields: &[EditField], t: &mut models::Trade) {
    // ── Strategy
    let sidx = fields.iter().find(|f| f.label == "Strategy")
        .and_then(|f| f.value.parse::<usize>().ok()).unwrap_or(0);
    t.strategy = all_strategy_types().into_iter().nth(sidx).unwrap_or(models::StrategyType::Custom);

    // ── Core
    t.ticker   = field_str(fields, "Ticker").to_uppercase();
    t.quantity = field_str(fields, "Quantity").parse().unwrap_or(t.quantity);
    if let Some(d) = parse_date_field(fields, "Trade Date")    { t.trade_date = d; }
    if let Some(d) = parse_date_field(fields, "Expiration")    { t.expiration_date = d; }
    t.back_month_expiration = parse_date_field(fields, "Back Month Exp");
    // Auto-calculate entry_dte from trade_date→expiration (only if blank; preserves manual overrides)
    if t.entry_dte.is_none() && t.trade_date < t.expiration_date {
        let days = (t.expiration_date.date_naive() - t.trade_date.date_naive()).num_days().max(0) as i32;
        t.entry_dte = Some(days);
    }

    // Read underlying price early — needed for BPR calculation
    let underlying_for_bpr = fields.iter().find(|f| f.label == "Underlying $")
        .and_then(|f| if f.value.is_empty() { None } else { f.value.parse::<f64>().ok() });

    // ── Legs (from indexed "Leg N ..." fields)
    t.legs = extract_legs_from_edit_fields(fields);

    // Recompute net credit from legs (per-contract, not multiplied by qty)
    let short_sum: f64 = t.legs.iter().filter(|l| l.leg_type.is_short()).map(|l| l.premium).sum();
    let long_sum:  f64 = t.legs.iter().filter(|l| !l.leg_type.is_short()).map(|l| l.premium).sum();
    t.credit_received = short_sum - long_sum;

    // Update convenience strike/premium fields
    if let Some(sp) = t.legs.iter().find(|l| l.leg_type == models::LegType::ShortPut)  { t.short_strike = sp.strike; }
    if let Some(sc) = t.legs.iter().find(|l| l.leg_type == models::LegType::ShortCall) { t.short_strike = sc.strike; }
    if let Some(lp) = t.legs.iter().find(|l| l.leg_type == models::LegType::LongPut)   { t.long_strike  = lp.strike; }
    if let Some(lc) = t.legs.iter().find(|l| l.leg_type == models::LegType::LongCall)  { t.long_strike  = lc.strike; }
    t.short_premium = t.legs.iter().find(|l| l.leg_type.is_short()).map(|l| l.premium).unwrap_or(0.0);
    t.long_premium  = t.legs.iter().find(|l| !l.leg_type.is_short()).map(|l| l.premium).unwrap_or(0.0);

    // Recalculate spread_width and BPR from new legs
    let sw = calculations::compute_spread_width_from_legs(&t.legs);
    t.spread_width = if sw > 0.0 { Some(sw) } else { None };

    let bpr = calculations::calculate_bpr(
        &t.legs, t.credit_received, t.quantity, underlying_for_bpr, t.strategy.as_str(),
    );
    t.bpr = if bpr > 0.0 { Some(bpr) } else { None };

    // ── Exit
    t.exit_date   = parse_date_field(fields, "Exit Date");
    t.exit_reason = field_select_value(fields, "Exit Reason");

    // ── Greeks
    t.delta = field_opt_f64(fields, "Delta");
    t.theta = field_opt_f64(fields, "Theta");
    t.gamma = field_opt_f64(fields, "Gamma");
    t.vega  = field_opt_f64(fields, "Vega");
    t.pop   = field_opt_f64(fields, "POP %");

    // ── Entry conditions
    t.underlying_price          = field_opt_f64(fields, "Underlying $");
    t.iv_rank                   = field_opt_f64(fields, "IVR %");
    t.iv_percentile             = field_opt_f64(fields, "IV Pct %");
    t.vix_at_entry              = field_opt_f64(fields, "VIX at Entry");
    t.implied_volatility        = field_opt_f64(fields, "Impl Vol %");
    t.underlying_price_at_close = field_opt_f64(fields, "Underlying @ Close");

    // ── Live Greek Estimation (if blank)
    if t.delta.is_none() && t.underlying_price.is_some() && t.implied_volatility.is_some() {
        let up = t.underlying_price.unwrap();
        // Normalize IV: user may enter as whole-number % (25.0) or decimal (0.25)
        let iv_raw = t.implied_volatility.unwrap();
        let iv = if iv_raw > 2.0 { iv_raw / 100.0 } else { iv_raw };
        let dte = (t.expiration_date.date_naive() - t.trade_date.date_naive()).num_days().max(1) as i32;
        
        let mut d_total = 0.0;
        let mut t_total = 0.0;
        let mut g_total = 0.0;
        let mut v_total = 0.0;

        for leg in &t.legs {
            let (d, th, g, v) = calculations::estimate_greeks(
                up, leg.strike, dte, 0.045, iv, 
                leg.leg_type.is_call(), leg.leg_type.is_short()
            );
            d_total += d;
            t_total += th;
            g_total += g;
            v_total += v;
        }
        t.delta = Some(d_total);
        t.theta = Some(t_total);
        t.gamma = Some(g_total);
        t.vega  = Some(v_total);
        t.pop   = Some(calculations::estimate_pop(t));
    }

    // ── Meta
    t.commission        = field_opt_f64(fields, "Commission");
    t.target_profit_pct = field_opt_f64(fields, "Target Profit %");
    t.management_rule   = field_select_value(fields, "Management Rule");
    t.trade_grade       = field_select_value(fields, "Grade");
    t.grade_notes       = field_opt_str(fields, "Grade Notes");
    t.entry_reason      = field_opt_str(fields, "Entry Reason");
    t.notes             = field_opt_str(fields, "Notes");
    t.tags = {
        let s = field_str(fields, "Tags (comma)");
        s.split(',').filter(|x| !x.is_empty()).map(|x| x.trim().to_string()).collect()
    };
    t.is_earnings_play = field_str(fields, "Earnings Play") == "true";
    t.is_tested        = field_str(fields, "Is Tested") == "true";
    t.bid_ask_spread_at_entry = field_opt_f64(fields, "Bid-Ask Spread");
    t.fill_vs_mid             = field_opt_f64(fields, "Fill vs Mid");
    t.next_earnings = {
        let s = field_str(fields, "Earnings Date");
        if s.is_empty() { None } else { chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok() }
    };

    // Recalculate P&L if all legs have close_premium (uses per-leg qty for custom)
    let all_closed = !t.legs.is_empty() && t.legs.iter().all(|l| l.close_premium.is_some());
    if all_closed {
        if let Some(pnl) = calculations::calculate_pnl_from_legs(&t.legs, t.quantity, t.commission, t.strategy.as_str()) {
            t.pnl = Some(pnl);
        }
        let close_short: f64 = t.legs.iter().filter(|l| l.leg_type.is_short()).map(|l| l.close_premium.unwrap_or(0.0)).sum();
        let close_long:  f64 = t.legs.iter().filter(|l| !l.leg_type.is_short()).map(|l| l.close_premium.unwrap_or(0.0)).sum();
        t.debit_paid = Some(close_short - close_long);
        if let Some(exit) = t.exit_date {
            t.dte_at_close = Some((t.expiration_date.date_naive() - exit.date_naive()).num_days().max(0) as i32);
        }
    }
}

fn parse_date_field(fields: &[EditField], label: &str) -> Option<chrono::DateTime<Utc>> {
    let s = field_str(fields, label);
    if s.is_empty() { return None; }
    // Try YYYY-MM-DD
    if let Ok(nd) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        let ndt = nd.and_hms_opt(0, 0, 0)?;
        return Some(chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Build close fields
// ─────────────────────────────────────────────────────────────────────────────

fn build_close_fields(t: &models::Trade) -> Vec<EditField> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut f: Vec<EditField> = Vec::new();

    f.push(EditField::date("Exit Date", &today).with_section("── Close Trade ───────────────────────────────────────────────────"));

    let exit_reasons = vec!["closed".to_string(), "expired".to_string(), "rolled".to_string(), "stopped".to_string()];
    f.push(EditField::select("Exit Reason", "0", exit_reasons));
    f.push(EditField::number("Underlying @ Close", &t.underlying_price.map(|v| format!("{:.2}", v)).unwrap_or_default()));
    f.push(EditField::number("IV at Close",    &t.iv_at_close.map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("Delta at Close", &t.delta_at_close.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("Theta at Close", &t.theta_at_close.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("Gamma at Close", &t.gamma_at_close.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("Vega at Close",  &t.vega_at_close.map(|v| format!("{:.4}", v)).unwrap_or_default()));
    f.push(EditField::number("Close Commission",  ""));

    // Assignment tracking
    f.push(EditField::bool_field("Assigned", t.was_assigned)
        .with_section("── Assignment ────────────────────────────────────────────────────"));
    f.push(EditField::number("Assigned Shares",
        &t.assigned_shares.map(|v| v.to_string()).unwrap_or_default()));
    f.push(EditField::number("Cost Basis/Share",
        &t.cost_basis.map(|v| format!("{:.2}", v)).unwrap_or_default()));

    // Exit thesis
    f.push(EditField::text("Close Notes", t.close_notes.as_deref().unwrap_or(""))
        .with_section("── Exit Thesis ─────────────────────────────────────────────────────"));

    // Per-leg close premiums
    f.push(EditField::text("", "").with_section("── Close Premiums (BTC/STC price per contract) ─────────────────────"));
    for leg in &t.legs {
        let leg_label = match leg.leg_type {
            models::LegType::ShortPut  => "BTC Short Put",
            models::LegType::LongPut   => "STC Long Put",
            models::LegType::ShortCall => "BTC Short Call",
            models::LegType::LongCall  => "STC Long Call",
        };
        f.push(EditField::number(&format!("  {leg_label}"), "0.00"));
    }

    f
}

fn apply_close_fields_to_trade(fields: &[EditField], t: &mut models::Trade) {
    t.exit_date   = parse_date_field(fields, "Exit Date");
    t.exit_reason = field_select_value(fields, "Exit Reason");
    t.underlying_price_at_close = field_opt_f64(fields, "Underlying @ Close");
    t.iv_at_close    = field_opt_f64(fields, "IV at Close");
    t.delta_at_close = field_opt_f64(fields, "Delta at Close");
    t.theta_at_close = field_opt_f64(fields, "Theta at Close");
    t.gamma_at_close = field_opt_f64(fields, "Gamma at Close");
    t.vega_at_close  = field_opt_f64(fields, "Vega at Close");
    t.was_assigned   = field_str(fields, "Assigned") == "true";
    t.assigned_shares = fields.iter().find(|f| f.label == "Assigned Shares")
        .and_then(|f| if f.value.is_empty() { None } else { f.value.parse::<i32>().ok() });
    t.cost_basis     = field_opt_f64(fields, "Cost Basis/Share");
    t.close_notes    = field_opt_str(fields, "Close Notes");
    let close_comm = field_opt_f64(fields, "Close Commission").unwrap_or(0.0);

    // Per-leg close premiums
    for leg in t.legs.iter_mut() {
        let leg_label = match leg.leg_type {
            models::LegType::ShortPut  => "BTC Short Put",
            models::LegType::LongPut   => "STC Long Put",
            models::LegType::ShortCall => "BTC Short Call",
            models::LegType::LongCall  => "STC Long Call",
        };
        let cp = field_opt_f64(fields, &format!("  {leg_label}")).unwrap_or(0.0);
        leg.close_premium = Some(cp);
    }

    // Compute P&L — include both entry commission (already stored) and close commission
    let close_short: f64 = t.legs.iter().filter(|l| l.leg_type.is_short()).map(|l| l.close_premium.unwrap_or(0.0)).sum();
    let close_long:  f64 = t.legs.iter().filter(|l| !l.leg_type.is_short()).map(|l| l.close_premium.unwrap_or(0.0)).sum();
    let debit_to_close = close_short - close_long;
    t.debit_paid = Some(debit_to_close);
    let entry_comm = t.commission.unwrap_or(0.0);
    let gross = (t.credit_received - debit_to_close) * 100.0 * t.quantity as f64;
    t.pnl = Some(gross - entry_comm - close_comm);
    t.commission = Some(entry_comm + close_comm);

    // DTE at close
    if let Some(exit) = t.exit_date {
        let dte = (t.expiration_date.date_naive() - exit.date_naive()).num_days().max(0) as i32;
        t.dte_at_close = Some(dte);
    }

    // Auto-set closed_at_target: true when P&L ≥ target_profit_pct% of max profit
    if let Some(pnl) = t.pnl {
        let max_profit = t.credit_received * 100.0 * t.quantity as f64;
        let target_pct = t.target_profit_pct.unwrap_or(50.0);
        if max_profit > 0.0 {
            t.closed_at_target = pnl >= max_profit * (target_pct / 100.0);
        }
    }

    // Auto-grade if no grade set
    if t.trade_grade.is_none() {
        if let Some(pnl) = t.pnl {
            let max_profit = t.credit_received * 100.0 * t.quantity as f64;
            let pct_of_max = if max_profit > 0.0 { pnl / max_profit * 100.0 } else { 0.0 };
            let managed_on_time = t.dte_at_close.map(|d| d >= 21).unwrap_or(false);
            let grade = if pct_of_max >= 85.0 && managed_on_time { "A" }
                else if pct_of_max >= 65.0 { "B" }
                else if pct_of_max >= 40.0 { "C" }
                else if pct_of_max >= 0.0  { "D" }
                else { "F" };
            t.trade_grade = Some(grade.to_string());
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Playbook edit field helpers
// ─────────────────────────────────────────────────────────────────────────────

fn all_spread_type_strings() -> Vec<String> {
    vec![
        "short_put_vertical".to_string(),
        "short_call_vertical".to_string(),
        "iron_condor".to_string(),
        "iron_butterfly".to_string(),
        "strangle".to_string(),
        "straddle".to_string(),
        "cash_secured_put".to_string(),
        "covered_call".to_string(),
        "calendar_spread".to_string(),
        "pmcc".to_string(),
        "long_diagonal_spread".to_string(),
        "short_diagonal_spread".to_string(),
        "long_call_vertical".to_string(),
        "long_put_vertical".to_string(),
        "zebra".to_string(),
        "custom".to_string(),
    ]
}

fn build_playbook_edit_fields(pb: &models::PlaybookStrategy) -> Vec<EditField> {
    let mut f: Vec<EditField> = Vec::new();
    let spread_types = all_spread_type_strings();

    // Strategy section
    let type_idx = pb.spread_type.as_deref()
        .and_then(|st| spread_types.iter().position(|s| s == st))
        .unwrap_or(0);

    f.push(EditField::text("Name", &pb.name)
        .with_section("── Strategy ─────────────────────────────────────────────────────"));
    f.push(EditField::select("Type", &type_idx.to_string(), spread_types));

    // Entry Checklist section
    let ec = pb.entry_criteria.as_ref();
    f.push(EditField::number("IVR Min",
        &ec.and_then(|e| e.min_ivr).map(|v| format!("{:.1}", v)).unwrap_or_default())
        .with_section("── Entry Checklist ──────────────────────────────────────────────"));
    f.push(EditField::number("IVR Max",
        &ec.and_then(|e| e.max_ivr).map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("Delta Min",
        &ec.and_then(|e| e.min_delta).map(|v| format!("{:.2}", v)).unwrap_or_default()));
    f.push(EditField::number("Delta Max",
        &ec.and_then(|e| e.max_delta).map(|v| format!("{:.2}", v)).unwrap_or_default()));
    f.push(EditField::number("DTE Min",
        &ec.and_then(|e| e.min_dte).map(|v| v.to_string()).unwrap_or_default()));
    f.push(EditField::number("DTE Max",
        &ec.and_then(|e| e.max_dte).map(|v| v.to_string()).unwrap_or_default()));
    f.push(EditField::number("Max Alloc %",
        &ec.and_then(|e| e.max_allocation_pct).map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("Target Profit %",
        &ec.and_then(|e| e.target_profit_pct).map(|v| format!("{:.0}", v)).unwrap_or_default()));
    f.push(EditField::text("Exit Rule",
        &ec.and_then(|e| e.management_rule.clone()).unwrap_or_default()));
    f.push(EditField::number("Min POP %",
        &ec.and_then(|e| e.min_pop).map(|v| format!("{:.0}", v)).unwrap_or_default()));
    f.push(EditField::number("VIX Min",
        &ec.and_then(|e| e.vix_min).map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("VIX Max",
        &ec.and_then(|e| e.vix_max).map(|v| format!("{:.1}", v)).unwrap_or_default()));
    f.push(EditField::number("Max BPR %",
        &ec.and_then(|e| e.max_bpr_pct).map(|v| format!("{:.1}", v)).unwrap_or_default()));

    // Structured exit ladder
    f.push(EditField::number("Stop Loss %",
        &ec.and_then(|e| e.stop_loss_pct).map(|v| format!("{:.0}", v)).unwrap_or_default())
        .with_section("── Exit Ladder ──────────────────────────────────────────────────"));
    f.push(EditField::number("Profit Target %",
        &ec.and_then(|e| e.profit_target_pct).map(|v| format!("{:.0}", v)).unwrap_or_default()));
    f.push(EditField::number("DTE Exit",
        &ec.and_then(|e| e.dte_exit).map(|v| v.to_string()).unwrap_or_default()));

    // Avoidance conditions
    f.push(EditField::text("When to Avoid",
        &ec.and_then(|e| e.when_to_avoid.clone()).unwrap_or_default())
        .with_section("── Avoidance ────────────────────────────────────────────────────"));
    f.push(EditField::text("Notes",
        &ec.and_then(|e| e.notes.clone()).unwrap_or_default()));

    // Thesis section
    f.push(EditField::multiline("Thesis",
        &pb.description.clone().unwrap_or_default())
        .with_section("── Thesis ───────────────────────────────────────────────────────"));

    f
}

fn build_playbook_from_edit_fields_fn(fields: &[EditField]) -> models::PlaybookStrategy {
    let name = field_str(fields, "Name");

    // Spread type: read selected index → map to string
    let spread_type = fields.iter().find(|f| f.label == "Type").and_then(|f| {
        if let FieldKind::Select(opts) = &f.kind {
            let idx = f.value.parse::<usize>().unwrap_or(0);
            opts.get(idx).cloned()
        } else {
            None
        }
    });

    let description = field_opt_str(fields, "Thesis");

    let min_ivr              = field_opt_f64(fields, "IVR Min");
    let max_ivr              = field_opt_f64(fields, "IVR Max");
    let min_delta            = field_opt_f64(fields, "Delta Min");
    let max_delta            = field_opt_f64(fields, "Delta Max");
    let min_dte              = fields.iter().find(|f| f.label == "DTE Min")
        .and_then(|f| if f.value.is_empty() { None } else { f.value.parse::<i32>().ok() });
    let max_dte              = fields.iter().find(|f| f.label == "DTE Max")
        .and_then(|f| if f.value.is_empty() { None } else { f.value.parse::<i32>().ok() });
    let max_allocation_pct   = field_opt_f64(fields, "Max Alloc %");
    let target_profit_pct    = field_opt_f64(fields, "Target Profit %");
    let management_rule      = field_opt_str(fields, "Exit Rule");
    let min_pop              = field_opt_f64(fields, "Min POP %");
    let vix_min              = field_opt_f64(fields, "VIX Min");
    let vix_max              = field_opt_f64(fields, "VIX Max");
    let max_bpr_pct          = field_opt_f64(fields, "Max BPR %");
    let stop_loss_pct        = field_opt_f64(fields, "Stop Loss %");
    let profit_target_pct    = field_opt_f64(fields, "Profit Target %");
    let dte_exit             = fields.iter().find(|f| f.label == "DTE Exit")
        .and_then(|f| if f.value.is_empty() { None } else { f.value.parse::<i32>().ok() });
    let when_to_avoid        = field_opt_str(fields, "When to Avoid");
    let ec_notes             = field_opt_str(fields, "Notes");

    let has_criteria = min_ivr.is_some() || max_ivr.is_some()
        || min_delta.is_some() || max_delta.is_some()
        || min_dte.is_some() || max_dte.is_some()
        || max_allocation_pct.is_some() || target_profit_pct.is_some()
        || management_rule.is_some() || min_pop.is_some()
        || vix_min.is_some() || vix_max.is_some() || max_bpr_pct.is_some()
        || stop_loss_pct.is_some() || profit_target_pct.is_some() || dte_exit.is_some()
        || when_to_avoid.is_some() || ec_notes.is_some();

    let entry_criteria = if has_criteria {
        Some(models::EntryCriteria {
            min_ivr,
            max_ivr,
            min_delta,
            max_delta,
            min_dte,
            max_dte,
            max_allocation_pct,
            target_profit_pct,
            management_rule,
            min_pop,
            vix_min,
            vix_max,
            max_bpr_pct,
            stop_loss_pct,
            profit_target_pct,
            dte_exit,
            when_to_avoid,
            notes: ec_notes,
        })
    } else {
        None
    };

    models::PlaybookStrategy {
        id: 0,
        name,
        description,
        spread_type,
        entry_criteria,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin settings helpers
// ─────────────────────────────────────────────────────────────────────────────

fn build_admin_settings_fields(account_size: f64, max_heat_pct: f64, max_pos_bpr_pct: f64, target_undefined_pct: f64, monthly_pnl_target: f64, drawdown_circuit_breaker_pct: f64, default_profit_target_pct: f64, default_mgmt_dte: i32, risk_free_rate_pct: f64, rolling_window: usize) -> Vec<EditField> {
    vec![
        EditField::number("Account Size", &format!("{:.2}", account_size))
            .with_section("── Account ───────────────────────────────────────────────────────"),
        EditField::number("Monthly P&L Target ($)", &format!("{:.2}", monthly_pnl_target)),
        EditField::number("Max Heat %",         &format!("{:.1}", max_heat_pct))
            .with_section("── Risk Limits (tastytrade defaults) ─────────────────────────────"),
        EditField::number("Max Pos BPR %",      &format!("{}", max_pos_bpr_pct)),
        EditField::number("Target Undefined %", &format!("{:.1}", target_undefined_pct)),
        EditField::number("Drawdown Breaker %", &format!("{:.1}", drawdown_circuit_breaker_pct)),
        EditField::number("Default Profit Target %", &format!("{:.1}", default_profit_target_pct))
            .with_section("── Trade Management ─────────────────────────────────────────────"),
        EditField::number("Default Mgmt DTE", &format!("{}", default_mgmt_dte)),
        EditField::number("Risk-Free Rate %", &format!("{:.2}", risk_free_rate_pct))
            .with_section("── Performance Benchmarks ──────────────────────────────────────"),
        EditField::number("Rolling Window (trades)", &format!("{}", rolling_window)),
    ]
}

fn apply_admin_fields(fields: &[EditField], storage: &storage::Storage)
    -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<i32>, Option<f64>, Option<usize>)
{
    let mut new_account       = None;
    let mut new_max_heat      = None;
    let mut new_max_pos_bpr   = None;
    let mut new_target_undef  = None;
    let mut new_monthly_tgt   = None;
    let mut new_drawdown_cb   = None;
    let mut new_profit_target = None;
    let mut new_mgmt_dte      = None;
    let mut new_rf_rate       = None;
    let mut new_rolling_window = None;
    for field in fields {
        match field.label.as_str() {
            "Account Size" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("account_size", &v.to_string());
                    new_account = Some(v);
                }
            }
            "Monthly P&L Target ($)" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("monthly_pnl_target", &v.to_string());
                    new_monthly_tgt = Some(v);
                }
            }
            "Max Heat %" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("max_heat_pct", &v.to_string());
                    new_max_heat = Some(v);
                }
            }
            "Max Pos BPR %" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("max_pos_bpr_pct", &v.to_string());
                    new_max_pos_bpr = Some(v);
                }
            }
            "Target Undefined %" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("target_undefined_pct", &v.to_string());
                    new_target_undef = Some(v);
                }
            }
            "Drawdown Breaker %" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("drawdown_circuit_breaker_pct", &v.to_string());
                    new_drawdown_cb = Some(v);
                }
            }
            "Default Profit Target %" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("default_profit_target_pct", &v.to_string());
                    new_profit_target = Some(v);
                }
            }
            "Default Mgmt DTE" => {
                if let Ok(v) = field.value.parse::<i32>() {
                    let _ = storage.set_setting("default_mgmt_dte", &v.to_string());
                    new_mgmt_dte = Some(v);
                }
            }
            "Risk-Free Rate %" => {
                if let Ok(v) = field.value.parse::<f64>() {
                    let _ = storage.set_setting("risk_free_rate_pct", &v.to_string());
                    new_rf_rate = Some(v);
                }
            }
            "Rolling Window (trades)" => {
                if let Ok(v) = field.value.parse::<usize>() {
                    let _ = storage.set_setting("rolling_window", &v.to_string());
                    new_rolling_window = Some(v);
                }
            }
            _ => {}
        }
    }
    (new_account, new_max_heat, new_max_pos_bpr, new_target_undef, new_monthly_tgt, new_drawdown_cb, new_profit_target, new_mgmt_dte, new_rf_rate, new_rolling_window)
}

// ─────────────────────────────────────────────────────────────────────────────
// Column visibility helpers
// ─────────────────────────────────────────────────────────────────────────────

fn parse_col_visibility(s: &str) -> [bool; 22] {
    let mut vis = [true; 22];
    let chars: Vec<char> = s.chars().collect();
    let padded: Vec<char> = if chars.len() == 17 {
        // Legacy 17-char: insert BPR at pos 9, BPR% at pos 10
        let mut p = chars;
        p.insert(9, '1');  // BPR defaults ON
        p.insert(10, '1'); // BPR% defaults ON
        p
    } else if chars.len() == 18 {
        // Previous 18-char: insert BPR% at pos 10
        let mut p = chars;
        p.insert(10, '1'); // BPR% defaults ON
        p
    } else {
        chars
    };
    // Pad to 22 with col 21 (Mgmt) defaulting to ON
    let mut padded = padded;
    while padded.len() < 22 {
        padded.push('1');
    }
    for (i, ch) in padded.iter().take(22).enumerate() {
        vis[i] = *ch != '0';
    }
    vis
}

fn col_visibility_to_string(vis: &[bool; 22]) -> String {
    vis.iter().map(|&b| if b { '1' } else { '0' }).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Perf tab cursor helpers
// ─────────────────────────────────────────────────────────────────────────────

fn perf_section_count(subtab: usize) -> usize {
    match subtab {
        0 => 2,  // health, returns
        1 => 1,  // growth chart
        _ => 11, // analytics
    }
}

/// Maps (subtab, cursor) → `perf_collapsed` global index.
/// Overview map: [0=Health, 1=Returns]
/// Charts map:   [3=Growth]
/// Analytics map: [2=Advanced, 4=Strategy, 5=Ticker, 6=Monthly, 7=IVR, 8=VIX, 9=DTE, 10=IVREntry, 11=PnlDist, 12=Held, 13=Commission]
fn perf_cursor_to_gi(subtab: usize, cursor: usize) -> Option<usize> {
    const OVERVIEW_MAP:  [usize; 2]  = [0, 1];
    const CHARTS_MAP:    [usize; 1]  = [3];
    const ANALYTICS_MAP: [usize; 11] = [2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    match subtab {
        0 => OVERVIEW_MAP.get(cursor).copied(),
        1 => CHARTS_MAP.get(cursor).copied(),
        2 => ANALYTICS_MAP.get(cursor).copied(),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // ── DB init (before TUI — keeps terminal output readable)
    let db_path = std::env::args().nth(1).unwrap_or_else(|| "trades.db".to_string());
    let storage = storage::Storage::new(&db_path)?;
    // One-time migrations / seed data
    let _ = storage.migrate_zebra_type();
    let _ = storage.ensure_ratio_spread_playbook();
    let _ = storage.ensure_put_bwb_playbook();
    let _ = storage.backfill_ivr_all_trades();
    let mut app = AppState::new(&storage);

    // ── Yahoo Finance startup fetch (10 s timeout)
    {
        let open_tickers: Vec<String> = {
            let mut t: Vec<String> = app.trades.iter()
                .filter(|t| t.is_open())
                .map(|t| t.ticker.clone())
                .collect();
            t.sort();
            t.dedup();
            t
        };

        if !open_tickers.is_empty() {
            let sector_tickers: Vec<String> = {
                let mut t: Vec<String> = app.trades.iter()
                    .filter(|t| t.is_open() && t.sector.is_none())
                    .map(|t| t.ticker.clone())
                    .collect();
                t.sort(); t.dedup();
                t
            };
            let fetch = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                async {
                    tokio::join!(
                        yahoo::fetch_earnings_dates(&open_tickers),
                        yahoo::fetch_vix(),
                        yahoo::fetch_betas(&open_tickers),
                        yahoo::fetch_spy_price(),
                        yahoo::fetch_underlying_prices(&open_tickers),
                        yahoo::fetch_sectors(&sector_tickers),
                    )
                },
            )
            .await;

            if let Ok((earnings_map, vix_val, beta_map, spy_val, prices, sector_map)) = fetch {
                // Collect trades that need updating
                let updates: Vec<(i32, chrono::NaiveDate)> = app.trades.iter()
                    .filter(|t| t.is_open())
                    .filter_map(|t| earnings_map.get(&t.ticker).map(|&ed| (t.id, ed)))
                    .collect();
                // Apply updates to in-memory trades and persist to DB
                for (id, ed) in updates {
                    if let Some(trade) = app.trades.iter_mut().find(|t| t.id == id) {
                        trade.next_earnings = Some(ed);
                        let _ = storage.update_trade(id, &*trade);
                    }
                }
                // Persist sector for open trades that were missing it
                let sector_updates: Vec<(i32, String)> = app.trades.iter()
                    .filter(|t| t.is_open() && t.sector.is_none())
                    .filter_map(|t| sector_map.get(&t.ticker).map(|s| (t.id, s.clone())))
                    .collect();
                for (id, sec) in sector_updates {
                    if let Some(trade) = app.trades.iter_mut().find(|t| t.id == id) {
                        trade.sector = Some(sec);
                        let _ = storage.update_trade(id, &*trade);
                    }
                }
                app.current_vix = vix_val;
                app.beta_map    = beta_map;
                app.spy_price   = spy_val;
                app.live_prices = prices;
                app.stats = calculations::build_portfolio_stats(
                    &app.trades, app.account_size, app.current_vix,
                    &app.beta_map, app.spy_price, app.target_undefined_pct,
                    app.monthly_pnl_target,
                );
                app.alerts = theta_vault_rust::actions::compute_alerts(
                    &app.trades, &app.live_prices, app.account_size, app.current_vix,
                    app.stats.net_beta_weighted_delta, app.stats.drift, app.stats.target_undefined_pct,
                    app.stats.max_drawdown_pct, app.drawdown_circuit_breaker_pct,
                    app.default_profit_target_pct, app.default_mgmt_dte,
                );
                if !app.alerts.is_empty() {
                    app.actions_list_state.select(Some(0));
                }
            }
        }
    }

    // ── SPY monthly returns fetch (always runs, regardless of open positions)
    {
        use chrono::Datelike;
        let (spy_start_year, spy_start_month) = app.trades.iter()
            .filter_map(|t| t.exit_date)
            .map(|d| { let n = d.date_naive(); (n.year(), n.month()) })
            .min_by(|a, b| a.cmp(b))
            .unwrap_or_else(|| { let n = chrono::Utc::now().date_naive(); (n.year() - 1, n.month()) });
        if let Ok(map) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            yahoo::fetch_spy_monthly_returns(spy_start_year, spy_start_month),
        ).await {
            app.spy_monthly = map;
        }
    }

    // ── TUI init
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    loop {
        // Compute trade count for display (Trade rows only, not headers)
        let display_count = app.visual_rows.iter().filter(|r| matches!(r, VisualRowKind::Trade(_))).count();

        // Refresh thesis scroll limit
        if app.selected_tab == 2 {
            if let Ok(size) = term.size() {
                let inner_w = (size.width as usize * 72 / 100).saturating_sub(2); // 72% panel - borders
                let inner_h = size.height.saturating_sub(17) as usize; // tabs(3)+footer(1)+entry(5)+stats(6)+borders(2)
                let desc = app.playbooks
                    .get(app.playbook_state.selected().unwrap_or(0))
                    .and_then(|pb| pb.description.as_deref())
                    .unwrap_or("No description provided.");
                let total = theta_vault_rust::ui::count_thesis_lines(desc, inner_w);
                app.thesis_max_scroll = total.saturating_sub(inner_h) as u16;
            }
        }

        // Refresh dashboard open positions and risk distribution scroll limits
        if app.selected_tab == 0 {
            if let Ok(size) = term.size() {
                let content_h    = size.height.saturating_sub(4) as usize; // tabs(3)+footer(1)
                let lower_h      = content_h.saturating_sub(9);            // top KPI row
                let open_panel_h = lower_h * 60 / 100;                     // Percentage(60)
                let visible_rows = open_panel_h.saturating_sub(3);         // borders+header
                let open_count   = app.trades.iter().filter(|t| t.is_open()).count();
                app.dash_open_max_scroll = open_count.saturating_sub(visible_rows);
                // Risk distribution panel: same height as open panel, minus borders
                let risk_visible_h = lower_h.saturating_sub(2);
                app.dash_risk_max_scroll = theta_vault_rust::ui::count_risk_lines(&app.trades, &app.stats)
                    .saturating_sub(risk_visible_h);
            }
        }

        // Refresh perf scroll limits (overview + analytics)
        if app.selected_tab == 5 {
            if let Ok(size) = term.size() {
                let content_w = size.width.saturating_sub(2) as usize;
                let visible_h = size.height.saturating_sub(8) as usize;

                app.perf_overview_max_scroll = theta_vault_rust::ui::count_perf_overview_lines(
                    &app.stats, &app.perf_stats, content_w, &app.perf_collapsed
                ).saturating_sub(visible_h) as u16;

                app.perf_analytics_max_scroll = theta_vault_rust::ui::count_perf_analytics_lines(
                    &app.stats, &app.perf_stats, content_w, &app.perf_collapsed, &app.spy_monthly
                ).saturating_sub(visible_h) as u16;

                // Sync scroll to cursor position after cursor movement
                if app.perf_scroll_dirty {
                    app.perf_scroll_dirty = false;
                    let target = theta_vault_rust::ui::perf_header_scroll_for_cursor(
                        app.perf_section_cursor,
                        app.perf_subtab,
                        &app.perf_collapsed,
                        &app.stats,
                        &app.perf_stats,
                        &app.spy_monthly,
                        content_w,
                    );
                    if app.perf_subtab == 0 {
                        app.perf_overview_scroll = target.min(app.perf_overview_max_scroll);
                    } else if app.perf_subtab == 2 {
                        app.perf_analytics_scroll = target.min(app.perf_analytics_max_scroll);
                    }
                    // subtab 1 (Charts) has no text scroll — no-op
                }
            }
        }

        // Refresh detail line count + max scroll
        if app.show_detail {
            if let Some(i) = app.table_state.selected() {
                if let Some(VisualRowKind::Trade(ti)) = app.visual_rows.get(i) {
                    let left_n  = theta_vault_rust::ui::count_detail_lines_left(&app.trades[*ti], &app.selected_trade_chain, &app.playbooks);
                    let right_n = theta_vault_rust::ui::count_detail_lines_right(&app.trades[*ti], &app.selected_trade_chain, &app.playbooks);
                    app.detail_total_lines = left_n.max(right_n);
                    if let Ok(size) = term.size() {
                        // Detail panel = 45% of content area (tabs+footer = 4 rows), minus 2 borders
                        let content_h = size.height.saturating_sub(4) as usize;
                        let detail_h  = content_h * 45 / 100;
                        let visible   = detail_h.saturating_sub(2);
                        app.detail_max_scroll = app.detail_total_lines.saturating_sub(visible) as u16;
                    }
                }
            }
        }

        // VIX-adaptive max heat: use tastytrade regime target, capped by user's ceiling
        let effective_max_heat = app.stats.vix
            .map(|v| calculations::vix_max_heat(v).min(app.max_heat_pct))
            .unwrap_or(app.max_heat_pct);

        term.draw(|f| ui::draw_ui(
            f,
            display_count,
            &app.visual_rows,
            &app.collapsed_months,
            &app.trades,
            &app.stats,
            &app.perf_stats,
            app.perf_subtab,
            app.perf_overview_scroll,
            app.perf_analytics_scroll,
            &app.playbooks,
            app.selected_tab,
            &mut app.table_state,
            &mut app.playbook_state,
            app.thesis_scroll,
            app.show_detail,
            app.detail_scroll,
            &app.selected_trade_chain,
            app.dash_open_scroll,
            app.filter_status,
            &app.filter_ticker,
            app.sort_key,
            app.sort_desc,
            app.app_mode,
            &app.edit_fields,
            app.edit_field_idx,
            app.edit_scroll,
            &app.close_fields,
            app.close_field_idx,
            app.delete_trade_id,
            &app.playbook_edit_fields,
            app.playbook_edit_field_idx,
            app.playbook_edit_scroll,
            &app.alerts,
            &mut app.actions_list_state,
            &app.collapsed_action_kinds,
            app.pulse_on,
            app.under_tauri,
            app.dash_kpi_popup,
            app.dash_kpi_scroll,
            &mut app.dash_kpi_max_scroll,
            app.perf_kpi_popup,
            app.perf_kpi_scroll,
            &mut app.perf_kpi_max_scroll,
            app.journal_help_popup,
            app.journal_help_scroll,
            &mut app.journal_help_max_scroll,
            app.journal_help_page,
            effective_max_heat,
            app.max_heat_pct,
            app.max_pos_bpr_pct,
            &app.admin_fields,
            app.admin_field_idx,
            app.admin_scroll,
            app.cal_year,
            app.cal_month,
            app.cal_day,
            &app.thesis_edit_buf,
            &app.spy_monthly,
            &app.live_prices,
            &app.perf_collapsed,
            app.perf_section_cursor,
            &app.col_visibility,
            app.show_col_picker,
            app.journal_chain_view,
            app.default_profit_target_pct,
            app.default_mgmt_dte,
            app.export_status.as_deref(),
            app.dash_risk_scroll,
            app.dash_panel_focus,
            &app.playbook_analytics,
            &app.journal_note_buf,
            app.journal_note_trade_id,
        ))?;
        // Decrement export status tick
        if app.export_status_tick > 0 {
            app.export_status_tick -= 1;
            if app.export_status_tick == 0 {
                app.export_status = None;
            }
        }

        let has_event = event::poll(std::time::Duration::from_millis(750))?;
        if !has_event {
            app.pulse_on = !app.pulse_on;
            continue;
        }
        if let Event::Key(key) = event::read()? {
            // ── Filter input mode ────────────────────────────────────────────
            if app.app_mode == AppMode::FilterInput {
                match key.code {
                    KeyCode::Esc => {
                        app.app_mode = AppMode::Normal;
                    }
                    KeyCode::Enter => {
                        app.app_mode = AppMode::Normal;
                        app.rebuild_visual_rows();
                        let len = app.visual_rows.len();
                        app.table_state.select(if len == 0 { None } else { Some(0) });
                    }
                    KeyCode::Backspace => {
                        app.filter_ticker.pop();
                        app.rebuild_visual_rows();
                        let len = app.visual_rows.len();
                        app.table_state.select(if len == 0 { None } else { Some(0) });
                    }
                    KeyCode::Char(c) => {
                        app.filter_ticker.push(c);
                        app.rebuild_visual_rows();
                        let len = app.visual_rows.len();
                        app.table_state.select(if len == 0 { None } else { Some(0) });
                    }
                    _ => {}
                }
                continue;
            }

            // ── Popups (KPIs, Help) ──────────────────────────────────────────────────
            if app.dash_kpi_popup && app.selected_tab == 0 {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => {
                        app.dash_kpi_popup = false;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.dash_kpi_scroll = app.dash_kpi_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.dash_kpi_scroll < app.dash_kpi_max_scroll {
                            app.dash_kpi_scroll += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        app.dash_kpi_scroll = app.dash_kpi_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        app.dash_kpi_scroll = std::cmp::min(
                            app.dash_kpi_scroll + 10,
                            app.dash_kpi_max_scroll,
                        );
                    }
                    _ => {}
                }
                continue;
            }

            if app.perf_kpi_popup && app.selected_tab == 5 {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => {
                        app.perf_kpi_popup = false;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.perf_kpi_scroll = app.perf_kpi_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.perf_kpi_scroll < app.perf_kpi_max_scroll {
                            app.perf_kpi_scroll += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        app.perf_kpi_scroll = app.perf_kpi_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        app.perf_kpi_scroll = std::cmp::min(
                            app.perf_kpi_scroll + 10,
                            app.perf_kpi_max_scroll,
                        );
                    }
                    _ => {}
                }
                continue;
            }

            if app.journal_help_popup && app.selected_tab == 1 {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => {
                        app.journal_help_popup = false;
                        app.journal_help_page = 0;
                    }
                    KeyCode::Tab => {
                        app.journal_help_page = 1 - app.journal_help_page;
                        app.journal_help_scroll = 0;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.journal_help_scroll > 0 {
                            app.journal_help_scroll -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.journal_help_scroll < app.journal_help_max_scroll {
                            app.journal_help_scroll += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        app.journal_help_scroll = app.journal_help_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        app.journal_help_scroll = std::cmp::min(
                            app.journal_help_scroll + 10,
                            app.journal_help_max_scroll
                        );
                    }
                    _ => {}
                }
                continue;
            }

            // ── Date picker mode ─────────────────────────────────────────────
            if app.app_mode == AppMode::DatePicker {
                match key.code {
                    KeyCode::Esc                         => { app.app_mode = app.cal_from_mode; }
                    KeyCode::Enter                       => app.cal_confirm_selection(),
                    KeyCode::Left  | KeyCode::Char('h') => app.cal_move_days(-1),
                    KeyCode::Right | KeyCode::Char('l') => app.cal_move_days(1),
                    KeyCode::Up    | KeyCode::Char('k') => app.cal_move_days(-7),
                    KeyCode::Down  | KeyCode::Char('j') => app.cal_move_days(7),
                    KeyCode::Char('[') | KeyCode::PageUp   => app.cal_move_months(-1),
                    KeyCode::Char(']') | KeyCode::PageDown => app.cal_move_months(1),
                    _ => {}
                }
                continue;
            }

            // ── Edit trade mode ──────────────────────────────────────────────
            if app.app_mode == AppMode::EditTrade {
                match key.code {
                    KeyCode::Esc => app.cancel_mode(),
                    KeyCode::Tab | KeyCode::Down => app.nav_down(),
                    KeyCode::BackTab | KeyCode::Up => app.nav_up(),
                    KeyCode::Enter => {
                        let kind = app.edit_fields.get(app.edit_field_idx).map(|f| f.kind.clone());
                        match kind {
                            Some(FieldKind::Date)      => app.open_date_picker(true),
                            Some(FieldKind::Multiline) => { if let Some(f) = app.edit_fields.get_mut(app.edit_field_idx) { f.value.push('\n'); } }
                            Some(FieldKind::Button(_)) => app.add_leg_to_edit_fields(),
                            _ => {}
                        }
                    }
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Save
                        if let Some(tid) = app.edit_trade_id {
                            if let Some(orig) = app.trades.iter().find(|t| t.id == tid).cloned() {
                                let mut updated = app.build_updated_trade(&orig);
                                // Auto-compute IVR if implied_volatility is set and iv_rank is blank
                                if updated.iv_rank.is_none() {
                                    if let Some(iv) = updated.implied_volatility {
                                        if let Ok(Some(ivr)) = storage.compute_ivr_for_ticker(&updated.ticker, iv) {
                                            updated.iv_rank = Some(ivr);
                                        }
                                    }
                                }
                                let _ = storage.update_trade(tid, &updated);
                                app.cancel_mode();
                                app.reload(&storage);
                            }
                        }
                    }
                    KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.add_leg_to_edit_fields();
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.delete_focused_leg_from_edit_fields();
                    }
                    KeyCode::Backspace => {
                        if let Some(f) = app.edit_fields.get_mut(app.edit_field_idx) {
                            if matches!(f.kind, FieldKind::Text | FieldKind::Number | FieldKind::Multiline | FieldKind::Date) {
                                f.value.pop();
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        // Detect if Strategy SELECT is focused before the char changes it
                        let strategy_was_focused = app.edit_fields.get(app.edit_field_idx)
                            .map(|f| f.label == "Strategy")
                            .unwrap_or(false);
                        app.edit_key_char(c);
                        // Rebuild legs when strategy changes (preserves existing leg values)
                        if strategy_was_focused && (c == '+' || c == '-' || c == ' ') {
                            app.rebuild_legs_in_edit_fields();
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // ── Close trade mode ─────────────────────────────────────────────
            if app.app_mode == AppMode::CloseTrade {
                match key.code {
                    KeyCode::Esc => app.cancel_mode(),
                    KeyCode::Tab | KeyCode::Down => app.nav_down(),
                    KeyCode::BackTab | KeyCode::Up => app.nav_up(),
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(tid) = app.close_trade_id {
                            if let Some(orig) = app.trades.iter().find(|t| t.id == tid).cloned() {
                                let closed = app.build_closed_trade(&orig);
                                let _ = storage.update_trade(tid, &closed);
                                app.cancel_mode();
                                app.reload(&storage);
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let kind = app.close_fields.get(app.close_field_idx).map(|f| f.kind.clone());
                        if matches!(kind, Some(FieldKind::Date)) { app.open_date_picker(false); }
                    }
                    KeyCode::Backspace => app.close_key_backspace(),
                    KeyCode::Char(c)   => app.close_key_char(c),
                    _ => {}
                }
                continue;
            }

            // ── Confirm delete mode ──────────────────────────────────────────
            if app.app_mode == AppMode::ConfirmDelete {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        if let Some(tid) = app.delete_trade_id {
                            let _ = storage.delete_trade(tid);
                            app.cancel_mode();
                            app.reload(&storage);
                        }
                    }
                    _ => app.cancel_mode(),
                }
                continue;
            }

            // ── Analyze trade mode (payoff chart) ────────────────────────────
            if app.app_mode == AppMode::AnalyzeTrade {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('a') | KeyCode::Char('A') => {
                        {
                            use std::io::Write;
                            let _ = std::io::stdout().lock().write_all(b"\x1b]9999;THETAVAULT_CHART_CLOSE\x07");
                            let _ = std::io::stdout().lock().flush();
                        }
                        app.cancel_mode();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = app.table_state.selected().unwrap_or(0);
                        if i > 0 { 
                            app.table_state.select(Some(i - 1));
                            if let Some(t) = app.selected_trade_cloned() {
                                app.start_analyze(&t);
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = app.table_state.selected().unwrap_or(0);
                        if i + 1 < app.visual_rows.len() {
                            app.table_state.select(Some(i + 1));
                            if let Some(t) = app.selected_trade_cloned() {
                                app.start_analyze(&t);
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // ── Admin settings mode ──────────────────────────────────────────
            if app.app_mode == AppMode::AdminSettings {
                match key.code {
                    KeyCode::Esc => app.cancel_mode(),
                    KeyCode::Tab | KeyCode::Down => {
                        if app.admin_field_idx + 1 < app.admin_fields.len() {
                            app.admin_field_idx += 1;
                            app.sync_admin_scroll();
                        }
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        if app.admin_field_idx > 0 {
                            app.admin_field_idx -= 1;
                            app.sync_admin_scroll();
                        }
                    }
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let fields = app.admin_fields.clone();
                        let (new_acct, new_heat, new_pos_bpr, new_undef, new_monthly, new_drawdown_cb, new_profit_target, new_mgmt_dte, new_rf_rate, new_rolling_window) = apply_admin_fields(&fields, &storage);
                        if let Some(v) = new_acct          { app.account_size                  = v; }
                        if let Some(v) = new_heat          { app.max_heat_pct                  = v; }
                        if let Some(v) = new_pos_bpr       { app.max_pos_bpr_pct               = v; }
                        if let Some(v) = new_undef         { app.target_undefined_pct          = v; }
                        if let Some(v) = new_monthly       { app.monthly_pnl_target            = v; }
                        if let Some(v) = new_drawdown_cb   { app.drawdown_circuit_breaker_pct  = v; }
                        if let Some(v) = new_profit_target { app.default_profit_target_pct     = v; }
                        if let Some(v) = new_mgmt_dte      { app.default_mgmt_dte              = v; }
                        if let Some(v) = new_rf_rate       { app.risk_free_rate_pct            = v; }
                        if let Some(v) = new_rolling_window { app.rolling_window              = v; }
                        app.cancel_mode();
                        app.reload(&storage);
                    }
                    KeyCode::Backspace => {
                        if let Some(f) = app.admin_fields.get_mut(app.admin_field_idx) {
                            f.value.pop();
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(f) = app.admin_fields.get_mut(app.admin_field_idx) {
                            if matches!(f.kind, FieldKind::Number | FieldKind::Text) {
                                f.value.push(c);
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // ── Edit thesis mode ─────────────────────────────────────────────
            if app.app_mode == AppMode::EditThesis {
                match key.code {
                    KeyCode::Esc => { app.app_mode = AppMode::Normal; }
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.save_thesis(&storage);
                    }
                    KeyCode::Enter => { app.thesis_edit_buf.push('\n'); }
                    KeyCode::Backspace => { app.thesis_edit_buf.pop(); }
                    KeyCode::Char(c) => { app.thesis_edit_buf.push(c); }
                    _ => {}
                }
                continue;
            }

            // ── JournalNote mode (Actions tab, N key) ────────────────────────
            if app.app_mode == AppMode::JournalNote {
                match key.code {
                    KeyCode::Esc => {
                        app.app_mode = AppMode::Normal;
                        app.journal_note_buf.clear();
                        app.journal_note_trade_id = None;
                    }
                    KeyCode::Enter => {
                        // Append timestamped note to the trade's notes field
                        let note = app.journal_note_buf.trim().to_string();
                        if !note.is_empty() {
                            if let Some(tid) = app.journal_note_trade_id {
                                if let Some(trade) = app.trades.iter_mut().find(|t| t.id == tid) {
                                    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
                                    let tagged = format!("[{}] {}", ts, note);
                                    trade.notes = Some(match &trade.notes {
                                        Some(existing) if !existing.is_empty() => {
                                            format!("{}\n{}", existing, tagged)
                                        }
                                        _ => tagged,
                                    });
                                    let _ = storage.update_trade(tid, trade);
                                }
                            }
                        }
                        app.app_mode = AppMode::Normal;
                        app.journal_note_buf.clear();
                        app.journal_note_trade_id = None;
                        app.reload(&storage);
                    }
                    KeyCode::Backspace => { app.journal_note_buf.pop(); }
                    KeyCode::Char(c)   => { app.journal_note_buf.push(c); }
                    _ => {}
                }
                continue;
            }

            // ── Edit playbook mode ───────────────────────────────────────────
            if app.app_mode == AppMode::EditPlaybook {
                match key.code {
                    KeyCode::Esc => app.cancel_mode(),
                    KeyCode::Tab | KeyCode::Down => {
                        if app.playbook_edit_field_idx + 1 < app.playbook_edit_fields.len() {
                            app.playbook_edit_field_idx += 1;
                            app.sync_playbook_edit_scroll();
                        }
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        if app.playbook_edit_field_idx > 0 {
                            app.playbook_edit_field_idx -= 1;
                            app.sync_playbook_edit_scroll();
                        }
                    }
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let pb = app.build_playbook_from_edit_fields();
                        if let Some(id) = app.edit_playbook_id {
                            let _ = storage.update_playbook(id, &pb);
                        } else {
                            let _ = storage.insert_playbook(&pb);
                        }
                        app.cancel_mode();
                        app.reload(&storage);
                    }
                    KeyCode::Enter => {
                        if let Some(f) = app.playbook_edit_fields.get_mut(app.playbook_edit_field_idx) {
                            if matches!(f.kind, FieldKind::Multiline) {
                                f.value.push('\n');
                            }
                        }
                    }
                    KeyCode::Char('\r') | KeyCode::Char('\n') => {
                        if let Some(f) = app.playbook_edit_fields.get_mut(app.playbook_edit_field_idx) {
                            if matches!(f.kind, FieldKind::Multiline) {
                                f.value.push('\n');
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(f) = app.playbook_edit_fields.get_mut(app.playbook_edit_field_idx) {
                            if matches!(f.kind, FieldKind::Text | FieldKind::Number | FieldKind::Multiline) {
                                f.value.pop();
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(f) = app.playbook_edit_fields.get_mut(app.playbook_edit_field_idx) {
                            match &f.kind {
                                FieldKind::Bool => {
                                    f.value = if f.value == "true" { "false" } else { "true" }.to_string();
                                }
                                FieldKind::Select(opts) => {
                                    let n = opts.len();
                                    let idx = f.value.parse::<usize>().unwrap_or(0);
                                    f.value = if c == '+' || c == ' ' {
                                        ((idx + 1) % n).to_string()
                                    } else if c == '-' {
                                        (if idx == 0 { n - 1 } else { idx - 1 }).to_string()
                                    } else {
                                        idx.to_string()
                                    };
                                }
                                FieldKind::Text | FieldKind::Number | FieldKind::Multiline | FieldKind::Date => {
                                    f.value.push(c);
                                }
                                FieldKind::Button(_) => {}
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // ── Normal mode ──────────────────────────────────────────────────
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => break,

                // Tab navigation
                KeyCode::Tab => {
                    if app.selected_tab == 5 && app.perf_subtab == 1 { app.stop_perf_charts(); }
                    app.selected_tab = (app.selected_tab + 1) % 6;
                    app.show_detail  = false;
                    app.detail_scroll = 0;
                    app.perf_overview_scroll = 0;
                    app.perf_analytics_scroll = 0;
                    app.perf_section_cursor = 0;
                    app.perf_scroll_dirty = false;
                    app.cancel_mode();
                }

                // Dashboard panel focus: ←/→ switches between Risk Distribution and Open Positions
                KeyCode::Left if app.selected_tab == 0 => {
                    app.dash_panel_focus = 0;
                }
                KeyCode::Right if app.selected_tab == 0 => {
                    app.dash_panel_focus = 1;
                }
                KeyCode::BackTab => {
                    if app.selected_tab == 5 && app.perf_subtab == 1 { app.stop_perf_charts(); }
                    app.selected_tab = if app.selected_tab == 0 { 5 } else { app.selected_tab - 1 };
                    app.show_detail  = false;
                    app.detail_scroll = 0;
                    app.perf_overview_scroll = 0;
                    app.perf_analytics_scroll = 0;
                    app.perf_section_cursor = 0;
                    app.perf_scroll_dirty = false;
                    app.cancel_mode();
                }

                // Refresh data
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    app.reload(&storage);
                }

                // Dashboard KPI popup toggle
                KeyCode::Char('i') | KeyCode::Char('I') if app.selected_tab == 0 => {
                    app.dash_kpi_popup = !app.dash_kpi_popup;
                    if app.dash_kpi_popup { app.dash_kpi_scroll = 0; }
                }

                // Journal help popup toggle
                KeyCode::Char('i') | KeyCode::Char('I') if app.selected_tab == 1 => {
                    app.journal_help_popup = !app.journal_help_popup;
                    if app.journal_help_popup {
                        app.journal_help_scroll = 0;
                        app.journal_help_page = 0;
                    }
                }

                // Performance KPI popup toggle
                KeyCode::Char('i') | KeyCode::Char('I') if app.selected_tab == 5 => {
                    app.perf_kpi_popup = !app.perf_kpi_popup;
                    if app.perf_kpi_popup { app.perf_kpi_scroll = 0; }
                }

                // Performance sub-tab navigation
                KeyCode::Char('[') if app.selected_tab == 5 => {
                    if app.perf_subtab > 0 {
                        let prev = app.perf_subtab;
                        app.perf_subtab -= 1;
                        app.perf_overview_scroll = 0;
                        app.perf_analytics_scroll = 0;
                        app.perf_section_cursor = 0;
                        app.perf_scroll_dirty = false;
                        if prev == 1 { app.stop_perf_charts(); }
                        if app.perf_subtab == 1 { app.start_perf_charts(); }
                    }
                }
                KeyCode::Char(']') | KeyCode::Char('/') if app.selected_tab == 5 => {
                    let prev = app.perf_subtab;
                    app.perf_subtab = (app.perf_subtab + 1) % 3;
                    app.perf_overview_scroll = 0;
                    app.perf_analytics_scroll = 0;
                    app.perf_section_cursor = 0;
                    app.perf_scroll_dirty = false;
                    if prev == 1 { app.stop_perf_charts(); }
                    if app.perf_subtab == 1 { app.start_perf_charts(); }
                }

                // Performance section collapse/expand (keys 1-9 + 0 for section 10, context-sensitive)
                KeyCode::Char(c) if app.selected_tab == 5 && (('1'..='9').contains(&c) || c == '0') => {
                    let ki = if c == '0' { 9 } else { (c as usize) - ('1' as usize) };
                    let gi: Option<usize> = match app.perf_subtab {
                        0 => match ki { 0 => Some(0), 1 => Some(1), _ => None },
                        1 => match ki { 0 => Some(3), _ => None },
                        _ => match ki { 0 => Some(2), 1 => Some(4), 2 => Some(5), 3 => Some(6), 4 => Some(7), 5 => Some(8), 6 => Some(9), 7 => Some(10), 8 => Some(11), 9 => Some(12), _ => None },
                    };
                    if let Some(gi) = gi {
                        app.perf_collapsed[gi] = !app.perf_collapsed[gi];
                        app.perf_overview_scroll = 0;
                        app.perf_analytics_scroll = 0;
                    }
                }

                // Admin tab — open settings editor
                _ if app.selected_tab == 4 => {
                    match key.code {
                        KeyCode::Char('e') | KeyCode::Enter => {
                            app.start_admin_settings();
                        }
                        KeyCode::Char('E') => {
                            match storage.export_trades_csv("theta_vault_export.csv") {
                                Ok(()) => {
                                    app.export_status = Some("✓ Exported to theta_vault_export.csv".to_string());
                                    app.export_status_tick = 120;
                                }
                                Err(e) => {
                                    app.export_status = Some(format!("✗ Export failed: {}", e));
                                    app.export_status_tick = 120;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Journal-specific controls
                _ if app.selected_tab == 1 => {
                    // Column picker intercepts number/letter keys when open
                    if app.show_col_picker {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('v') => {
                                app.show_col_picker = false;
                                let s = col_visibility_to_string(&app.col_visibility);
                                let _ = storage.set_setting("col_visibility", &s);
                            }
                            KeyCode::Char(c) => {
                                let idx: Option<usize> = match c {
                                    '1' => Some(0), '2' => Some(1), '3' => Some(2),
                                    '4' => Some(3), '5' => Some(4), '6' => Some(5),
                                    '7' => Some(6), '8' => Some(7), '9' => Some(8),
                                    'a' => Some(9),  // BPR
                                    'b' => Some(10), // BPR%
                                    'c' => Some(11), // MaxPft
                                    'd' => Some(12), // P&L
                                    'e' => Some(13), // ROC%
                                    'f' => Some(14), // $V/d
                                    'g' => Some(15), // DTE
                                    'h' => Some(16), // Exit
                                    'i' => Some(17), // Held
                                    'j' => Some(18), // Status
                                    'k' => Some(19), // OTM%
                                    'l' => Some(20), // EM
                                    'm' => Some(21), // Mgmt
                                    _ => None,
                                };
                                if let Some(i) = idx {
                                    app.col_visibility[i] = !app.col_visibility[i];
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match key.code {
                        // Filter status cycle (lowercase f)
                        KeyCode::Char('f') => {
                            app.filter_status = app.filter_status.next();
                            app.show_detail   = false;
                            app.rebuild_visual_rows();
                            let len = app.visual_rows.len();
                            app.table_state.select(if len == 0 { None } else { Some(0) });
                        }
                        // Clear ticker filter (uppercase F)
                        KeyCode::Char('F') => {
                            app.filter_ticker.clear();
                            app.rebuild_visual_rows();
                            let len = app.visual_rows.len();
                            app.table_state.select(if len == 0 { None } else { Some(0) });
                        }
                        // Ticker search
                        KeyCode::Char('/') => {
                            app.app_mode = AppMode::FilterInput;
                            app.show_detail = false;
                        }
                        // Sort cycle
                        KeyCode::Char('s') => {
                            app.sort_key = app.sort_key.next();
                            app.rebuild_visual_rows();
                        }
                        // Flip sort direction
                        KeyCode::Char('S') => {
                            app.sort_desc = !app.sort_desc;
                            app.rebuild_visual_rows();
                        }
                        // Toggle Chain View (grouped by ticker → roll chain)
                        KeyCode::Char('G') => {
                            app.journal_chain_view = !app.journal_chain_view;
                            app.show_detail = false;
                            app.rebuild_visual_rows();
                            let len = app.visual_rows.len();
                            app.table_state.select(if len == 0 { None } else { Some(0) });
                        }
                        // Toggle detail
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            if let Some(trade) = app.selected_trade() {
                                if let Ok(chain) = storage.get_roll_chain(trade.id) {
                                    app.selected_trade_chain = chain;
                                } else {
                                    app.selected_trade_chain = vec![trade.clone()];
                                }
                                app.show_detail   = !app.show_detail;
                                app.detail_scroll = 0;
                            }
                        }
                        // Edit trade
                        KeyCode::Char('e') | KeyCode::Char('E') => {
                            if let Some(t) = app.selected_trade_cloned() {
                                app.start_edit(&t);
                            }
                        }
                        // Close trade
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            if let Some(t) = app.selected_trade_cloned() {
                                if t.is_open() {
                                    app.start_close(&t);
                                }
                            }
                        }
                        // Analyze trade (payoff chart)
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            if let Some(t) = app.selected_trade_cloned() {
                                app.start_analyze(&t);
                            }
                        }
                        // Column visibility picker
                        KeyCode::Char('v') => {
                            app.show_col_picker = !app.show_col_picker;
                        }
                        // Delete trade
                        KeyCode::Char('x') | KeyCode::Delete => {
                            if let Some(t) = app.selected_trade() {
                                app.delete_trade_id = Some(t.id);
                                app.app_mode = AppMode::ConfirmDelete;
                            }
                        }
                        // Collapse/expand year/month group or toggle detail
                        KeyCode::Enter => {
                            if let Some(idx) = app.table_state.selected() {
                                match app.visual_rows.get(idx).cloned() {
                                    Some(VisualRowKind::YearHeader { year }) => {
                                        if app.collapsed_years.contains(&year) {
                                            app.collapsed_years.remove(&year);
                                        } else {
                                            app.collapsed_years.insert(year);
                                        }
                                        app.rebuild_visual_rows();
                                        let len = app.visual_rows.len();
                                        if idx >= len {
                                            app.table_state.select(Some(len.saturating_sub(1)));
                                        }
                                    }
                                    Some(VisualRowKind::MonthHeader { year, month }) => {
                                        let key = (year, month);
                                        if app.collapsed_months.contains(&key) {
                                            app.collapsed_months.remove(&key);
                                        } else {
                                            app.collapsed_months.insert(key);
                                        }
                                        app.rebuild_visual_rows();
                                        let len = app.visual_rows.len();
                                        if idx >= len {
                                            app.table_state.select(Some(len.saturating_sub(1)));
                                        }
                                    }
                                    Some(VisualRowKind::Trade(_)) => {
                                        if let Some(trade) = app.selected_trade() {
                                            if let Ok(chain) = storage.get_roll_chain(trade.id) {
                                                app.selected_trade_chain = chain;
                                            } else {
                                                app.selected_trade_chain = vec![trade.clone()];
                                            }
                                        }
                                        app.show_detail   = !app.show_detail;
                                        app.detail_scroll = 0;
                                    }
                                    Some(VisualRowKind::TickerHeader { .. })
                                    | Some(VisualRowKind::ChainHeader { .. }) => {}
                                    None => {}
                                }
                            }
                        }
                        // Navigation
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.nav_down();
                            if app.show_detail {
                                if let Some(trade) = app.selected_trade() {
                                    if let Ok(chain) = storage.get_roll_chain(trade.id) {
                                        app.selected_trade_chain = chain;
                                    } else {
                                        app.selected_trade_chain = vec![trade.clone()];
                                    }
                                }
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.nav_up();
                            if app.show_detail {
                                if let Some(trade) = app.selected_trade() {
                                    if let Ok(chain) = storage.get_roll_chain(trade.id) {
                                        app.selected_trade_chain = chain;
                                    } else {
                                        app.selected_trade_chain = vec![trade.clone()];
                                    }
                                }
                            }
                        }
                        KeyCode::PageDown => for _ in 0..10 { app.nav_down(); },
                        KeyCode::PageUp   => for _ in 0..10 { app.nav_up(); },
                        KeyCode::Home => {
                            if !app.visual_rows.is_empty() {
                                app.table_state.select(Some(0));
                                app.detail_scroll = 0;
                            }
                        }
                        KeyCode::End => {
                            let len = app.visual_rows.len();
                            if len > 0 {
                                app.table_state.select(Some(len - 1));
                                app.detail_scroll = 0;
                            }
                        }
                        // Navigate to next/prev trade while staying in detail mode
                        KeyCode::Right if app.show_detail => {
                            app.nav_detail_next_trade();
                            if let Some(trade) = app.selected_trade() {
                                if let Ok(chain) = storage.get_roll_chain(trade.id) {
                                    app.selected_trade_chain = chain;
                                } else {
                                    app.selected_trade_chain = vec![trade.clone()];
                                }
                            }
                        }
                        KeyCode::Left if app.show_detail => {
                            app.nav_detail_prev_trade();
                            if let Some(trade) = app.selected_trade() {
                                if let Ok(chain) = storage.get_roll_chain(trade.id) {
                                    app.selected_trade_chain = chain;
                                } else {
                                    app.selected_trade_chain = vec![trade.clone()];
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Perf tab controls (Enter, PageUp, PageDown)
                _ if app.selected_tab == 5 => {
                    match key.code {
                        KeyCode::Enter => {
                            if let Some(gi) = perf_cursor_to_gi(app.perf_subtab, app.perf_section_cursor) {
                                app.perf_collapsed[gi] = !app.perf_collapsed[gi];
                                // do NOT reset scroll — keep view on toggled header
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.nav_down(),
                        KeyCode::Up   | KeyCode::Char('k') => app.nav_up(),
                        KeyCode::PageDown => {
                            // Scroll content without moving cursor (Charts tab has no text scroll)
                            if app.perf_subtab == 0 {
                                app.perf_overview_scroll = app.perf_overview_scroll.saturating_add(5).min(app.perf_overview_max_scroll);
                            } else if app.perf_subtab == 2 {
                                app.perf_analytics_scroll = app.perf_analytics_scroll.saturating_add(5).min(app.perf_analytics_max_scroll);
                            }
                        }
                        KeyCode::PageUp => {
                            if app.perf_subtab == 0 {
                                app.perf_overview_scroll = app.perf_overview_scroll.saturating_sub(5);
                            } else if app.perf_subtab == 2 {
                                app.perf_analytics_scroll = app.perf_analytics_scroll.saturating_sub(5);
                            }
                        }
                        _ => {}
                    }
                }

                // Actions tab controls
                _ if app.selected_tab == 3 => {
                    match key.code {
                        KeyCode::Enter => {
                            if let Some(idx) = app.actions_list_state.selected() {
                                let rows = theta_vault_rust::actions::build_action_rows(
                                    &app.alerts, &app.collapsed_action_kinds,
                                );
                                match rows.get(idx) {
                                    Some(theta_vault_rust::actions::ActionRow::GroupHeader { kind, .. }) => {
                                        let kind = kind.clone();
                                        if app.collapsed_action_kinds.contains(&kind) {
                                            app.collapsed_action_kinds.remove(&kind);
                                        } else {
                                            app.collapsed_action_kinds.insert(kind);
                                        }
                                    }
                                    Some(theta_vault_rust::actions::ActionRow::Alert(alert)) if alert.trade_id > 0 => {
                                        let trade_id = alert.trade_id;
                                        app.selected_tab    = 1;
                                        app.filter_status   = FilterStatus::Open;
                                        app.filter_ticker   = String::new();
                                        app.show_detail     = false;
                                        app.detail_scroll   = 0;
                                        app.rebuild_visual_rows();
                                        let row_idx = app.visual_rows.iter().position(|r| {
                                            matches!(r, VisualRowKind::Trade(ti) if app.trades[*ti].id == trade_id)
                                        });
                                        app.table_state.select(row_idx.or(Some(0)));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // L14: N = add quick journal note to the selected alert's trade
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            if let Some(idx) = app.actions_list_state.selected() {
                                let rows = theta_vault_rust::actions::build_action_rows(
                                    &app.alerts, &app.collapsed_action_kinds,
                                );
                                if let Some(theta_vault_rust::actions::ActionRow::Alert(alert)) = rows.get(idx) {
                                    if alert.trade_id > 0 {
                                        app.journal_note_trade_id = Some(alert.trade_id);
                                        app.journal_note_buf.clear();
                                        app.app_mode = AppMode::JournalNote;
                                    }
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.nav_down(),
                        KeyCode::Up   | KeyCode::Char('k') => app.nav_up(),
                        KeyCode::PageDown => for _ in 0..10 { app.nav_down(); },
                        KeyCode::PageUp   => for _ in 0..10 { app.nav_up(); },
                        _ => {}
                    }
                }

                // Playbook-specific controls
                _ if app.selected_tab == 2 => {
                    match key.code {
                        KeyCode::Char('n') | KeyCode::Char('N') => app.start_new_playbook(),
                        KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
                            if !app.playbooks.is_empty() { app.start_edit_playbook(); }
                        }
                        KeyCode::Char('t') | KeyCode::Char('T') => {
                            if !app.playbooks.is_empty() { app.start_thesis_edit(); }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.nav_down(),
                        KeyCode::Up   | KeyCode::Char('k') => app.nav_up(),
                        KeyCode::Right | KeyCode::Char('l') => app.scroll_right(),
                        KeyCode::Left  | KeyCode::Char('h') => app.scroll_left(),
                        KeyCode::PageDown => for _ in 0..10 { app.nav_down(); },
                        KeyCode::PageUp   => for _ in 0..10 { app.nav_up(); },
                        KeyCode::Home if !app.playbooks.is_empty() => {
                            app.playbook_state.select(Some(0));
                            app.thesis_scroll = 0;
                        }
                        KeyCode::End if !app.playbooks.is_empty() => {
                            app.playbook_state.select(Some(app.playbooks.len() - 1));
                            app.thesis_scroll = 0;
                        }
                        _ => {}
                    }
                }

                // Shared navigation (non-playbook tabs)
                KeyCode::Down | KeyCode::Char('j') => app.nav_down(),
                KeyCode::Up   | KeyCode::Char('k') => app.nav_up(),
                KeyCode::Right | KeyCode::Char('l') => app.scroll_right(),
                KeyCode::Left  | KeyCode::Char('h') => app.scroll_left(),
                KeyCode::PageDown => for _ in 0..10 { app.nav_down(); },
                KeyCode::PageUp   => for _ in 0..10 { app.nav_up(); },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    Ok(())
}
