// Copyright (c) 2025 Chris Wenk. All rights reserved.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear,
        List, ListItem, ListState, Paragraph, Row, Table, TableState, Tabs, Wrap,
        Chart, Axis, Dataset, GraphType,
    },
    symbols,
    Frame,
};
use crate::models::{LegType, PlaybookStrategy, PerformanceStats, PortfolioStats, StrategyType, Trade};
use crate::calculations::{
    calculate_breakevens, calculate_calendar_payoff_at_price, calculate_held_duration,
    calculate_max_loss_from_legs, calculate_max_profit, calculate_payoff_at_price,
    calculate_pct_max_profit, calculate_pnl_per_day, calculate_roc, calculate_remaining_dte,
    compute_spread_width_from_legs, estimate_pop, format_trade_description, vix_max_heat,
};
use chrono::Utc;
use crate::app::{AppMode, EditField, FieldKind, FilterStatus, SortKey, VisualRowKind};
use std::collections::HashSet;

// ── Color palette (OTJ mobile dark theme) ───────────────────────────────────
const C_GREEN:  Color = Color::Rgb(34,  197, 94);   // green-500
const C_RED:    Color = Color::Rgb(239, 68,  68);   // red-500
const C_YELLOW: Color = Color::Rgb(234, 179, 8);    // yellow-500
const C_CYAN:   Color = Color::Rgb(6,   182, 212);  // cyan-500
const C_BLUE:   Color = Color::Rgb(59,  130, 246);  // blue-500
const C_GRAY:   Color = Color::Rgb(100, 116, 139);  // slate-500
const C_DARK:   Color = Color::Rgb(30,  41,  59);   // slate-800
const C_WHITE:  Color = Color::White;

// ── Main entry point ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_ui(
    f:                &mut Frame,
    display_count:    usize,                        // # of Trade rows (not headers)
    visual_rows:      &[VisualRowKind],             // visual row list (headers + trades)
    collapsed_months: &HashSet<(i32, u32)>,
    all_trades:       &[Trade],
    stats:            &PortfolioStats,
    perf_stats:       &PerformanceStats,
    perf_subtab:      usize,
    perf_overview_scroll: u16,
    perf_analytics_scroll: u16,
    playbooks:        &[PlaybookStrategy],
    selected_tab:     usize,
    table_state:      &mut TableState,
    playbook_state:   &mut ListState,
    thesis_scroll:    u16,
    show_detail:      bool,
    detail_scroll:    u16,
    trade_chain:      &[Trade],
    dash_open_scroll: usize,
    filter_status:    FilterStatus,
    filter_ticker:    &str,
    sort_key:         SortKey,
    sort_desc:        bool,
    app_mode:         AppMode,
    edit_fields:      &[EditField],
    edit_field_idx:   usize,
    edit_scroll:      u16,
    close_fields:            &[EditField],
    close_field_idx:         usize,
    delete_trade_id:         Option<i32>,
    playbook_edit_fields:    &[EditField],
    playbook_edit_field_idx: usize,
    playbook_edit_scroll:    u16,
    alerts:                  &[crate::actions::TradeAlert],
    actions_list_state:      &mut ListState,
    collapsed_action_kinds:  &HashSet<crate::actions::AlertKind>,
    pulse_on:                bool,
    under_tauri:             bool,
    dash_kpi_popup:          bool,
    dash_kpi_scroll:         u16,
    dash_kpi_max_scroll:     &mut u16,
    perf_kpi_popup:          bool,
    perf_kpi_scroll:         u16,
    perf_kpi_max_scroll:     &mut u16,
    journal_help_popup:      bool,
    journal_help_scroll:     u16,
    journal_help_max_scroll: &mut u16,
    journal_help_page:       u8,
    max_heat_pct:            f64,   // effective (VIX-adaptive, capped by ceiling)
    stored_heat_ceiling:     f64,   // raw stored setting, for Admin display only
    max_pos_bpr_pct:         f64,
    admin_fields:            &[EditField],
    admin_field_idx:         usize,
    admin_scroll:            u16,
    cal_year:                i32,
    cal_month:               u32,
    cal_day:                 u32,
    thesis_edit_buf:         &str,
    spy_monthly:             &std::collections::HashMap<(i32, u32), f64>,
    live_prices:             &std::collections::HashMap<String, f64>,
    perf_collapsed:          &[bool; 14],
    perf_section_cursor:     usize,
    col_visibility:          &[bool; 22],
    show_col_picker:         bool,
    journal_chain_view:      bool,
    default_profit_target_pct: f64,
    default_mgmt_dte:          i32,
    export_status:             Option<&str>,
    dash_risk_scroll:          usize,
    dash_panel_focus:          u8,
    playbook_analytics:        &[crate::models::PlaybookAnalytics],
    journal_note_buf:          &str,
    journal_note_trade_id:     Option<i32>,
) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // footer
        ])
        .split(area);

    // ── Tab bar
    let tabs = Tabs::new(vec![
        " ◆ Dashboard   ",
        " ≡ Journal     ",
        " ⊞ Playbook    ",
        " ⚡ Actions     ",
        " ✦ Admin       ",
        " ★ Perf        ",
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(
                " ⟨ ThetaVault - Trade small, Trade often ⟩ ",
                Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD),
            )),
    )
    .select(selected_tab)
    .style(Style::default().fg(C_GRAY))
    .highlight_style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    // ── Content
    match selected_tab {
        0 => draw_dashboard(f, chunks[1], stats, perf_stats, all_trades, dash_open_scroll, max_heat_pct, alerts, dash_risk_scroll, dash_panel_focus),
        1 => draw_journal(
            f, chunks[1],
            display_count, visual_rows, collapsed_months, all_trades,
            table_state,
            show_detail, detail_scroll, trade_chain,
            filter_status, filter_ticker,
            sort_key, sort_desc,
            app_mode,
            edit_fields, edit_field_idx, edit_scroll,
            close_fields, close_field_idx,
            live_prices,
            under_tauri,
            playbooks,
            col_visibility,
            show_col_picker,
            journal_chain_view,
            stats.account_size,
            max_pos_bpr_pct,
            default_profit_target_pct,
        ),
        2 => draw_playbook(
            f, chunks[1], playbooks, playbook_state, thesis_scroll,
            app_mode, playbook_edit_fields, playbook_edit_field_idx, playbook_edit_scroll,
            thesis_edit_buf, perf_stats, playbook_analytics,
        ),
        3 => draw_daily_actions(f, chunks[1], alerts, actions_list_state, collapsed_action_kinds, pulse_on, stats),
        4 => draw_admin(f, chunks[1], app_mode, admin_fields, admin_field_idx, admin_scroll, stats, max_heat_pct, stored_heat_ceiling, max_pos_bpr_pct, default_mgmt_dte, export_status),
        5 => draw_performance(f, chunks[1], stats, perf_stats, perf_subtab, perf_overview_scroll, perf_analytics_scroll, spy_monthly, perf_collapsed as &[bool; 14], perf_section_cursor, under_tauri),
        _ => {}
    }

    // ── Footer — changes per mode
    let footer_text = match (selected_tab, app_mode) {
        (1, AppMode::FilterInput)   => " Esc:Done  Backspace:Del  (type ticker to filter) ",
        (1, AppMode::EditTrade)     => " ↑↓/Tab:Field  +/-:Cycle  Ctrl+S:Save  Esc:Cancel  Ctrl+A:AddLeg  Ctrl+D:DelLeg  Enter:Button ",
        (1, AppMode::CloseTrade)    => " ↑↓/Tab:Field  Ctrl+S:Save  Esc:Cancel ",
        (1, AppMode::ConfirmDelete) => " Y/Enter:Confirm Delete  Any other key:Cancel ",
        (1, AppMode::AnalyzeTrade)  => if under_tauri { " Esc:Close  (graphical overlay)" } else { " Esc:Close  (Payoff at Expiration — ASCII chart)" },
        (_, AppMode::DatePicker)    => " ←→:Day  ↑↓:Week  [/]:Month  Enter:Confirm  Esc:Cancel ",
        (1, _) if journal_chain_view => " Q:Quit  ↑↓:Nav  Enter:Detail  f:Filter  /:Search  G:Chain[ON]  e:Edit  c:Close  a:Analyze  x:Del  v:Cols  i:Help  R:Refresh ",
        (1, _) => " Q:Quit  ↑↓:Nav  Enter:Detail  f:Filter  /:Search  s:Sort  G:Chain  e:Edit  c:Close  a:Analyze  x:Del  v:Cols  i:Help  R:Refresh ",
        (0, _) => " Q:Quit  Tab:Switch  ←→:Focus  ↑↓:Scroll  i:KPI Info  R:Refresh ",
        (2, AppMode::EditThesis)   => " Type to edit  Enter:Newline  Backspace:Del  Ctrl+S:Save  Esc:Cancel ",
        (2, AppMode::EditPlaybook) => " ↑↓/Tab:Field  +/-:Cycle  Ctrl+S:Save  Esc:Cancel ",
        (2, _) if under_tauri      => " Q:Quit  Tab:Switch  ↑↓:Select  N:New  E:Edit  T:Thesis  ?:Guide (CC/SPV/IC) ",
        (2, _)                     => " Q:Quit  Tab:Switch  ↑↓:Select  ↕:Scroll  N:New  E:Edit  T:Edit Thesis ",
        (3, AppMode::JournalNote)  => " Type note  Enter:Save  Esc:Cancel ",
        (3, _)                     => " Q:Quit  ↑↓:Nav  Enter:Collapse/→Journal  N:Add Note  R:Refresh ",
        (4, AppMode::AdminSettings) => " ↑↓/Tab:Field  Ctrl+S:Save  Esc:Cancel ",
        (4, _)                     => " Q:Quit  E:Edit Settings  R:Refresh ",
        (5, _)                     => " Q:Quit  ↑↓:Nav  Enter:Toggle  PgUp/Dn:Scroll  /:SubTab  1-N:Collapse  i:KPI  Tab:Switch  R:Refresh ",
        _                          => " Q:Quit  Tab:Switch  R:Refresh ",
    };
    f.render_widget(
        Paragraph::new(footer_text).style(Style::default().bg(C_DARK).fg(C_GRAY)),
        chunks[2],
    );

    // ── ConfirmDelete overlay — rendered last so it sits on top
    if app_mode == AppMode::ConfirmDelete && selected_tab == 1 {
        draw_confirm_delete(f, chunks[1], delete_trade_id, all_trades);
    }

    // ── KPI info popup (Dashboard tab, i key)
    if dash_kpi_popup && selected_tab == 0 {
        draw_kpi_popup(f, chunks[1], stats, perf_stats, max_heat_pct, dash_kpi_scroll, dash_kpi_max_scroll, default_mgmt_dte);
    }

    // ── Performance KPI popup (Perf tab, i key)
    if perf_kpi_popup && selected_tab == 5 {
        draw_perf_kpi_popup(f, chunks[1], perf_kpi_scroll, perf_kpi_max_scroll);
    }

    // ── Date picker overlay — always on top
    if app_mode == AppMode::DatePicker {
        draw_date_picker(f, chunks[1], cal_year, cal_month, cal_day);
    }

    // ── Column picker popup (Journal tab, 'v' key)
    if show_col_picker && selected_tab == 1 {
        draw_col_picker_popup(f, chunks[1], col_visibility);
    }

    // ── Journal help popup (Journal tab, 'i' key)
    if journal_help_popup && selected_tab == 1 {
        draw_journal_help_popup(f, chunks[1], journal_help_scroll, journal_help_max_scroll, journal_help_page);
    }

    // ── L14: Journal Note quick-entry popup (Actions tab, N key)
    if app_mode == AppMode::JournalNote {
        draw_journal_note_popup(f, chunks[1], journal_note_buf, journal_note_trade_id, all_trades);
    }
}

// ── Dashboard ────────────────────────────────────────────────────────────────

/// H6: Composite A-F health score from win rate / drawdown / heat / BWD / EV.
/// Returns (grade_char, grade_color, detail_string).
fn health_grade(stats: &PortfolioStats) -> (char, Color, String) {
    let score = |s: u32| -> char { match s { 4 => 'A', 3 => 'B', 2 => 'C', 1 => 'D', _ => 'F' } };

    // Win rate (0.0–1.0)
    let wr_s = if stats.closed_trades == 0 { 2u32 } else {
        let wr = stats.win_rate;
        if wr >= 0.65 { 4 } else if wr >= 0.55 { 3 } else if wr >= 0.50 { 2 } else if wr >= 0.40 { 1 } else { 0 }
    };
    // Max drawdown %
    let dd_s = {
        let dd = stats.max_drawdown_pct.abs();
        if dd <= 5.0 { 4 } else if dd <= 10.0 { 3 } else if dd <= 15.0 { 2 } else if dd <= 25.0 { 1 } else { 0 }
    };
    // Heat (alloc_pct = total_open_bpr / account × 100)
    let heat_s = {
        let h = stats.alloc_pct;
        if h <= 25.0 { 4 } else if h <= 40.0 { 3 } else if h <= 50.0 { 2 } else if h <= 65.0 { 1 } else { 0 }
    };
    // BWD (net beta-weighted delta, closer to 0 is better)
    let bwd_s = {
        let b = stats.net_beta_weighted_delta.abs();
        if b <= 5.0 { 4 } else if b <= 15.0 { 3 } else if b <= 30.0 { 2 } else if b <= 50.0 { 1 } else { 0 }
    };
    // EV (avg P&L per trade)
    let ev_s = if stats.closed_trades == 0 { 2u32 } else {
        let ev = stats.avg_pnl_per_trade;
        if ev >= 50.0 { 4 } else if ev >= 20.0 { 3 } else if ev >= 0.0 { 2 } else if ev >= -20.0 { 1 } else { 0 }
    };

    let avg = (wr_s + dd_s + heat_s + bwd_s + ev_s) as f64 / 5.0;
    let (grade, color) = if avg >= 3.5 { ('A', C_GREEN) }
        else if avg >= 2.5 { ('B', Color::Rgb(74, 222, 128)) }  // light green
        else if avg >= 1.5 { ('C', C_YELLOW) }
        else if avg >= 0.5 { ('D', C_RED) }
        else { ('F', C_RED) };

    let detail = format!("WR:{} DD:{} Heat:{} BWD:{} EV:{}",
        score(wr_s), score(dd_s), score(heat_s), score(bwd_s), score(ev_s));
    (grade, color, detail)
}

fn draw_dashboard(f: &mut Frame, area: Rect, stats: &PortfolioStats, perf_stats: &PerformanceStats, trades: &[Trade], open_scroll: usize, max_heat_pct: f64, alerts: &[crate::actions::TradeAlert], risk_scroll: usize, panel_focus: u8) {
    // L9: 2-row KPI fallback when terminal is narrow (<130 cols)
    let wide = area.width >= 130;

    let (kpi_areas, footer1_area, footer2_area, bottom_area): (Vec<Rect>, Rect, Rect, Rect) = if wide {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
            .split(area);
        let kpi = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 9), Constraint::Ratio(1, 9), Constraint::Ratio(1, 9),
                Constraint::Ratio(1, 9), Constraint::Ratio(1, 9), Constraint::Ratio(1, 9),
                Constraint::Ratio(1, 9), Constraint::Ratio(1, 9), Constraint::Ratio(1, 9),
            ])
            .split(rows[0]);
        (kpi.iter().copied().collect(), rows[1], rows[2], rows[3])
    } else {
        // Narrow: 5 cards in row A (Balance, P&L, Unreal, BWD, Vega)
        //         4 cards in row B (Heat, Risk, VIX, POP)
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9), Constraint::Length(9),
                Constraint::Length(1), Constraint::Length(1), Constraint::Min(0),
            ])
            .split(area);
        let kpi_a = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 5), Constraint::Ratio(1, 5), Constraint::Ratio(1, 5),
                Constraint::Ratio(1, 5), Constraint::Ratio(1, 5),
            ])
            .split(rows[0]);
        let kpi_b = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4), Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4), Constraint::Ratio(1, 4),
            ])
            .split(rows[1]);
        let mut areas: Vec<Rect> = kpi_a.iter().copied().collect();
        areas.extend(kpi_b.iter().copied());
        (areas, rows[2], rows[3], rows[4])
    };

    let kpi = &kpi_areas;

    // Card 1 — Balance
    let bal_color = if stats.balance >= stats.account_size { C_GREEN } else { C_RED };
    let bp_color = if stats.bp_available >= 0.0 { C_GRAY } else { C_RED };
    let (hgrade, hcolor, hdetail) = health_grade(stats);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" ${:.0}", stats.balance),
                Style::default().fg(bal_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(format!(" acct ${:.0}", stats.account_size), Style::default().fg(C_GRAY)),
            ]),
            Line::from(vec![
                Span::styled(" BP Avail:", Style::default().fg(C_GRAY)),
            ]),
            Line::from(vec![
                Span::styled(format!(" ${:.0}", stats.bp_available), Style::default().fg(bp_color)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Health: ", Style::default().fg(C_GRAY)),
                Span::styled(hgrade.to_string(), Style::default().fg(hcolor).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!(" {}", hdetail), Style::default().fg(Color::Rgb(100, 116, 139))),
            ]),
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Balance ", Style::default().fg(C_CYAN)))),
        kpi[0],
    );

    // Card 2 — P&L (realized)
    let pnl_color = if stats.realized_pnl >= 0.0 { C_GREEN } else { C_RED };
    let pace_line = if stats.monthly_pnl_target > 0.0 {
        let pct = stats.monthly_pnl_pace / stats.monthly_pnl_target * 100.0;
        let pace_color = if pct >= 100.0 { C_GREEN } else if pct >= 50.0 { C_YELLOW } else { C_RED };
        Line::from(vec![
            Span::styled(" Pace: ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!("${:.0}/mo ({:+.0}%)", stats.monthly_pnl_pace, pct - 100.0),
                Style::default().fg(pace_color),
            ),
        ])
    } else if stats.monthly_pnl_pace != 0.0 {
        Line::from(vec![Span::styled(
            format!(" {:.1}% WR", stats.win_rate * 100.0),
            Style::default().fg(C_GRAY),
        )])
    } else {
        Line::from(vec![Span::styled(
            format!(" {:.1}% WR", stats.win_rate * 100.0),
            Style::default().fg(C_GRAY),
        )])
    };
    let streak_color_d = if stats.current_streak >= 0 { C_GREEN } else { C_RED };
    let streak_str_d = if stats.current_streak >= 0 {
        format!(" +{}W", stats.current_streak)
    } else {
        format!(" {}L", stats.current_streak.abs())
    };
    let progress_line = if stats.monthly_pnl_target > 0.0 {
        let pct = (stats.monthly_pnl_pace / stats.monthly_pnl_target).clamp(0.0, 1.0);
        let filled = (pct * 8.0).round() as usize;
        let empty = 8usize.saturating_sub(filled);
        let bar: String = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(empty);
        let bar_pct = stats.monthly_pnl_pace / stats.monthly_pnl_target * 100.0;
        let bar_color = if bar_pct >= 100.0 { C_GREEN } else if bar_pct >= 50.0 { C_YELLOW } else { C_RED };
        Line::from(vec![
            Span::styled(format!(" {}", bar), Style::default().fg(bar_color)),
            Span::styled(format!(" {:.0}%", bar_pct), Style::default().fg(bar_color)),
        ])
    } else {
        Line::from(vec![Span::styled(
            streak_str_d.clone(),
            Style::default().fg(streak_color_d),
        )])
    };
    // H3: Month-end P&L projection — shown when ≥5 days elapsed in current month
    let month_end_proj_line = {
        use chrono::Datelike;
        let now = Utc::now().date_naive();
        let cur_year  = now.year();
        let cur_month = now.month();
        let days_elapsed = now.day() as f64;
        let days_in_month = {
            let next = if cur_month == 12 {
                chrono::NaiveDate::from_ymd_opt(cur_year + 1, 1, 1)
            } else {
                chrono::NaiveDate::from_ymd_opt(cur_year, cur_month + 1, 1)
            };
            next.map(|d| (d - chrono::NaiveDate::from_ymd_opt(cur_year, cur_month, 1).unwrap()).num_days() as f64)
                .unwrap_or(30.0)
        };
        if days_elapsed >= 5.0 {
            if let Some(mp) = perf_stats.monthly_pnl.iter().find(|m| m.year == cur_year && m.month == cur_month) {
                let projection = (mp.pnl / days_elapsed) * days_in_month;
                let proj_color = if projection >= 0.0 { C_GREEN } else { C_RED };
                Line::from(vec![
                    Span::styled(" Est. M-end: ", Style::default().fg(Color::Rgb(148, 163, 184))),
                    Span::styled(format!("${:+.0}", projection), Style::default().fg(proj_color)),
                ])
            } else {
                Line::from("")
            }
        } else {
            Line::from("")
        }
    };
    // Item 14: R:R + EV lines for P&L card (separate lines so EV isn't truncated)
    let (rr_line, ev_line) = if perf_stats.avg_loss > 0.0 {
        let rr = perf_stats.avg_win / perf_stats.avg_loss;
        let rr_color = if rr >= 1.0 { C_GREEN } else if rr >= 0.5 { C_YELLOW } else { C_RED };
        let ev_color = if perf_stats.expected_value >= 0.0 { C_GREEN } else { C_RED };
        (
            Line::from(vec![
                Span::styled(" R:R ", Style::default().fg(C_GRAY)),
                Span::styled(format!("1:{:.2}", rr), Style::default().fg(rr_color)),
            ]),
            Line::from(vec![
                Span::styled(" EV ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:+.0}/tr", perf_stats.expected_value), Style::default().fg(ev_color)),
            ]),
        )
    } else {
        (Line::from(""), Line::from(""))
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(" ${:+.0}", stats.realized_pnl),
                    Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(streak_str_d, Style::default().fg(streak_color_d)),
            ]),
            pace_line,
            progress_line,
            month_end_proj_line,
            rr_line,
            ev_line,
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" P&L ", Style::default().fg(C_CYAN)))),
        kpi[1],
    );

    // Card 3 — Unreal (estimated unrealized P&L from theta decay)
    let unreal_color = if stats.unrealized_pnl >= 0.0 { C_GREEN } else { C_RED };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" ${:+.0}", stats.unrealized_pnl),
                Style::default().fg(unreal_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(" est. θ decay", Style::default().fg(C_GRAY))]),
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Unreal ", Style::default().fg(C_CYAN)))),
        kpi[2],
    );

    // Card 4 — BWD (Beta-Weighted Delta)
    let (bwd_str, bwd_color) = if stats.spy_price.is_none() {
        ("N/A".to_string(), C_GRAY)
    } else {
        let v = stats.net_beta_weighted_delta;
        let abs_v = v.abs();
        let c = if abs_v <= 5.0 { C_GREEN } else if abs_v <= 15.0 { C_YELLOW } else { C_RED };
        (format!("{:+.1}", v), c)
    };
    let stress_line = if let Some(spy) = stats.spy_price {
        let stress_est = stats.net_beta_weighted_delta * spy * (-0.05);
        let stress_color = if stress_est > 0.0 { C_GREEN } else { C_RED };
        Line::from(vec![
            Span::styled(" -5%≈ ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:+.0}", stress_est), Style::default().fg(stress_color)),
        ])
    } else {
        Line::from(vec![Span::styled(" β-wtd delta", Style::default().fg(C_GRAY))])
    };
    let theta_delta_line = if let Some(ratio) = stats.theta_delta_ratio {
        let td_color = if ratio >= 1.0 { C_GREEN } else if ratio >= 0.5 { C_YELLOW } else { C_RED };
        Line::from(vec![
            Span::styled(" Θ/Δ: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.2}", ratio), Style::default().fg(td_color)),
        ])
    } else {
        stress_line.clone()
    };
    // Gamma risk: open positions with DTE ≤ 7
    let gamma_risk_tickers: Vec<&str> = stats.next_critical_positions.iter()
        .filter(|(_, dte)| *dte <= 7 && *dte >= 0)
        .map(|(t, _)| t.as_str())
        .collect();
    let gamma_line = if !gamma_risk_tickers.is_empty() {
        Line::from(vec![
            Span::styled(" \u{26a0} ", Style::default().fg(C_RED).add_modifier(Modifier::BOLD)),
            Span::styled(gamma_risk_tickers.join(","), Style::default().fg(C_RED)),
        ])
    } else {
        theta_delta_line
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {}Δ", bwd_str),
                Style::default().fg(bwd_color).add_modifier(Modifier::BOLD),
            )]),
            gamma_line,
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" BWD ", Style::default().fg(C_CYAN)))),
        kpi[3],
    );

    // Card 5 — Vega (Net portfolio vega — standalone)
    {
        let vega_val = stats.net_vega;
        // Premium sellers are naturally short vega; mildly negative is normal/yellow
        let vega_color = if vega_val > 0.0 {
            C_RED   // long vega — unusual for premium seller
        } else if vega_val < -5000.0 {
            Color::Rgb(239, 68, 68)  // dangerously short
        } else {
            C_YELLOW // normal negative vega for premium seller
        };
        // Approximate $ impact per 1% IV move: vega is per-contract, already totalled
        let per_pct = vega_val; // net_vega is already the full portfolio sensitivity
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!(" {:+.0}V", vega_val),
                    Style::default().fg(vega_color).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled(" /1% IV: ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("${:+.0}", per_pct), Style::default().fg(vega_color)),
                ]),
            ])
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
                .title(Span::styled(" Vega ", Style::default().fg(C_CYAN)))),
            kpi[4],
        );
    }

    // Card 5 — Heat (BPR/Account × 100, colored vs max_heat_pct)
    let heat_color = if stats.alloc_pct >= max_heat_pct { C_RED }
        else if stats.alloc_pct >= max_heat_pct * 0.75 { C_YELLOW }
        else { C_GREEN };
    let per_trade_str = format!(" ${:.0}k–${:.0}k/tr", stats.account_size * 0.01 / 1000.0, stats.account_size * 0.03 / 1000.0);
    // M4: room for more trades = remaining BPR capacity ÷ avg BPR per open trade
    let room_line = if stats.open_trades > 0 && stats.total_open_bpr > 0.0 {
        let max_bpr = max_heat_pct / 100.0 * stats.account_size;
        let remaining = max_bpr - stats.total_open_bpr;
        let avg_bpr = stats.total_open_bpr / stats.open_trades as f64;
        let room = (remaining / avg_bpr).floor() as i64;
        let (room_str, room_color) = if room <= 0 {
            (format!(" Full ({:+} trades)", room), C_RED)
        } else if room <= 2 {
            (format!(" Room: ~{} more", room), C_YELLOW)
        } else {
            (format!(" Room: ~{} more", room), C_GREEN)
        };
        Line::from(vec![Span::styled(room_str, Style::default().fg(room_color))])
    } else {
        Line::from("")
    };
    let kelly_heat_line = if let Some(kelly) = perf_stats.kelly_fraction {
        if kelly > 0.0 {
            let half_k = kelly / 2.0;
            let kc = if kelly <= max_heat_pct { C_GREEN } else if kelly <= max_heat_pct * 2.0 { C_YELLOW } else { C_RED };
            Line::from(vec![Span::styled(
                format!(" K:{:.1}% \u{00bd}K:{:.1}%", kelly, half_k),
                Style::default().fg(kc),
            )])
        } else {
            Line::from(vec![Span::styled(" K: neg edge", Style::default().fg(C_RED))])
        }
    } else {
        Line::from("")
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {:.1}%", stats.alloc_pct),
                Style::default().fg(heat_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(" ${:.0}k / max{:.0}%", stats.total_open_bpr / 1000.0, max_heat_pct),
                Style::default().fg(C_GRAY),
            )]),
            Line::from(vec![Span::styled(per_trade_str, Style::default().fg(C_GRAY))]),
            room_line,
            kelly_heat_line,
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Heat ", Style::default().fg(C_CYAN)))),
        kpi[5],
    );

    // Card 7 — Risk (undefined%/defined% split as "78/22")
    let (risk_str, risk_color) = if stats.total_open_bpr > 0.0 {
        let u = stats.undefined_risk_pct.round() as u32;
        let d = stats.defined_risk_pct.round() as u32;
        let diff = (stats.undefined_risk_pct - stats.target_undefined_pct).abs();
        let c = if diff <= 10.0 { C_GREEN } else if diff <= 20.0 { C_YELLOW } else { C_RED };
        (format!("{}/{}", u, d), c)
    } else {
        ("—/—".to_string(), C_GRAY)
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {}", risk_str),
                Style::default().fg(risk_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(" tgt {:.0}/{:.0}", stats.target_undefined_pct, 100.0 - stats.target_undefined_pct),
                Style::default().fg(C_GRAY),
            )]),
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Risk ", Style::default().fg(C_CYAN)))),
        kpi[6],
    );

    // Card 8 — VIX
    let (vix_str, vix_regime) = stats.vix.map_or(("—".to_string(), "".to_string()), |v| {
        let regime = if v >= 40.0 { "[STRESS]" } else if v >= 30.0 { "[HIGH]" } else if v >= 20.0 { "[ELEVATED]" } else if v >= 15.0 { "[NORMAL]" } else { "[CALM]" };
        (format!("{:.1}", v), regime.to_string())
    });
    let vix_color = stats.vix.map_or(C_GRAY, |v| {
        if v > 30.0 { C_RED } else if v > 20.0 { C_YELLOW } else { C_GREEN }
    });
    let vix_ivr_line = if let Some(avg_ivr) = stats.avg_ivr_open {
        let ivr_color = if avg_ivr >= 50.0 { C_GREEN } else if avg_ivr >= 25.0 { C_YELLOW } else { C_RED };
        Line::from(vec![
            Span::styled(" Avg IVR: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.0}", avg_ivr), Style::default().fg(ivr_color)),
        ])
    } else {
        Line::from(vec![Span::styled(" env fear", Style::default().fg(C_GRAY))])
    };
    let vix_suggestion = stats.vix.map(|v| {
        if v >= 40.0 { "→ Max/straddles" }
        else if v >= 30.0 { "→ Max/CSPs" }
        else if v >= 20.0 { "→ Normal+ ICs" }
        else if v >= 15.0 { "→ Std premium" }
        else { "→ Reduce/cals" }
    }).unwrap_or("");
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {} {}", vix_str, vix_regime),
                Style::default().fg(vix_color).add_modifier(Modifier::BOLD),
            )]),
            vix_ivr_line,
            Line::from(vec![Span::styled(format!(" {}", vix_suggestion), Style::default().fg(C_GRAY))]),
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" VIX ", Style::default().fg(C_CYAN)))),
        kpi[7],
    );

    // Card 9 — POP + P50
    let pop_color = if stats.avg_pop >= 68.0 { C_GREEN }
        else if stats.avg_pop >= 50.0 { C_YELLOW }
        else { C_RED };
    let p50_line = if let Some(p50) = stats.avg_p50_open {
        let p50_color = if p50 >= 60.0 { C_GREEN } else if p50 >= 45.0 { C_YELLOW } else { C_RED };
        Line::from(vec![
            Span::styled(" P50: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.0}%", p50), Style::default().fg(p50_color)),
        ])
    } else {
        Line::from(vec![Span::styled(format!(" {} open", stats.open_trades), Style::default().fg(C_GRAY))])
    };
    // M3: count open trades where short strike is within 3% of current price
    let near_be_count = trades.iter()
        .filter(|t| t.is_open())
        .filter(|t| {
            if let Some(u) = t.underlying_price {
                let sp = t.legs.iter().find(|l| l.leg_type == LegType::ShortPut).map(|l| l.strike);
                let sc = t.legs.iter().find(|l| l.leg_type == LegType::ShortCall).map(|l| l.strike);
                let otm = match (sp, sc) {
                    (Some(p), Some(c)) => Some(((u - p) / u * 100.0).min((c - u) / u * 100.0)),
                    (Some(p), None)    => Some((u - p) / u * 100.0),
                    (None,    Some(c)) => Some((c - u) / u * 100.0),
                    _                  => None,
                };
                otm.map_or(false, |o| o < 3.0)
            } else {
                false
            }
        })
        .count();
    let near_be_line = if near_be_count > 0 {
        Line::from(vec![Span::styled(format!(" {} near BE", near_be_count), Style::default().fg(C_RED))])
    } else {
        Line::from("")
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {:.1}%", stats.avg_pop),
                Style::default().fg(pop_color).add_modifier(Modifier::BOLD),
            )]),
            p50_line,
            near_be_line,
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" POP ", Style::default().fg(C_CYAN)))),
        kpi[8],
    );

    // ── Row 2: Next critical positions footer ─────────────────────────────────
    {
        let mut footer_spans: Vec<Span> = vec![Span::styled(" ⚡ Next: ", Style::default().fg(C_GRAY))];
        if stats.next_critical_positions.is_empty() {
            footer_spans.push(Span::styled("No positions expiring within 21d", Style::default().fg(C_GRAY)));
        } else {
            for (i, (ticker, dte)) in stats.next_critical_positions.iter().enumerate() {
                if i > 0 { footer_spans.push(Span::styled("  |  ", Style::default().fg(C_GRAY))); }
                let dte_color = if *dte <= 7 { C_RED } else if *dte <= 14 { C_YELLOW } else { C_WHITE };
                footer_spans.push(Span::styled(
                    format!("{} {}d", ticker, dte),
                    Style::default().fg(dte_color),
                ));
            }
        }
        // M2: Days since last trade entry
        if let Some(last_date) = trades.iter().map(|t| t.trade_date.date_naive()).max() {
            let days_ago = (Utc::now().date_naive() - last_date).num_days().max(0);
            let entry_color = if days_ago > 14 { C_RED } else { Color::Rgb(148, 163, 184) };
            footer_spans.push(Span::styled("    ", Style::default()));
            footer_spans.push(Span::styled(
                format!("Last entry: {}d ago", days_ago),
                Style::default().fg(entry_color),
            ));
        }
        f.render_widget(Paragraph::new(vec![Line::from(footer_spans)]), footer1_area);
    }

    // ── Row 3: Today's Actions summary ────────────────────────────────────────
    {
        use crate::actions::AlertKind;
        let action_alerts: Vec<&crate::actions::TradeAlert> = alerts.iter()
            .filter(|a| a.kind != AlertKind::Ok)
            .collect();
        let has_defense = action_alerts.iter().any(|a| matches!(a.kind, AlertKind::Defense | AlertKind::MaxLoss | AlertKind::Drawdown));
        let line = if action_alerts.is_empty() {
            Line::from(vec![Span::styled(" \u{2713} Clear \u{2014} no actions needed today", Style::default().fg(C_GREEN))])
        } else {
            let tickers: Vec<String> = {
                let mut seen = std::collections::HashSet::new();
                action_alerts.iter()
                    .filter(|a| a.ticker != "\u{2014}" && a.ticker != "PORTFOLIO" && a.ticker != "VIX")
                    .filter(|a| seen.insert(a.ticker.as_str()))
                    .take(6)
                    .map(|a| format!("[{}]", a.ticker))
                    .collect()
            };
            let color = if has_defense { C_RED } else { C_YELLOW };
            let ticker_str = if tickers.is_empty() { String::new() } else { format!(": {}", tickers.join(" ")) };
            Line::from(vec![Span::styled(
                format!(" \u{26A1} {} action{}{}", action_alerts.len(), if action_alerts.len() == 1 { "" } else { "s" }, ticker_str),
                Style::default().fg(color),
            )])
        };
        f.render_widget(Paragraph::new(vec![line]), footer2_area);
    }

    // ── Row 4: Risk panel | Open positions + Equity ───────────────────────────
    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(bottom_area);

    // ── Left: Risk Distribution panel
    let bar_width = (bot[0].width as usize).saturating_sub(8).max(8);
    let undef_bars = ((stats.undefined_risk_pct / 100.0) * bar_width as f64).round() as usize;
    let def_bars   = ((stats.defined_risk_pct   / 100.0) * bar_width as f64).round() as usize;
    let undef_bar = "▓".repeat(undef_bars) + &"░".repeat(bar_width.saturating_sub(undef_bars));
    let def_bar   = "▓".repeat(def_bars)   + &"░".repeat(bar_width.saturating_sub(def_bars));

    let drift_abs = stats.drift.abs();
    let (drift_str, drift_color) = if drift_abs <= 5.0 {
        ("On target".to_string(), C_GREEN)
    } else if stats.drift > 0.0 {
        (format!("+{:.1}% over", stats.drift), C_YELLOW)
    } else {
        (format!("{:.1}% under", stats.drift), C_YELLOW)
    };

    let mut risk_lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Undefined ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!("{:.0}%  ${:.0}", stats.undefined_risk_pct, stats.undefined_risk_bpr),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}", undef_bar), Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
            Span::styled("  Defined   ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!("{:.0}%  ${:.0}", stats.defined_risk_pct, stats.defined_risk_bpr),
                Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}", def_bar), Style::default().fg(C_CYAN)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Target   ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!(
                    "{:.0} / {:.0}",
                    stats.target_undefined_pct,
                    100.0 - stats.target_undefined_pct
                ),
                Style::default().fg(C_WHITE),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Drift    ", Style::default().fg(C_GRAY)),
            Span::styled(drift_str, Style::default().fg(drift_color)),
        ]),
        Line::from(vec![
            Span::styled("  Win Rate ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!("{:.1}%", stats.win_rate * 100.0),
                Style::default().fg(
                    if stats.win_rate >= 0.65 { C_GREEN }
                    else if stats.win_rate >= 0.50 { C_YELLOW }
                    else { C_RED }
                ),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Θ/day    ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!("${:.2}", stats.net_theta),
                Style::default().fg(if stats.net_theta >= 0.0 { C_GREEN } else { C_YELLOW }),
            ),
        ]),
        {
            // tastytrade KPI: Θ/NetLiq — target 0.1–0.3% daily
            if let Some(ratio) = stats.theta_netliq_ratio {
                let nl_color = if ratio >= 0.1 && ratio <= 0.3 { C_GREEN }
                               else if ratio < 0.1 { C_YELLOW }
                               else { C_RED };
                let bar = if ratio < 0.1 { "░▓░" } else if ratio <= 0.3 { "▓▓░" } else { "▓▓▓" };
                Line::from(vec![
                    Span::styled("  Θ/NetLiq ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("{:.2}%", ratio), Style::default().fg(nl_color)),
                    Span::styled(format!(" [{}] 0.1–0.3%", bar), Style::default().fg(C_GRAY)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("  Θ/NetLiq ", Style::default().fg(C_GRAY)),
                    Span::styled("—", Style::default().fg(C_GRAY)),
                ])
            }
        },
        {
            // Θ/BPR efficiency — theta earned per dollar of capital committed per day
            if let Some(ratio) = stats.theta_bpr_ratio {
                let bpr_color = if ratio >= 0.05 { C_GREEN } else if ratio >= 0.02 { C_YELLOW } else { C_RED };
                Line::from(vec![
                    Span::styled("  Θ/BPR    ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("{:.3}%/d", ratio), Style::default().fg(bpr_color)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("  Θ/BPR    ", Style::default().fg(C_GRAY)),
                    Span::styled("—", Style::default().fg(C_GRAY)),
                ])
            }
        },
        {
            let bwd_val = stats.net_beta_weighted_delta;
            // tastytrade goal: delta-neutral (BWD near 0). Near 0 = green, far = red.
            let (bwd_str, bwd_color) = if stats.spy_price.is_none() {
                ("N/A".to_string(), C_GRAY)
            } else {
                let abs_bwd = bwd_val.abs();
                let c = if abs_bwd <= 5.0 { C_GREEN }
                         else if abs_bwd <= 15.0 { C_YELLOW }
                         else { C_RED };
                (format!("{:+.1} Δ", bwd_val), c)
            };
            Line::from(vec![
                Span::styled("  β-WΔ     ", Style::default().fg(C_GRAY)),
                Span::styled(bwd_str, Style::default().fg(bwd_color)),
            ])
        },
        {
            let ratio = if stats.net_beta_weighted_delta.abs() > 0.1 {
                stats.net_theta / stats.net_beta_weighted_delta.abs()
            } else {
                stats.net_theta // if delta is near 0, ratio is essentially pure theta
            };
            let (r_str, r_color) = if ratio >= 1.0 {
                (format!("{:.2} (Theta-Dom)", ratio), C_GREEN)
            } else if ratio >= 0.5 {
                (format!("{:.2} (Balanced)", ratio), C_YELLOW)
            } else {
                (format!("{:.2} (Delta-Dom)", ratio), C_RED)
            };
            Line::from(vec![
                Span::styled("  Θ/|Δ| Ratio ", Style::default().fg(C_GRAY)),
                Span::styled(r_str, Style::default().fg(r_color)),
            ])
        },
        Line::from(vec![
            Span::styled("  Max DD   ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!("-${:.0} ({:.1}%)", stats.max_drawdown, stats.max_drawdown_pct),
                Style::default().fg(if stats.max_drawdown > 0.0 { C_RED } else { C_GREEN }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Avg ROC  ", Style::default().fg(C_GRAY)),
            Span::styled(
                format!("{:.1}%", stats.avg_roc),
                Style::default().fg(if stats.avg_roc >= 0.0 { C_GREEN } else { C_RED }),
            ),
        ]),
    ];

    // ── H7: Open Risk Summary ──────────────────────────────────────────────
    {
        let mut def_max_loss = 0.0_f64;
        let mut undef_bpr    = 0.0_f64;
        let mut undef_count  = 0usize;
        for t in trades.iter().filter(|t| t.is_open()) {
            let ml = calculate_max_loss_from_legs(&t.legs, t.credit_received, t.quantity, t.spread_type());
            if ml > 0.0 {
                def_max_loss += ml;
            } else if let Some(b) = t.bpr {
                undef_bpr += b;
                undef_count += 1;
            }
        }
        if def_max_loss > 0.0 || undef_count > 0 {
            risk_lines.push(Line::from(""));
            if def_max_loss > 0.0 {
                let pct = if stats.account_size > 0.0 { def_max_loss / stats.account_size * 100.0 } else { 0.0 };
                let ml_color = if pct > 20.0 { C_RED } else if pct > 10.0 { C_YELLOW } else { C_GREEN };
                risk_lines.push(Line::from(vec![
                    Span::styled("  Def max loss  ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("${:.0}", def_max_loss), Style::default().fg(ml_color)),
                    Span::styled(format!(" ({:.1}%)", pct), Style::default().fg(ml_color)),
                ]));
            }
            if undef_count > 0 {
                risk_lines.push(Line::from(vec![
                    Span::styled("  Undef BPR     ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("${:.0}", undef_bpr), Style::default().fg(C_YELLOW)),
                    Span::styled(format!(" ({} pos)", undef_count), Style::default().fg(C_GRAY)),
                ]));
            }
        }
    }

    // ── Sector Concentration (open trades only) ───────────────────────────
    let total_open_bpr: f64 = trades.iter()
        .filter(|t| t.is_open())
        .filter_map(|t| t.bpr)
        .sum();

    let mut sector_bpr_map: std::collections::HashMap<&str, (f64, Vec<&str>)> =
        std::collections::HashMap::new();
    for t in trades.iter().filter(|t| t.is_open()) {
        if let (Some(b), Some(sec)) = (t.bpr, t.sector.as_deref()) {
            let entry = sector_bpr_map.entry(sec).or_insert((0.0, Vec::new()));
            entry.0 += b;
            let ticker_ref: &str = t.ticker.as_str();
            if !entry.1.contains(&ticker_ref) {
                entry.1.push(ticker_ref);
            }
        }
    }
    let mut sector_rows: Vec<(&str, f64, Vec<&str>)> = sector_bpr_map
        .into_iter()
        .map(|(s, (b, tickers))| (s, b, tickers))
        .collect();
    sector_rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Divider
    risk_lines.push(Line::from(""));
    risk_lines.push(Line::from(vec![
        Span::styled(
            format!("  {}", "─".repeat((bot[0].width as usize).saturating_sub(4).min(32))),
            Style::default().fg(C_DARK),
        ),
    ]));
    risk_lines.push(Line::from(vec![
        Span::styled("  Sector Concentration", Style::default().fg(C_CYAN)),
    ]));
    risk_lines.push(Line::from(""));

    if sector_rows.is_empty() {
        risk_lines.push(Line::from(vec![
            Span::styled("  —", Style::default().fg(C_GRAY)),
        ]));
    } else {
        let sec_bar_max = (bot[0].width as usize).saturating_sub(24).max(4);
        for (sector, bpr, tickers) in &sector_rows {
            let pct = if total_open_bpr > 0.0 { bpr / total_open_bpr * 100.0 } else { 0.0 };
            let filled = ((pct / 100.0) * sec_bar_max as f64).round() as usize;
            let bar = "▓".repeat(filled) + &"░".repeat(sec_bar_max.saturating_sub(filled));
            let ticker_str = tickers.join(", ");

            // Line 1: sector name + count
            risk_lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ({})", sector, tickers.len()),
                    Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD),
                ),
            ]));
            // Lines 2+: tickers wrapped to panel width (4-char indent, 2-char border)
            let ticker_avail = (bot[0].width as usize).saturating_sub(6).max(10);
            for wrapped_line in word_wrap(&ticker_str, ticker_avail) {
                risk_lines.push(Line::from(vec![
                    Span::styled(format!("    {}", wrapped_line), Style::default().fg(Color::Rgb(148, 163, 184))),
                ]));
            }
            // Final line: bar + $ + %
            let conc_color = if pct > 40.0 { C_RED } else if pct > 30.0 { C_YELLOW } else { C_BLUE };
            risk_lines.push(Line::from(vec![
                Span::styled(format!("  {}", bar), Style::default().fg(conc_color)),
                Span::styled(format!("  ${:.0}", bpr), Style::default().fg(C_GRAY)),
                Span::styled(format!("  {:.0}%", pct), Style::default().fg(conc_color)),
            ]));
        }
    }

    let risk_border_color = if panel_focus == 0 { C_CYAN } else { C_BLUE };
    let risk_title_style = if panel_focus == 0 {
        Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(C_CYAN)
    };
    f.render_widget(
        Paragraph::new(risk_lines)
            .scroll((risk_scroll as u16, 0))
            .block(Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(risk_border_color))
                .title(Span::styled(" Risk Distribution ", risk_title_style))),
        bot[0],
    );

    // ── Right: Strategy bar (1 line) + Open positions + Equity sparkline
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(bot[1]);

    // L3: strategy distribution bar
    {
        let total_open = stats.open_strategy_counts.iter().map(|(_, c)| c).sum::<usize>().max(1);
        let bar_w = (right_rows[0].width as usize).saturating_sub(4).max(10);
        let mut spans: Vec<Span> = vec![Span::styled(" ", Style::default())];
        for (badge, count) in &stats.open_strategy_counts {
            if *count == 0 { continue; }
            let blocks = ((*count as f64 / total_open as f64) * 8.0).round() as usize;
            let _ = bar_w;
            let bc = badge_color(crate::models::StrategyType::from_str(
                // Convert badge back to snake_case via a map
                match badge.as_str() {
                    "SPV" => "short_put_vertical",
                    "SCV" => "short_call_vertical",
                    "IC"  => "iron_condor",
                    "IB"  => "iron_butterfly",
                    "STR" => "strangle",
                    "STD" => "straddle",
                    "CAL" => "calendar_spread",
                    "CSP" => "cash_secured_put",
                    "CC"  => "covered_call",
                    "PMCC" => "pmcc",
                    _ => "custom",
                }
            ).as_str());
            spans.push(Span::styled(badge.clone(), Style::default().fg(bc).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(format!(" {} ", "█".repeat(blocks.max(1))), Style::default().fg(bc)));
            spans.push(Span::styled(format!("{}", count), Style::default().fg(C_WHITE)));
            spans.push(Span::styled("  ", Style::default()));
        }
        f.render_widget(Paragraph::new(vec![Line::from(spans)]), right_rows[0]);
    }

    let open_trades: Vec<&Trade> = trades.iter().filter(|t| t.is_open()).collect();
    let today      = Utc::now().date_naive();
    let c_orange   = Color::Rgb(249, 115, 22);

    let open_header = Row::new(vec![
        Cell::from("Ticker").style(Style::default().fg(C_CYAN)),
        Cell::from("Opened").style(Style::default().fg(C_CYAN)),
        Cell::from("Str").style(Style::default().fg(C_CYAN)),
        Cell::from("Credit").style(Style::default().fg(C_CYAN)),
        Cell::from("GTC").style(Style::default().fg(C_GRAY)),
        Cell::from("DTE").style(Style::default().fg(C_CYAN)),
        Cell::from("ER").style(Style::default().fg(C_CYAN)),
        Cell::from("BPR").style(Style::default().fg(C_CYAN)),
        Cell::from("BPR%").style(Style::default().fg(C_GRAY)),
        Cell::from("OTM%").style(Style::default().fg(C_CYAN)),
        Cell::from("P&L%").style(Style::default().fg(C_CYAN)),
        Cell::from("Action").style(Style::default().fg(C_GRAY)),
    ])
    .style(Style::default().bg(C_DARK));

    let open_rows: Vec<Row> = open_trades.iter().map(|t| {
        let dte   = calculate_remaining_dte(&t.expiration_date);
        let dte_c = if dte <= 14 { C_RED } else if dte <= 21 { C_YELLOW } else { C_GREEN };
        let (er_str, er_style) = match t.next_earnings {
            Some(ed) => {
                let days = (ed - today).num_days();
                if days < 0 {
                    ("—".to_string(), Style::default().fg(C_GRAY))
                } else {
                    let style = if days <= 4 {
                        Style::default().fg(C_RED).add_modifier(Modifier::SLOW_BLINK)
                    } else if days <= 7 {
                        Style::default().fg(c_orange)
                    } else if days <= 14 {
                        Style::default().fg(C_YELLOW)
                    } else {
                        Style::default().fg(C_GRAY)
                    };
                    (format!("{}d", days), style)
                }
            }
            None => ("—".to_string(), Style::default().fg(C_GRAY)),
        };
        let bpr_str  = t.bpr.map_or("\u{2014}".to_string(), |b| format!("${:.0}", b));
        let cr_color = if t.credit_received >= 0.0 { C_GREEN } else { C_RED };
        // OTM% for dashboard — exposes otm_pct scalar for Action column
        let (otm_pct, dash_otm_cell) = {
            if let Some(u) = t.underlying_price {
                let short_put = t.legs.iter().find(|l| l.leg_type == crate::models::LegType::ShortPut)
                    .map(|l| l.strike);
                let short_call = t.legs.iter().find(|l| l.leg_type == crate::models::LegType::ShortCall)
                    .map(|l| l.strike);
                let otm = match (short_put, short_call) {
                    (Some(sp), Some(sc)) => {
                        let put_otm  = (u - sp) / u * 100.0;
                        let call_otm = (sc - u) / u * 100.0;
                        Some(put_otm.min(call_otm))
                    }
                    (Some(sp), None) => Some((u - sp) / u * 100.0),
                    (None, Some(sc)) => Some((sc - u) / u * 100.0),
                    _ => None,
                };
                let cell = match otm {
                    Some(p) if p >= 0.0 => {
                        let c = if p < 5.0 { C_RED } else if p < 10.0 { C_YELLOW } else { C_GREEN };
                        Cell::from(format!("{:.1}%", p)).style(Style::default().fg(c))
                    }
                    Some(_) => Cell::from("ITM").style(Style::default().fg(C_RED)),
                    None    => Cell::from("\u{2014}").style(Style::default().fg(C_GRAY)),
                };
                (otm, cell)
            } else {
                (None, Cell::from("\u{2014}").style(Style::default().fg(C_GRAY)))
            }
        };
        // Est P&L% (theta-based % of max profit captured) — exposes pct_of_max for Action column
        let (pct_of_max, dash_pnl_pct_cell) = {
            let max_profit_d = crate::calculations::calculate_max_profit(t.credit_received, t.quantity);
            if max_profit_d > 0.0 {
                let held = (Utc::now().date_naive() - t.trade_date.date_naive()).num_days().max(0) as f64;
                let theta_est = t.theta.map_or(0.0, |th| th * held * 100.0 * t.quantity as f64);
                let pct = (theta_est / max_profit_d * 100.0).clamp(-999.0, 999.0);
                let c = if pct >= 50.0 { C_GREEN } else if pct >= 25.0 { C_YELLOW } else { C_GRAY };
                (pct, Cell::from(format!("{:.0}%", pct)).style(Style::default().fg(c)))
            } else {
                (0.0_f64, Cell::from("\u{2014}").style(Style::default().fg(C_GRAY)))
            }
        };
        // ── GTC: Good Till Cancel buy-to-close target price ───────────────────────
        let target_pct = t.target_profit_pct
            .unwrap_or_else(|| t.strategy.default_profit_target_pct());
        let (gtc_str, gtc_color) = if t.credit_received > 0.0 {
            let gtc = t.credit_received * (1.0 - target_pct / 100.0);
            (format!("${:.2}", gtc), C_CYAN)
        } else {
            ("\u{2014}".to_string(), C_GRAY)
        };

        // ── BPR%: BPR as % of account size ────────────────────────────────────────
        let (bpr_pct_str, bpr_pct_color) = match t.bpr {
            Some(b) if stats.account_size > 0.0 => {
                let pct = b / stats.account_size * 100.0;
                let color = if pct > 5.0 { C_RED } else if pct > 3.0 { C_YELLOW } else if pct >= 1.0 { C_GREEN } else { C_YELLOW };
                (format!("{:.1}%", pct), color)
            }
            _ => ("\u{2014}".to_string(), C_GRAY),
        };

        // ── Action: tastytrade management signal ──────────────────────────────────
        let sp_strike = t.legs.iter()
            .find(|l| l.leg_type == crate::models::LegType::ShortPut)
            .map(|l| l.strike);
        let sc_strike = t.legs.iter()
            .find(|l| l.leg_type == crate::models::LegType::ShortCall)
            .map(|l| l.strike);
        let put_itm  = sp_strike.map(|sp| t.underlying_price.map_or(false, |u| u < sp)).unwrap_or(false);
        let call_itm = sc_strike.map(|sc| t.underlying_price.map_or(false, |u| u > sc)).unwrap_or(false);

        let (action_str, action_color): (&str, Color) =
            if pct_of_max >= target_pct || pct_of_max >= 50.0 {
                ("TAKE", C_GREEN)
            } else if pct_of_max <= -200.0 {
                ("STOP", C_RED)
            } else if dte <= 7 {
                ("CLOSE", C_RED)
            } else if put_itm {
                ("ROLL\u{2193}", C_RED)
            } else if call_itm {
                ("ROLL\u{2191}", C_RED)
            } else if otm_pct.map_or(false, |o| o < 5.0) && dte <= 21 {
                ("DEFEND", C_YELLOW)
            } else if dte <= 21 {
                ("21DTE", C_YELLOW)
            } else if otm_pct.map_or(false, |o| o < 5.0) {
                ("TEST", C_YELLOW)
            } else {
                ("\u{2014}", C_GRAY)
            };

        Row::new(vec![
            Cell::from(t.ticker.clone()).style(Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            Cell::from(t.trade_date.format("%m/%d/%y").to_string()).style(Style::default().fg(C_GRAY)),
            Cell::from(t.strategy.badge()).style(Style::default().fg(badge_color(t.spread_type()))),
            Cell::from(format!("${:.2}", t.credit_received)).style(Style::default().fg(cr_color)),
            Cell::from(gtc_str).style(Style::default().fg(gtc_color)),
            Cell::from(format!("{}", dte)).style(Style::default().fg(dte_c)),
            Cell::from(er_str).style(er_style),
            Cell::from(bpr_str).style(Style::default().fg(C_YELLOW)),
            Cell::from(bpr_pct_str).style(Style::default().fg(bpr_pct_color)),
            dash_otm_cell,
            dash_pnl_pct_cell,
            Cell::from(action_str).style(Style::default().fg(action_color)),
        ])
    }).collect();

    let mut open_table_state = TableState::default();
    *open_table_state.offset_mut() = open_scroll.min(open_trades.len().saturating_sub(1));
    f.render_stateful_widget(
        Table::new(open_rows, [
            Constraint::Length(7),  // Ticker
            Constraint::Length(8),  // Opened
            Constraint::Length(5),  // Str
            Constraint::Length(7),  // Credit
            Constraint::Length(6),  // GTC
            Constraint::Length(4),  // DTE
            Constraint::Length(5),  // ER
            Constraint::Length(8),  // BPR
            Constraint::Length(5),  // BPR%
            Constraint::Length(5),  // OTM%
            Constraint::Length(5),  // P&L%
            Constraint::Length(6),  // Action
        ])
        .header(open_header)
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(if panel_focus == 1 { C_CYAN } else { C_BLUE }))
            .title(Span::styled(
                format!(" Open Positions ({}) ↑↓ ", open_trades.len()),
                if panel_focus == 1 {
                    Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(C_CYAN)
                },
            ))),
        right_rows[1],
        &mut open_table_state,
    );

    let mut closed: Vec<&Trade> = trades.iter().filter(|t| t.pnl.is_some()).collect();
    closed.sort_by_key(|t| t.exit_date.unwrap_or(t.trade_date));
    draw_equity_curve(f, right_rows[2], &closed);
}

fn month_full(month: u32) -> &'static str {
    match month {
        1  => "Jan",  2  => "Feb",  3  => "Mar",
        4  => "Apr",  5  => "May",  6  => "Jun",
        7  => "Jul",  8  => "Aug",  9  => "Sep",
        10 => "Oct",  11 => "Nov",  12 => "Dec",
        _  => "???",
    }
}

// ── Journal ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_journal(
    f:                &mut Frame,
    area:             Rect,
    display_count:    usize,
    visual_rows:      &[VisualRowKind],
    collapsed_months: &HashSet<(i32, u32)>,
    all_trades:       &[Trade],
    table_state:      &mut TableState,
    show_detail:      bool,
    detail_scroll:    u16,
    trade_chain:      &[Trade],
    filter_status:    FilterStatus,
    filter_ticker:    &str,
    sort_key:         SortKey,
    sort_desc:        bool,
    app_mode:         AppMode,
    edit_fields:      &[EditField],
    edit_field_idx:   usize,
    edit_scroll:      u16,
    close_fields:     &[EditField],
    close_field_idx:  usize,
    live_prices:      &std::collections::HashMap<String, f64>,
    under_tauri:      bool,
    playbooks:        &[crate::models::PlaybookStrategy],
    col_visibility:     &[bool; 22],
    show_col_picker:    bool,
    journal_chain_view: bool,
    account_size:       f64,
    max_pos_bpr_pct:    f64,
    default_profit_target_pct: f64,
) {
    let _ = show_col_picker; // handled in draw_ui overlay
    // Filter bar (1 line, always visible)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    draw_filter_bar(
        f, layout[0],
        filter_status, filter_ticker, sort_key, sort_desc,
        app_mode, display_count, all_trades,
    );

    let main_area = layout[1];

    // Helper: resolve selected visual row → Trade
    let ctx_trade = |state: &TableState| -> Option<&Trade> {
        let i = state.selected()?;
        match visual_rows.get(i)? {
            VisualRowKind::Trade(ti) => all_trades.get(*ti),
            VisualRowKind::YearHeader { .. } | VisualRowKind::MonthHeader { .. }
            | VisualRowKind::TickerHeader { .. } | VisualRowKind::ChainHeader { .. } => None,
        }
    };

    match app_mode {
        AppMode::AnalyzeTrade => {
            if under_tauri {
                // Svelte right pane handles the chart; table fills the full terminal width
                draw_trade_table(f, main_area, visual_rows, collapsed_months, all_trades, table_state, live_prices, playbooks, col_visibility, journal_chain_view, account_size, max_pos_bpr_pct, default_profit_target_pct);
            } else {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                    .split(main_area);
                draw_trade_table(f, split[0], visual_rows, collapsed_months, all_trades, table_state, live_prices, playbooks, col_visibility, journal_chain_view, account_size, max_pos_bpr_pct, default_profit_target_pct);
                if let Some(trade) = ctx_trade(table_state) {
                    draw_analyze_pane(f, split[1], trade);
                }
            }
        }
        AppMode::EditTrade => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(main_area);
            draw_trade_table(f, split[0], visual_rows, collapsed_months, all_trades, table_state, live_prices, playbooks, col_visibility, journal_chain_view, account_size, max_pos_bpr_pct, default_profit_target_pct);
            let ctx = ctx_trade(table_state);
            draw_edit_pane(f, split[1], edit_fields, edit_field_idx, edit_scroll, ctx);
        }
        AppMode::CloseTrade => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(main_area);
            draw_trade_table(f, split[0], visual_rows, collapsed_months, all_trades, table_state, live_prices, playbooks, col_visibility, journal_chain_view, account_size, max_pos_bpr_pct, default_profit_target_pct);
            let ctx = ctx_trade(table_state);
            draw_close_pane(f, split[1], close_fields, close_field_idx, ctx);
        }
        _ => {
            // Normal / FilterInput / ConfirmDelete
            let (table_area, detail_area) = if show_detail {
                let v = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                    .split(main_area);
                (v[0], Some(v[1]))
            } else {
                (main_area, None)
            };

            draw_trade_table(f, table_area, visual_rows, collapsed_months, all_trades, table_state, live_prices, playbooks, col_visibility, journal_chain_view, account_size, max_pos_bpr_pct, default_profit_target_pct);

            if let Some(det) = detail_area {
                if let Some(trade) = ctx_trade(table_state) {
                    draw_trade_detail(f, det, trade, detail_scroll, trade_chain, playbooks, live_prices, account_size, max_pos_bpr_pct);
                }
            }
        }
    }
}

// ── Filter bar ────────────────────────────────────────────────────────────────

fn draw_filter_bar(
    f:             &mut Frame,
    area:          Rect,
    filter_status: FilterStatus,
    filter_ticker: &str,
    sort_key:      SortKey,
    sort_desc:     bool,
    app_mode:      AppMode,
    visible_count: usize,
    all_trades:    &[Trade],
) {
    let total  = all_trades.len();
    let open_c = all_trades.iter().filter(|t| t.is_open()).count();

    let mut spans: Vec<Span> = vec![Span::styled(" Filter: ", Style::default().fg(C_GRAY))];

    for s in [FilterStatus::All, FilterStatus::Open, FilterStatus::Closed, FilterStatus::Rolled, FilterStatus::Expired] {
        let active = s == filter_status;
        let style = if active {
            Style::default().bg(C_BLUE).fg(C_WHITE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_GRAY)
        };
        let label = if active { format!("[●{}]", s.label()) } else { format!("[{}]", s.label()) };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }

    // Ticker search indicator
    let is_searching = app_mode == AppMode::FilterInput;
    spans.push(Span::styled("  /: ", Style::default().fg(C_GRAY)));
    let ticker_style = if is_searching {
        Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)
    } else if !filter_ticker.is_empty() {
        Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(C_GRAY)
    };
    let ticker_display = if filter_ticker.is_empty() {
        if is_searching { "▌".to_string() } else { "—".to_string() }
    } else if is_searching {
        format!("{}▌", filter_ticker.to_uppercase())
    } else {
        filter_ticker.to_uppercase()
    };
    spans.push(Span::styled(ticker_display, ticker_style));

    // Sort
    let dir = if sort_desc { "▼" } else { "▲" };
    spans.push(Span::styled(
        format!("   Sort:{}{}", sort_key.label(), dir),
        Style::default().fg(C_CYAN),
    ));

    // Counts
    spans.push(Span::styled(
        format!("   {}/{} shown  ({} open)", visible_count, total, open_c),
        Style::default().fg(C_GRAY),
    ));

    // M6: win rate of filtered set when ticker filter is active
    if !filter_ticker.is_empty() {
        let filter_up = filter_ticker.to_uppercase();
        let closed: Vec<_> = all_trades.iter()
            .filter(|t| t.ticker.to_uppercase().contains(&filter_up)
                     || t.spread_type().to_uppercase().contains(&filter_up))
            .filter(|t| !t.is_open())
            .filter(|t| t.pnl.is_some())
            .collect();
        if closed.len() >= 2 {
            let wins = closed.iter().filter(|t| t.pnl.unwrap_or(0.0) > 0.0).count();
            let wr = wins as f64 / closed.len() as f64 * 100.0;
            let wr_color = if wr >= 60.0 { C_GREEN } else if wr >= 45.0 { C_YELLOW } else { C_RED };
            spans.push(Span::styled("   Win: ", Style::default().fg(C_GRAY)));
            spans.push(Span::styled(
                format!("{:.0}%", wr),
                Style::default().fg(wr_color).add_modifier(Modifier::BOLD),
            ));
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(C_DARK)),
        area,
    );
}

// ── Trade table ───────────────────────────────────────────────────────────────

fn draw_trade_table(
    f:                  &mut Frame,
    area:               Rect,
    visual_rows:        &[VisualRowKind],
    collapsed_months:   &HashSet<(i32, u32)>,
    all_trades:         &[Trade],
    state:              &mut TableState,
    live_prices:        &std::collections::HashMap<String, f64>,
    playbooks:          &[crate::models::PlaybookStrategy],
    col_visibility:     &[bool; 22],
    journal_chain_view: bool,
    account_size:       f64,
    max_pos_bpr_pct:    f64,
    default_profit_target_pct: f64,
) {
    const COL_NAMES: [&str; 22] = ["Date", "Ticker", "Spot", "ER", "Str", "Qty", "Credit", "GTC", "BE", "BPR", "BPR%", "MaxPft", "P&L", "ROC%", "$V/d", "DTE", "Exit", "Held", "Status", "OTM%", "EM", "Mgmt"];

    let header = Row::new(
        COL_NAMES.iter().enumerate()
            .filter(|(i, _)| col_visibility[*i])
            .map(|(_, h)| Cell::from(*h).style(Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))),
    )
    .style(Style::default().bg(C_DARK))
    .height(1);

    let today = Utc::now().date_naive();
    let c_orange = Color::Rgb(249, 115, 22);

    // Count totals for the title
    let open_c   = all_trades.iter().filter(|t| t.is_open()).count();
    let closed_c = all_trades.iter().filter(|t| !t.is_open()).count();
    let shown_c  = visual_rows.iter().filter(|r| matches!(r, VisualRowKind::Trade(_))).count();

    let rows: Vec<Row> = visual_rows.iter().map(|vr| {
        match vr {
            VisualRowKind::YearHeader { year } => {
                // Year is collapsed when no MonthHeader for this year is in visual_rows
                let year_collapsed = !visual_rows.iter().any(|r| {
                    matches!(r, VisualRowKind::MonthHeader { year: y, .. } if y == year)
                });
                let arrow = if year_collapsed { "▶" } else { "▼" };
                let label = format!(" {} {}", arrow, year);
                Row::new(vec![
                    Cell::from(label).style(
                        Style::default()
                            .fg(Color::Rgb(147, 197, 253))
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(Style::default().bg(Color::Rgb(15, 23, 42)))
                .height(1)
            }
            VisualRowKind::MonthHeader { year, month } => {
                let collapsed = collapsed_months.contains(&(*year, *month));
                let arrow = if collapsed { "▶" } else { "▼" };
                let label = format!("    {} {}", arrow, month_full(*month));
                Row::new(vec![
                    Cell::from(label)
                        .style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
                ])
                .style(Style::default().bg(Color::Rgb(20, 20, 35)))
                .height(1)
            }
            VisualRowKind::TickerHeader { ticker, open_count, closed_count, net_pnl } => {
                let pnl_sign = if *net_pnl >= 0.0 { "+" } else { "" };
                let pnl_color = if *net_pnl >= 0.0 { C_GREEN } else { C_RED };
                let label = format!(
                    " ▼ {}    {} open · {} closed    net P&L: {}{:.0}",
                    ticker, open_count, closed_count, pnl_sign, net_pnl
                );
                Row::new(vec![
                    Cell::from(label).style(
                        Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD),
                    ),
                    Cell::from("").style(Style::default().fg(pnl_color)),
                ])
                .style(Style::default().bg(Color::Rgb(10, 30, 45)))
                .height(1)
            }
            VisualRowKind::ChainHeader { strategy, roll_count, net_credit, chain_pnl, chain_commissions, is_open, entry_date, .. } => {
                let status_dot = if *is_open { "●" } else if *chain_pnl > 0.0 { "✓" } else { "✗" };
                let status_color = if *is_open { C_YELLOW } else if *chain_pnl > 0.0 { C_GREEN } else { C_RED };
                let net_adj = net_credit - chain_commissions;
                let comm_str = if *chain_commissions > 0.0 {
                    format!("  (comm: -${:.0})", chain_commissions)
                } else {
                    String::new()
                };
                let pnl_str = if *is_open {
                    format!("Net P&L: —{}    {} OPEN", comm_str, status_dot)
                } else {
                    format!("Net P&L: {:+.0}{}    {} CLSD", net_adj, comm_str, status_dot)
                };
                let entry = entry_date.format("%b %d").to_string();
                let label = format!(
                    "  ├─ [{}]  {}  Rolls: {}  Credit: ${:.2}  {}",
                    strategy, entry, roll_count, net_credit, pnl_str
                );
                Row::new(vec![
                    Cell::from(label).style(Style::default().fg(C_GRAY)),
                    Cell::from("").style(Style::default().fg(status_color)),
                ])
                .style(Style::default().bg(Color::Rgb(20, 28, 40)))
                .height(1)
            }
            VisualRowKind::Trade(ti) => {
                let t = &all_trades[*ti];
        let pnl_str = match t.pnl {
            Some(p) => if p.abs() >= 10_000.0 {
                format!("{:+.1}k", p / 1000.0)
            } else {
                format!("${:+.2}", p)
            },
            None    => "open".to_string(),
        };
        let pnl_style = match t.pnl {
            Some(p) if p > 0.0 => Style::default().fg(C_GREEN),
            Some(_)             => Style::default().fg(C_RED),
            None                => Style::default().fg(C_YELLOW),
        };
        let max_profit = calculate_max_profit(t.credit_received, t.quantity);
        let roc = t.pnl.and_then(|p| calculate_roc(p, &t.legs, t.credit_received, t.quantity, t.spread_type(), t.bpr, t.underlying_price));
        let dte = calculate_remaining_dte(&t.expiration_date);
        let dte_c = if t.is_open() {
            if dte <= 14 { C_RED } else if dte <= 21 { C_YELLOW } else { C_GREEN }
        } else {
            C_GRAY
        };

        let scratch_threshold = (calculate_max_profit(t.credit_received, t.quantity) * 0.05).max(10.0);
        let is_scratch = t.pnl.map_or(false, |p| p.abs() < scratch_threshold);
        let status_str = if t.is_open() {
            "OPEN".to_string()
        } else if is_scratch {
            "SCRTCH".to_string()
        } else {
            match t.exit_reason.as_deref().filter(|r| !r.is_empty()) {
                Some("closed")  => "CLOSED".to_string(),
                Some("expired") => "EXPIRD".to_string(),
                Some("rolled")  => "ROLLED".to_string(),
                Some("stopped") => "STOPPD".to_string(),
                Some(other)     => other.chars().take(6).collect::<String>().to_uppercase(),
                None            => "CLOSED".to_string(),
            }
        };
        let status_style = if t.is_open() {
            Style::default().fg(C_YELLOW)
        } else if is_scratch {
            Style::default().fg(C_GRAY)
        } else {
            match t.exit_reason.as_deref() {
                Some("expired") => Style::default().fg(C_GRAY),
                Some("rolled")  => Style::default().fg(C_BLUE),
                _ => if t.pnl.map_or(false, |p| p > 0.0) {
                    Style::default().fg(C_GREEN)
                } else {
                    Style::default().fg(C_RED)
                },
            }
        };

        // ── Earnings flag (ER column) ─────────────────────────────────────
        let (er_str, er_style) = if t.is_open() {
            match t.next_earnings {
                Some(ed) => {
                    let days = (ed - today).num_days();
                    if days < 0 {
                        // Earnings already passed — show nothing
                        ("—".to_string(), Style::default().fg(C_GRAY))
                    } else {
                        let label = format!("ER {}d", days);
                        let style = if days <= 4 {
                            Style::default().fg(C_RED).add_modifier(Modifier::SLOW_BLINK)
                        } else if days <= 7 {
                            Style::default().fg(c_orange)
                        } else if days <= 14 {
                            Style::default().fg(C_YELLOW)
                        } else {
                            Style::default().fg(C_GRAY)
                        };
                        (label, style)
                    }
                }
                None => ("—".to_string(), Style::default().fg(C_GRAY)),
            }
        } else {
            ("—".to_string(), Style::default().fg(C_GRAY))
        };

        // ── GTC column ───────────────────────────────────────────────────
        let (gtc_str, gtc_style) = if t.is_open() {
            // Cascade: per-trade → global default (if >0) → per-strategy default
            let target = t.target_profit_pct.unwrap_or_else(|| {
                if default_profit_target_pct > 0.0 { default_profit_target_pct }
                else { t.strategy.default_profit_target_pct() }
            });
            let cr = t.credit_received;
            if cr > 0.0 {
                // Credit spread: close (buy back) at cr * (1 - target/100)
                let gtc = cr * (1.0 - target / 100.0);
                (format!("${:.2}DB", gtc), Style::default().fg(C_CYAN))
            } else if cr < 0.0 {
                // Debit spread: close (sell) at |cr| * (1 + target/100)
                let gtc = cr.abs() * (1.0 + target / 100.0);
                (format!("${:.2}CR", gtc), Style::default().fg(C_GREEN))
            } else {
                ("—".to_string(), Style::default().fg(C_GRAY))
            }
        } else {
            ("—".to_string(), Style::default().fg(C_GRAY))
        };

        // ── Breakeven column ─────────────────────────────────────────────
        let be_str = if t.is_open() {
            let bes = calculate_breakevens(&t.legs, t.spread_type(), None);
            match bes.len() {
                2 => format!("{:.0}/{:.0}", bes[0], bes[1]),
                1 => format!("{:.0}", bes[0]),
                _ => "—".to_string(),
            }
        } else {
            "—".to_string()
        };

                // L4: compliance check for open trades with playbook
                let has_compliance_violation = if t.is_open() {
                    t.playbook_id.and_then(|pid| playbooks.iter().find(|pb| pb.id == pid))
                        .and_then(|pb| pb.entry_criteria.as_ref())
                        .map(|ec| !crate::calculations::check_playbook_compliance(t, ec).is_empty())
                        .unwrap_or(false)
                } else {
                    false
                };
                let ticker_display = if has_compliance_violation {
                    format!("{} [!]", t.ticker)
                } else {
                    t.ticker.clone()
                };
                let ticker_style = if has_compliance_violation {
                    Style::default().fg(C_RED).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)
                };

                let all_cells: Vec<Cell> = vec![
                    Cell::from(t.trade_date.format("%m/%d/%y").to_string()).style(Style::default().fg(C_GRAY)),
                    Cell::from(ticker_display).style(ticker_style),
                    {
                        let (spot_str, spot_color) = if t.is_open() {
                            match live_prices.get(&t.ticker).copied() {
                                Some(p) => {
                                    let color = match t.underlying_price {
                                        Some(entry) if p < entry => C_RED,
                                        _ => C_GREEN,
                                    };
                                    (format!("${:.2}", p), color)
                                }
                                None => ("—".to_string(), C_GRAY),
                            }
                        } else {
                            ("—".to_string(), C_GRAY)
                        };
                        Cell::from(spot_str).style(Style::default().fg(spot_color))
                    },
                    Cell::from(er_str).style(er_style),
                    Cell::from(t.strategy.badge()).style(Style::default().fg(badge_color(t.spread_type()))),
                    Cell::from(t.quantity.to_string()),
                    Cell::from(format!("${:.2}", t.credit_received))
                        .style(Style::default().fg(if t.credit_received >= 0.0 { C_GREEN } else { C_RED })),
                    Cell::from(gtc_str).style(gtc_style),
                    Cell::from(be_str).style(Style::default().fg(C_GRAY)),
                    {
                        if t.is_open() {
                            let bpr_str = t.bpr.map_or("—".to_string(), |b| format!("${:.0}", b));
                            Cell::from(bpr_str).style(Style::default().fg(C_YELLOW))
                        } else {
                            Cell::from("—").style(Style::default().fg(C_GRAY))
                        }
                    },
                    // BPR% — position BPR relative to account size, traffic-light coloring
                    {
                        if !t.is_open() {
                            Cell::from("—").style(Style::default().fg(C_GRAY))
                        } else {
                            match t.bpr {
                                None => Cell::from("—").style(Style::default().fg(C_GRAY)),
                                Some(b) if account_size > 0.0 => {
                                    let pct = b / account_size * 100.0;
                                    let color = if pct > max_pos_bpr_pct { C_RED }
                                                else if pct > max_pos_bpr_pct * 0.75 { C_YELLOW }
                                                else { C_GREEN };
                                    Cell::from(format!("{:.1}%", pct)).style(Style::default().fg(color))
                                },
                                _ => Cell::from("—").style(Style::default().fg(C_GRAY)),
                            }
                        }
                    },
                    {
                        // PBWB with invalid strike ladder shows warning instead of max profit
                        let pbwb_invalid = t.spread_type() == "put_broken_wing_butterfly"
                            && calculate_max_loss_from_legs(&t.legs, t.credit_received, t.quantity, t.spread_type()) <= 0.0;
                        if pbwb_invalid {
                            Cell::from("chk strikes").style(Style::default().fg(C_YELLOW))
                        } else if max_profit <= 0.0 {
                            // Debit spreads (LDS, debit calendars): max profit is IV-dependent
                            Cell::from("\u{2014}").style(Style::default().fg(C_GRAY))
                        } else {
                            Cell::from(format!("${:.0}", max_profit)).style(Style::default().fg(C_GRAY))
                        }
                    },
                    Cell::from(pnl_str).style(pnl_style),
                    {
                        // ★ indicator: open trade estimated theta P&L ≥ 50% of max profit
                        let at_target = t.is_open() && {
                            let target = t.target_profit_pct.unwrap_or_else(|| {
                                if default_profit_target_pct > 0.0 { default_profit_target_pct }
                                else { t.strategy.default_profit_target_pct() }
                            });
                            let days_held = (Utc::now().date_naive() - t.trade_date.date_naive()).num_days().max(0) as f64;
                            let est_pnl = t.theta.map(|th| th.abs() * 100.0 * t.quantity as f64 * days_held)
                                .unwrap_or(0.0);
                            let profit_target = max_profit * (target / 100.0);
                            max_profit > 0.0 && est_pnl >= profit_target && profit_target > 0.0
                        };
                        let roc_str = roc.map_or("—".to_string(), |r| {
                            if at_target { format!("{:.1}%★", r) } else { format!("{:.1}%", r) }
                        });
                        let roc_style = roc.map_or(Style::default().fg(C_GRAY), |r| {
                            if at_target { Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD) }
                            else if r > 0.0 { Style::default().fg(C_GREEN) }
                            else { Style::default().fg(C_RED) }
                        });
                        Cell::from(roc_str).style(roc_style)
                    },
                    {
                        let v_opt = calculate_pnl_per_day(t.pnl, &t.entry_date, t.exit_date.as_ref());
                        let (v_str, v_style) = match v_opt {
                            Some(v) => (format!("${:.0}", v), if v >= 0.0 { Style::default().fg(C_CYAN) } else { Style::default().fg(C_RED) }),
                            None => ("—".to_string(), Style::default().fg(C_GRAY)),
                        };
                        Cell::from(v_str).style(v_style)
                    },
                    Cell::from(if t.is_open() { format!("{}", dte) } else { "—".to_string() })
                        .style(Style::default().fg(dte_c)),
                    Cell::from(t.exit_date.map(|d| d.format("%m/%d/%y").to_string()).unwrap_or_else(|| "—".to_string()))
                        .style(Style::default().fg(C_GRAY)),
                    {
                        let (days, _, _) = match t.exit_date {
                            Some(exit) => calculate_held_duration(&t.entry_date, &exit),
                            None => calculate_held_duration(&t.entry_date, &Utc::now()),
                        };
                        Cell::from(format!("{}d", days)).style(Style::default().fg(C_GRAY))
                    },
                    Cell::from(status_str).style(status_style),
                    // OTM% — how far short strike is from current underlying (open only)
                    {
                        if t.is_open() {
                            let underlying = live_prices.get(&t.ticker).copied()
                                .or(t.underlying_price);
                            match underlying {
                                Some(u) if u > 0.0 => {
                                    let short_put = t.legs.iter().find(|l| l.leg_type == crate::models::LegType::ShortPut)
                                        .map(|l| l.strike);
                                    let short_call = t.legs.iter().find(|l| l.leg_type == crate::models::LegType::ShortCall)
                                        .map(|l| l.strike);
                                    let otm_pct = match (short_put, short_call) {
                                        (Some(sp), Some(sc)) => {
                                            // IC/strangle: show the TIGHTER side (least cushion)
                                            let put_otm  = (u - sp) / u * 100.0;
                                            let call_otm = (sc - u) / u * 100.0;
                                            Some(put_otm.min(call_otm))
                                        }
                                        (Some(sp), None) => Some((u - sp) / u * 100.0),
                                        (None, Some(sc)) => Some((sc - u) / u * 100.0),
                                        _ => None,
                                    };
                                    match otm_pct {
                                        Some(pct) if pct >= 0.0 => {
                                            // ≥10% = well cushioned (green), 5-10% = normal (yellow), <5% = at risk (red)
                                            let color = if pct < 5.0 { C_RED } else if pct < 10.0 { C_YELLOW } else { C_GREEN };
                                            Cell::from(format!("{:.1}%", pct)).style(Style::default().fg(color))
                                        }
                                        Some(pct) => Cell::from(format!("-{:.1}%", pct.abs())).style(Style::default().fg(C_RED)), // ITM — negative shown in red
                                        None => Cell::from("\u{2014}").style(Style::default().fg(C_GRAY)),
                                    }
                                }
                                _ => Cell::from("\u{2014}").style(Style::default().fg(C_GRAY)),
                            }
                        } else {
                            Cell::from("\u{2014}").style(Style::default().fg(C_GRAY))
                        }
                    },
                    // Col 20: EM — Expected Move (open trades only)
                    {
                        if t.is_open() {
                            let remaining_dte = calculate_remaining_dte(&t.expiration_date) as f64;
                            let iv_raw = t.implied_volatility.or(t.iv_rank).unwrap_or(0.0);
                            if iv_raw > 0.0 && remaining_dte > 0.0 {
                                let iv_dec = if iv_raw > 2.0 { iv_raw / 100.0 } else { iv_raw };
                                let price = t.underlying_price
                                    .or_else(|| live_prices.get(&t.ticker).copied())
                                    .unwrap_or(0.0);
                                let em = price * iv_dec * (remaining_dte / 365.0).sqrt();
                                if em > 0.0 {
                                    Cell::from(format!("\u{b1}${:.2}", em)).style(Style::default().fg(C_CYAN))
                                } else {
                                    Cell::from("\u{2014}").style(Style::default().fg(C_GRAY))
                                }
                            } else {
                                Cell::from("\u{2014}").style(Style::default().fg(C_GRAY))
                            }
                        } else {
                            Cell::from("\u{2014}").style(Style::default().fg(C_GRAY))
                        }
                    },
                    // Col 21: Mgmt — 21 DTE management trigger date = expiration - 21 days
                    {
                        if t.is_open() {
                            let mgmt_date = t.expiration_date.date_naive() - chrono::Duration::days(21);
                            let days_remaining = (mgmt_date - today).num_days();
                            let mgmt_color = if days_remaining <= 0 { C_RED }
                                else if days_remaining <= 5 { C_YELLOW }
                                else { C_GRAY };
                            Cell::from(mgmt_date.format("%m/%d").to_string()).style(Style::default().fg(mgmt_color))
                        } else {
                            Cell::from("\u{2014}").style(Style::default().fg(C_GRAY))
                        }
                    },
                ];
                let visible_cells: Vec<Cell> = all_cells.into_iter().enumerate()
                    .filter(|(i, _)| col_visibility[*i])
                    .map(|(_, c)| c)
                    .collect();
                Row::new(visible_cells)
            }
        }
    }).collect();

    const COL_WIDTHS: [u16; 22] = [11, 7, 8, 7, 5, 4, 7, 9, 10, 7, 6, 7, 9, 7, 7, 5, 9, 6, 7, 6, 7, 6];
    let visible_widths: Vec<Constraint> = COL_WIDTHS.iter().enumerate()
        .filter(|(i, _)| col_visibility[*i])
        .map(|(_, &w)| Constraint::Length(w))
        .collect();

    let table = Table::new(rows, visible_widths)
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(
                if journal_chain_view {
                    format!(
                        " Trade Journal [Chain View] — {} shown  ({} open, {} closed total) ",
                        shown_c, open_c, closed_c
                    )
                } else {
                    format!(
                        " Trade Journal — {} shown  ({} open, {} closed total) ",
                        shown_c, open_c, closed_c
                    )
                },
                Style::default().fg(C_CYAN),
            )),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(30, 58, 138))
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    f.render_stateful_widget(table, area, state);
}

// ── Edit pane ─────────────────────────────────────────────────────────────────

fn draw_edit_pane(
    f:              &mut Frame,
    area:           Rect,
    fields:         &[EditField],
    focused_idx:    usize,
    edit_scroll:    u16,
    context_trade:  Option<&Trade>,
) {
    let title = context_trade.map_or_else(
        || " Edit Strategy  (Ctrl+S:Save  Esc:Cancel) ".to_string(),
        |t| format!(" Edit: {} [{}]  (Ctrl+S:Save  Esc:Cancel) ", t.ticker, t.strategy.badge()),
    );

    // Build lines + map field index → starting line number
    let mut lines: Vec<Line> = Vec::new();
    let mut field_line_starts: Vec<usize> = Vec::with_capacity(fields.len());

    for (i, field) in fields.iter().enumerate() {
        if let Some(hdr) = &field.section_header {
            if !hdr.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(hdr.clone(), Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD)),
                ]));
            }
        }
        field_line_starts.push(lines.len());

        let is_focused = i == focused_idx;

        // Button: render distinctly and skip the normal label+value line
        if let FieldKind::Button(btn_label) = &field.kind {
            let btn_style = if is_focused {
                Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(C_GRAY)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}", btn_label), btn_style),
            ]));
            continue;
        }

        let (lbl_style, val_style) = if is_focused {
            (
                Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
                Style::default().bg(Color::Rgb(30, 58, 138)).fg(C_WHITE),
            )
        } else {
            (Style::default().fg(C_GRAY), Style::default().fg(C_WHITE))
        };

        // Multiline: label header + structural value below (respects internal newlines)
        if field.kind == FieldKind::Multiline {
            let hint = if is_focused { " (Enter:newline  Backspace:del)" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:22}", field.label), lbl_style),
                Span::styled(hint, Style::default().fg(C_GRAY)),
            ]));
            
            let wrap_width = (area.width as usize).saturating_sub(6).max(20);
            let wrapped = word_wrap(&field.value, wrap_width);
            let last_idx = wrapped.len().saturating_sub(1);
            
            // If focused, use a slightly lighter background for the whole block
            let multiline_val_style = if is_focused {
                Style::default().bg(Color::Rgb(30, 41, 59)).fg(C_WHITE)
            } else {
                Style::default().fg(C_WHITE)
            };

            for (wi, wline) in wrapped.iter().enumerate() {
                let is_last = wi == last_idx;
                let display = if is_last && is_focused {
                    format!("    {}▌", wline)
                } else {
                    format!("    {}", wline)
                };
                lines.push(Line::from(vec![
                    Span::styled(display, multiline_val_style),
                ]));
            }
            continue;
        }

        let (display_val, placeholder) = match &field.kind {
            FieldKind::Bool => {
                let v = if field.value == "true" { "✓ Yes".to_string() } else { "○ No ".to_string() };
                (v, false)
            }
            FieldKind::Select(opts) => {
                let idx = field.value.parse::<usize>().unwrap_or(0);
                (opts.get(idx).cloned().unwrap_or_default(), false)
            }
            FieldKind::Date => {
                if field.value.is_empty() {
                    ("YYYY-MM-DD".to_string(), true)
                } else {
                    (field.value.clone(), false)
                }
            }
            _ => (field.value.clone(), false),
        };

        // Dim the value style for placeholder text
        let effective_val_style = if placeholder && !is_focused {
            Style::default().fg(Color::Rgb(51, 65, 85))
        } else {
            val_style
        };

        let cursor = if is_focused { "▌" } else { " " };

        let mut field_spans = vec![
            Span::styled(format!("  {:22}", field.label), lbl_style),
            Span::styled(format!("{}{}", display_val, cursor), effective_val_style),
        ];
        if is_focused && matches!(field.kind, FieldKind::Date) {
            field_spans.push(Span::styled("  ⏎ calendar", Style::default().fg(Color::Rgb(71, 85, 105))));
        }
        lines.push(Line::from(field_spans));
    }

    // Convert field-based scroll offset to line-based scroll offset
    let line_scroll = field_line_starts
        .get(edit_scroll as usize)
        .copied()
        .unwrap_or(0) as u16;

    f.render_widget(
        Paragraph::new(lines)
            .scroll((line_scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_YELLOW))
                    .title(Span::styled(title, Style::default().fg(C_YELLOW))),
            ),
        area,
    );
}

// ── Close pane ────────────────────────────────────────────────────────────────

fn draw_close_pane(
    f:              &mut Frame,
    area:           Rect,
    fields:         &[EditField],
    focused_idx:    usize,
    context_trade:  Option<&Trade>,
) {
    let title = context_trade.map_or_else(
        || " Close Trade  (Ctrl+S:Save  Esc:Cancel) ".to_string(),
        |t| format!(
            " Close: {} [{}]  Cr:${:.2}  (Ctrl+S:Save  Esc:Cancel) ",
            t.ticker, t.strategy.badge(), t.credit_received
        ),
    );

    // Split: form above, P&L preview below
    let (form_area, preview_area) = if context_trade.is_some() && area.height > 10 {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(4)])
            .split(area);
        (v[0], Some(v[1]))
    } else {
        (area, None)
    };

    // Build form lines; skip blank spacer fields in rendering
    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        if let Some(hdr) = &field.section_header {
            if !hdr.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(hdr.clone(), Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD)),
                ]));
            }
        }
        if field.label.is_empty() {
            continue;
        }
        let is_focused = i == focused_idx;
        let display_val = match &field.kind {
            FieldKind::Select(opts) => {
                let idx = field.value.parse::<usize>().unwrap_or(0);
                opts.get(idx).cloned().unwrap_or_default()
            }
            _ => field.value.clone(),
        };
        let cursor = if is_focused { "▌" } else { " " };
        let (lbl_style, val_style) = if is_focused {
            (
                Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
                Style::default().bg(Color::Rgb(30, 58, 138)).fg(C_WHITE),
            )
        } else {
            (Style::default().fg(C_GRAY), Style::default().fg(C_WHITE))
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {:22}", field.label), lbl_style),
            Span::styled(format!("{}{}", display_val, cursor), val_style),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_GREEN))
                .title(Span::styled(title, Style::default().fg(C_GREEN))),
        ),
        form_area,
    );

    // ── P&L preview strip
    if let (Some(pv), Some(trade)) = (preview_area, context_trade) {
        let mut close_short = 0.0_f64;
        let mut close_long  = 0.0_f64;
        for leg in &trade.legs {
            let key = match leg.leg_type {
                LegType::ShortPut  => "  BTC Short Put",
                LegType::LongPut   => "  STC Long Put",
                LegType::ShortCall => "  BTC Short Call",
                LegType::LongCall  => "  STC Long Call",
            };
            let cp: f64 = fields.iter()
                .find(|f| f.label == key)
                .and_then(|f| f.value.parse().ok())
                .unwrap_or(0.0);
            if leg.leg_type.is_short() { close_short += cp; } else { close_long += cp; }
        }
        let debit   = close_short - close_long;
        let gross   = (trade.credit_received - debit) * 100.0 * trade.quantity as f64;
        let comm: f64 = fields.iter()
            .find(|f| f.label == "Close Commission")
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(0.0);
        let net    = gross - comm;
        let max_p  = calculate_max_profit(trade.credit_received, trade.quantity);
        let pct    = if max_p > 0.0 { (net / max_p) * 100.0 } else { 0.0 };
        let pnl_c  = if net >= 0.0 { C_GREEN } else { C_RED };

        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("  Est. P&L: ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("${:+.2}", net), Style::default().fg(pnl_c).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  ({:.1}% of max)", pct), Style::default().fg(C_GRAY)),
                    Span::styled(format!("   Debit: ${:.4}", debit), Style::default().fg(C_WHITE)),
                ]),
                Line::from(vec![
                    Span::styled("  Credit: ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("${:.2}", trade.credit_received), Style::default().fg(C_CYAN)),
                    Span::styled(format!("   Qty: {}  Comm: ${:.2}", trade.quantity, comm), Style::default().fg(C_GRAY)),
                ]),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_GRAY))
                    .title(Span::styled(" P&L Preview ", Style::default().fg(C_GRAY))),
            ),
            pv,
        );
    }
}

// ── Date picker overlay ───────────────────────────────────────────────────────

const MONTH_NAMES: [&str; 12] = [
    "January","February","March","April","May","June",
    "July","August","September","October","November","December",
];

fn days_in_month(year: i32, month: u32) -> u32 {
    use chrono::NaiveDate;
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1))
        .and_then(|d| d.pred_opt())
        .map(|d| { use chrono::Datelike; d.day() })
        .unwrap_or(28)
}

fn draw_date_picker(f: &mut Frame, area: Rect, year: i32, month: u32, day: u32) {
    use chrono::{Datelike, NaiveDate, Utc};

    let w: u16 = 34;
    let h: u16 = 13;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let dialog = Rect::new(x, y, w.min(area.width), h.min(area.height));

    f.render_widget(Clear, dialog);

    let today    = Utc::now().date_naive();
    let month_nm = MONTH_NAMES.get((month as usize).saturating_sub(1)).unwrap_or(&"");
    let dim      = days_in_month(year, month);

    // Day of week for the 1st of the month (0=Mon … 6=Sun)
    let first_wd = NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| d.weekday().num_days_from_monday())
        .unwrap_or(0) as u32;

    let mut lines: Vec<Line> = Vec::new();

    // ── Month / year header
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("  ◀  {:^14}  ▶", format!("{} {}", month_nm, year)),
            Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    // ── Day-of-week header
    lines.push(Line::from(vec![
        Span::styled("  Mo  Tu  We  Th  Fr  Sa  Su", Style::default().fg(C_GRAY)),
    ]));

    // ── Calendar grid — build rows of 7 days
    let mut col: u32 = first_wd; // offset of first day
    let mut row_spans: Vec<Span> = vec![Span::raw("  ")];
    // fill leading blanks
    for _ in 0..first_wd {
        row_spans.push(Span::raw("    "));
    }
    for d in 1..=dim {
        let cur = NaiveDate::from_ymd_opt(year, month, d);
        let is_selected = d == day;
        let is_today    = cur.map(|c| c == today).unwrap_or(false);

        let txt = if is_selected {
            format!("[{:2}]", d)
        } else {
            format!(" {:2} ", d)
        };
        let style = if is_selected {
            Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)
        } else if is_today {
            Style::default().fg(C_YELLOW).add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(C_WHITE)
        };
        row_spans.push(Span::styled(txt, style));
        col += 1;
        if col == 7 {
            lines.push(Line::from(std::mem::replace(&mut row_spans, vec![Span::raw("  ")])));
            col = 0;
        }
    }
    // flush any remaining partial row
    if col > 0 {
        lines.push(Line::from(row_spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ←→:Day  ↑↓:Week  [/]:Month", Style::default().fg(C_GRAY)),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_BLUE))
                    .title(Span::styled(
                        " 📅 Date  Enter:OK  Esc:Cancel ",
                        Style::default().fg(C_CYAN),
                    )),
            ),
        dialog,
    );
}

// ── Confirm delete overlay ────────────────────────────────────────────────────

fn draw_confirm_delete(
    f:          &mut Frame,
    area:       Rect,
    delete_id:  Option<i32>,
    all_trades: &[Trade],
) {
    let trade_desc = delete_id
        .and_then(|id| all_trades.iter().find(|t| t.id == id))
        .map(|t| format!("{} [{}]", t.ticker, t.strategy.badge()))
        .unwrap_or_else(|| "this trade".to_string());

    let w: u16 = 56;
    let h: u16 = 7;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let dialog = Rect::new(x, y, w.min(area.width), h.min(area.height));

    f.render_widget(Clear, dialog);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Delete ", Style::default().fg(C_GRAY)),
                Span::styled(trade_desc, Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
                Span::styled("?", Style::default().fg(C_GRAY)),
            ]),
            Line::from(vec![Span::styled("  This cannot be undone.", Style::default().fg(C_GRAY))]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [Y / Enter]", Style::default().fg(C_RED).add_modifier(Modifier::BOLD)),
                Span::styled(" Confirm    ", Style::default().fg(C_WHITE)),
                Span::styled("[Any key]", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
                Span::styled(" Cancel", Style::default().fg(C_WHITE)),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_RED))
                .title(Span::styled(
                    " ⚠  Confirm Delete ",
                    Style::default().fg(C_RED).add_modifier(Modifier::BOLD),
                )),
        ),
        dialog,
    );
}

// ── L14: Journal Note quick-entry popup ───────────────────────────────────────

fn draw_journal_note_popup(
    f:        &mut Frame,
    area:     Rect,
    buf:      &str,
    trade_id: Option<i32>,
    trades:   &[Trade],
) {
    let ticker = trade_id
        .and_then(|id| trades.iter().find(|t| t.id == id))
        .map(|t| format!("{} [{}]", t.ticker, t.strategy.badge()))
        .unwrap_or_else(|| "trade".to_string());

    let w: u16 = 64.min(area.width.saturating_sub(4));
    let h: u16 = 8;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let dialog = Rect::new(x, y, w.min(area.width), h.min(area.height));

    // Cursor blink indicator
    let display = format!("{}▌", buf);

    f.render_widget(Clear, dialog);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  For: ", Style::default().fg(C_GRAY)),
                Span::styled(ticker, Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  > ", Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(display, Style::default().fg(C_WHITE)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [Enter]", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
                Span::styled(" Save    ", Style::default().fg(C_GRAY)),
                Span::styled("[Esc]", Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(" Cancel", Style::default().fg(C_GRAY)),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_CYAN))
                .title(Span::styled(
                    " 📝 Add Journal Note ",
                    Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD),
                )),
        ),
        dialog,
    );
}

// ── Trade detail pane ─────────────────────────────────────────────────────────

fn draw_trade_detail(f: &mut Frame, area: Rect, trade: &Trade, scroll: u16, chain: &[Trade], playbooks: &[crate::models::PlaybookStrategy], live_prices: &std::collections::HashMap<String, f64>, account_size: f64, max_pos_bpr_pct: f64) {
    let spread_type = trade.spread_type();
    let max_profit  = crate::calculations::calculate_max_profit_from_legs(&trade.legs, trade.credit_received, trade.quantity, spread_type);
    let max_loss    = calculate_max_loss_from_legs(&trade.legs, trade.credit_received, trade.quantity, spread_type);
    let breakevens  = calculate_breakevens(&trade.legs, spread_type, Some(trade.credit_received));
    let leg_desc    = format_trade_description(&trade.legs, spread_type);
    let dte         = calculate_remaining_dte(&trade.expiration_date);
    let roc         = trade.pnl.and_then(|p| calculate_roc(p, &trade.legs, trade.credit_received, trade.quantity, spread_type, trade.bpr, trade.underlying_price));
    let pct_max     = trade.pnl.map(|p| calculate_pct_max_profit(p, trade.credit_received, trade.quantity));
    let pnl_per_day = calculate_pnl_per_day(trade.pnl, &trade.entry_date, trade.exit_date.as_ref());
    let be_str      = if breakevens.is_empty() {
        "—".to_string()
    } else {
        breakevens.iter().map(|b| format!("{:.2}", b)).collect::<Vec<_>>().join(" / ")
    };

    let p50 = crate::calculations::calculate_p50(trade);
    let sep_style = Style::default().fg(C_GRAY);
    let mut left_lines: Vec<Line> = Vec::new();
    let mut right_lines: Vec<Line> = Vec::new();

    // ── Header
    left_lines.push(Line::from(vec![
        Span::styled(format!(" {} ", trade.ticker), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
        Span::styled(format!("[{}]", trade.strategy.badge()), Style::default().fg(badge_color(spread_type))),
        Span::styled(format!("  {} ", trade.strategy.label()), Style::default().fg(C_GRAY)),
        Span::styled(format!("  Qty: {}", trade.quantity), Style::default().fg(C_GRAY)),
        Span::styled(
            if trade.is_open() { "  ● OPEN" } else { "  ✓ CLOSED" },
            Style::default().fg(if trade.is_open() { C_YELLOW } else { C_GRAY }),
        ),
    ]));
    left_lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(leg_desc, Style::default().fg(C_CYAN)),
    ]));
    left_lines.push(Line::from(""));

    // ── ENTRY block ───────────────────────────────────────────────────────────
    left_lines.push(Line::from(vec![Span::styled(
        format!("  ── ENTRY {}", "─".repeat(58)),
        sep_style,
    )]));

    // Date / Exp / DTE@entry
    left_lines.push(Line::from(vec![
        Span::styled("  Date: ", Style::default().fg(C_GRAY)),
        Span::styled(trade.entry_date.format("%Y-%m-%d").to_string(), Style::default().fg(C_WHITE)),
        Span::styled("   Exp: ", Style::default().fg(C_GRAY)),
        Span::styled(
            trade.expiration_date.format("%Y-%m-%d").to_string(),
            Style::default().fg(if dte <= 14 { C_RED } else if dte <= 21 { C_YELLOW } else { C_WHITE }),
        ),
        Span::styled(
            trade.entry_dte.map_or(String::new(), |de| format!("   DTE@entry: {}", de)),
            Style::default().fg(C_GRAY),
        ),
        Span::styled(
            if trade.is_open() { format!("   ({} remaining)", dte) } else { String::new() },
            Style::default().fg(C_GRAY),
        ),
    ]));

    // Manage by date (open trades) — expiration - 21 days
    if trade.is_open() {
        let mgmt_date = trade.expiration_date.date_naive() - chrono::Duration::days(21);
        let days_to_mgmt = (mgmt_date - Utc::now().date_naive()).num_days();
        let (mgmt_color, mgmt_note) = if days_to_mgmt < 0 {
            (C_RED, format!(" ({}d past)", days_to_mgmt.abs()))
        } else if days_to_mgmt <= 5 {
            (C_YELLOW, format!(" ({}d away)", days_to_mgmt))
        } else {
            (C_GRAY, format!(" ({}d away)", days_to_mgmt))
        };
        left_lines.push(Line::from(vec![
            Span::styled("  Manage by: ", Style::default().fg(C_GRAY)),
            Span::styled(mgmt_date.format("%Y-%m-%d").to_string(), Style::default().fg(mgmt_color)),
            Span::styled(mgmt_note, Style::default().fg(mgmt_color)),
            Span::styled("  (21 DTE target)", Style::default().fg(Color::Rgb(100, 116, 139))),
        ]));
    }

    // Spot / IVR / VIX / IV
    {
        let mut cs: Vec<Span> = vec![Span::styled("  ", Style::default())];
        let spot = trade.underlying_price
            .map(|p| (format!("${:.2}  ", p), C_WHITE))
            .or_else(|| live_prices.get(&trade.ticker).map(|&p| (format!("${:.2}*  ", p), C_GRAY)));
        if let Some((spot_str, spot_color)) = spot {
            cs.push(Span::styled("Spot: ", Style::default().fg(C_GRAY)));
            cs.push(Span::styled(spot_str, Style::default().fg(spot_color)));
        }
        if let Some(ivr) = trade.iv_rank {
            let ic = if ivr >= 50.0 { C_GREEN } else if ivr >= 30.0 { C_YELLOW } else { C_GRAY };
            cs.push(Span::styled("IVR: ", Style::default().fg(C_GRAY)));
            cs.push(Span::styled(format!("{:.0}", ivr), Style::default().fg(ic)));
            if let Some(ivp) = trade.iv_percentile {
                let pc = if ivp >= 50.0 { C_GREEN } else if ivp >= 30.0 { C_YELLOW } else { C_GRAY };
                cs.push(Span::styled(" / ", Style::default().fg(C_GRAY)));
                cs.push(Span::styled(format!("{:.0}%ile", ivp), Style::default().fg(pc)));
            }
            cs.push(Span::raw("  "));
        }
        if let Some(vx) = trade.vix_at_entry {
            cs.push(Span::styled("VIX: ", Style::default().fg(C_GRAY)));
            cs.push(Span::styled(format!("{:.1}  ", vx), Style::default().fg(C_YELLOW)));
        }
        if let Some(iv) = trade.implied_volatility {
            cs.push(Span::styled("IV: ", Style::default().fg(C_GRAY)));
            cs.push(Span::styled(format!("{:.1}%", iv * 100.0), Style::default().fg(C_WHITE)));
        }
        if let Some(sec) = trade.sector.as_deref() {
            cs.push(Span::styled("   Sector: ", Style::default().fg(C_GRAY)));
            cs.push(Span::styled(sec.to_string(), Style::default().fg(C_WHITE)));
        }
        if cs.len() > 1 { left_lines.push(Line::from(cs)); }
    }

    // B/A spread + Fill vs Mid (Tastytrade fill quality)
    {
        let mut fq: Vec<Span> = vec![Span::styled("  ", Style::default())];
        if let Some(ba) = trade.bid_ask_spread_at_entry {
            fq.push(Span::styled("B/A: ", Style::default().fg(C_GRAY)));
            fq.push(Span::styled(format!("${:.2}  ", ba), Style::default().fg(C_WHITE)));
        }
        if let Some(fvm) = trade.fill_vs_mid {
            let fc = if fvm >= 0.0 { C_GREEN } else { C_RED };
            let label = if fvm >= 0.0 { "Fill vs Mid: +" } else { "Fill vs Mid: " };
            fq.push(Span::styled(label, Style::default().fg(C_GRAY)));
            fq.push(Span::styled(format!("${:.2}", fvm), Style::default().fg(fc)));
        }
        if fq.len() > 1 { left_lines.push(Line::from(fq)); }
    }

    // Greeks / POP / P50
    // Displayed in per-contract terms (×100): tastytrade convention
    {
        let mut gs: Vec<Span> = vec![Span::styled("  ", Style::default())];
        if let Some(d)  = trade.delta {
            gs.push(Span::styled(format!("Δ{:<.1}  ", d * 100.0), Style::default().fg(C_BLUE)));
        }
        if let Some(th) = trade.theta {
            gs.push(Span::styled(format!("Θ{:<.2}  ", th * 100.0), Style::default().fg(C_GREEN)));
        }
        if let Some(g)  = trade.gamma {
            gs.push(Span::styled(format!("Γ{:<.4}  ", g * 100.0), Style::default().fg(C_WHITE)));
        }
        if let Some(v)  = trade.vega  {
            gs.push(Span::styled(format!("V{:<.3}  ", v * 100.0), Style::default().fg(C_YELLOW)));
        }
        if let Some(pop) = trade.pop {
            let pc = if pop >= 70.0 { C_GREEN } else if pop >= 50.0 { C_YELLOW } else { C_RED };
            gs.push(Span::styled(format!("POP: {:.1}%  ", pop), Style::default().fg(pc)));
        }
        if let Some(p) = p50 {
            let p50c = if p >= 60.0 { C_GREEN } else if p >= 45.0 { C_YELLOW } else { C_RED };
            gs.push(Span::styled(format!("P50: {:.0}%", p), Style::default().fg(p50c)));
        }
        if gs.len() > 1 { left_lines.push(Line::from(gs)); }
    }

    // OTM% at entry (M7)
    {
        let otm_pct: Option<f64> = trade.underlying_price.and_then(|up| {
            if up <= 0.0 { return None; }
            match trade.strategy {
                StrategyType::CashSecuredPut | StrategyType::ShortPutVertical | StrategyType::IronCondor | StrategyType::IronButterfly => {
                    let k = trade.legs.iter().find(|l| l.leg_type == LegType::ShortPut).map(|l| l.strike).unwrap_or(trade.short_strike);
                    if k > 0.0 { Some((up - k) / up * 100.0) } else { None }
                }
                StrategyType::CoveredCall | StrategyType::ShortCallVertical => {
                    let k = trade.legs.iter().find(|l| l.leg_type == LegType::ShortCall).map(|l| l.strike).unwrap_or(trade.short_strike);
                    if k > 0.0 { Some((k - up) / up * 100.0) } else { None }
                }
                _ => None,
            }
        });
        if let Some(otm) = otm_pct {
            let otm_color = if otm >= 10.0 { C_GREEN } else if otm >= 5.0 { C_YELLOW } else { C_RED };
            left_lines.push(Line::from(vec![
                Span::styled("  OTM%: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:.1}%", otm), Style::default().fg(otm_color)),
            ]));
        }
    }

    // ── RISK / STRUCTURE block ────────────────────────────────────────────────
    left_lines.push(Line::from(""));
    left_lines.push(Line::from(vec![Span::styled(
        format!("  ── RISK / STRUCTURE {}", "─".repeat(47)),
        sep_style,
    )]));
    {
        let eff_width = trade.spread_width.or_else(|| {
            let w = compute_spread_width_from_legs(&trade.legs);
            if w > 0.0 { Some(w) } else { None }
        });
        let cw_ratio = if let Some(sw) = eff_width {
            if sw > 0.0 { Some(trade.credit_received / sw * 100.0) } else { None }
        } else { None };
        let mut r1: Vec<Span> = vec![
            Span::styled("  Credit: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:.2}", trade.credit_received), Style::default().fg(C_CYAN)),
        ];
        if let Some(sw) = eff_width {
            r1.push(Span::styled("   Width: ", Style::default().fg(C_GRAY)));
            r1.push(Span::styled(format!("${:.0}", sw), Style::default().fg(C_WHITE)));
        }
        if let Some(cw) = cw_ratio {
            let cw_color = if cw >= 33.0 { C_GREEN } else if cw >= 25.0 { C_YELLOW } else { C_RED };
            r1.push(Span::styled("   C/W: ", Style::default().fg(C_GRAY)));
            r1.push(Span::styled(format!("{:.1}%", cw), Style::default().fg(cw_color)));
        }
        left_lines.push(Line::from(r1));

        let mut r2: Vec<Span> = Vec::new();
        if let Some(b) = trade.bpr {
            let pct = if account_size > 0.0 { b / account_size * 100.0 } else { 0.0 };
            let pct_color = if pct > max_pos_bpr_pct { C_RED }
                            else if pct > max_pos_bpr_pct * 0.75 { C_YELLOW }
                            else { C_GREEN };
            r2.push(Span::styled("  BPR: ", Style::default().fg(C_GRAY)));
            r2.push(Span::styled(format!("${:.0}", b), Style::default().fg(C_YELLOW)));
            r2.push(Span::styled(format!(" ({:.1}%)", pct), Style::default().fg(pct_color)));
        }
        r2.push(Span::styled("   Max Profit: ", Style::default().fg(C_GRAY)));
        r2.push(Span::styled(
            if max_profit > 0.0 { format!("${:.0}", max_profit) } else { "\u{2014}".to_string() },
            if max_profit > 0.0 { C_GREEN } else { C_GRAY },
        ));
        r2.push(Span::styled("   Max Loss: ", Style::default().fg(C_GRAY)));
        r2.push(Span::styled(
            if max_loss > 0.0 {
                format!("${:.0}", max_loss)
            } else if spread_type == "put_broken_wing_butterfly" {
                "chk strikes".to_string()
            } else {
                "Undef".to_string()
            },
            if max_loss <= 0.0 && spread_type == "put_broken_wing_butterfly" { C_YELLOW } else { C_RED },
        ));
        if !r2.is_empty() { left_lines.push(Line::from(r2)); }

        if let Some(b) = trade.bpr {
            let pct = if account_size > 0.0 { b / account_size * 100.0 } else { 0.0 };
            let lo_dollar = account_size * 0.01;
            let hi_dollar = account_size * (max_pos_bpr_pct / 100.0);
            let near_max_threshold = max_pos_bpr_pct * 0.75;
            let status = if pct > max_pos_bpr_pct        { "✗ OVER" }
                         else if pct > near_max_threshold { "⚠ NEAR MAX" }
                         else if pct >= 1.0               { "✓ OK" }
                         else                             { "↑ SMALL" };
            let status_color = if pct > max_pos_bpr_pct        { C_RED }
                               else if pct > near_max_threshold { C_YELLOW }
                               else if pct >= 1.0               { C_GREEN }
                               else                             { C_GRAY };
            let mut sz: Vec<Span> = vec![Span::styled("  Size: ", Style::default().fg(C_GRAY))];
            sz.push(Span::styled(status, Style::default().fg(status_color)));
            sz.push(Span::styled(
                format!("   Target: 1–{:.0}%  ${:.0}–${:.0}/trade", max_pos_bpr_pct, lo_dollar, hi_dollar),
                Style::default().fg(C_WHITE),
            ));
            left_lines.push(Line::from(sz));
        }
    }

    if trade.is_open() {
        left_lines.push(Line::from(vec![
            Span::styled("  Breakeven: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{}", be_str), Style::default().fg(C_YELLOW)),
        ]));
    }

    if trade.is_open() && trade.credit_received > 0.0 {
        let targets = [(85u32, "85%"), (50, "50%"), (25, "25%")];
        let mut tgt: Vec<Span> = vec![Span::styled("  Targets: ", Style::default().fg(C_GRAY))];
        for (pct, label) in targets {
            let close_at = trade.credit_received * (1.0 - pct as f64 / 100.0);
            tgt.push(Span::styled(format!("{} ${:.2}  ", label, close_at), Style::default().fg(C_WHITE)));
        }
        left_lines.push(Line::from(tgt));
    }

    // Time to 50% profit estimate (open trades with theta)
    if trade.is_open() {
        if let Some(th) = trade.theta {
            let th_abs = th.abs();
            if th_abs > 0.0 && max_profit > 0.0 {
                let qty = trade.quantity as f64;
                let daily_theta = th_abs * 100.0 * qty;
                let days_held = (Utc::now().date_naive() - trade.trade_date.date_naive()).num_days().max(0) as f64;
                let est_pnl = daily_theta * days_held;
                let profit_target = max_profit * 0.5;
                if est_pnl >= profit_target {
                    left_lines.push(Line::from(vec![
                        Span::styled("  50% target: ", Style::default().fg(C_GRAY)),
                        Span::styled("\u{2605} Already reached (est.)", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    let days_rem = ((profit_target - est_pnl) / daily_theta).ceil() as i64;
                    if days_rem > dte as i64 {
                        // Theta alone won't reach 50% before expiration — needs delta/move
                        left_lines.push(Line::from(vec![
                            Span::styled("  50% via \u{03b8}: ", Style::default().fg(C_GRAY)),
                            Span::styled("needs move, not \u{03b8} pace", Style::default().fg(C_YELLOW)),
                        ]));
                    } else {
                        let target_date = Utc::now() + chrono::Duration::days(days_rem);
                        let color = if days_rem <= dte as i64 / 2 { C_GREEN } else { C_YELLOW };
                        left_lines.push(Line::from(vec![
                            Span::styled("  50% in ~", Style::default().fg(C_GRAY)),
                            Span::styled(format!("{}d", days_rem), Style::default().fg(color)),
                            Span::styled(format!("  (est. by {})", target_date.format("%m/%d")), Style::default().fg(C_GRAY)),
                            Span::styled("  \u{03b8} heuristic", Style::default().fg(Color::Rgb(100, 116, 139))),
                        ]));
                    }
                }
            }
        }
    }

    // ── EXIT block (closed trades only) ──────────────────────────────────────
    if let Some(exit) = trade.exit_date {
        let held = (exit.date_naive() - trade.trade_date.date_naive()).num_days();
        let pc = trade.pnl.map_or(C_GRAY, |p| if p >= 0.0 { C_GREEN } else { C_RED });

        left_lines.push(Line::from(""));
        left_lines.push(Line::from(vec![Span::styled(
            format!("  ── EXIT {}", "─".repeat(59)),
            sep_style,
        )]));

        // Exit date / reason / held / DTE@close
        left_lines.push(Line::from(vec![
            Span::styled("  Exit: ", Style::default().fg(C_GRAY)),
            Span::styled(exit.format("%Y-%m-%d").to_string(), Style::default().fg(C_WHITE)),
            Span::styled(
                trade.exit_reason.as_deref().map(|r| format!("  ({})", r)).unwrap_or_default(),
                Style::default().fg(C_GRAY),
            ),
            Span::styled(format!("   Held: {}d", held), Style::default().fg(C_GRAY)),
            if let Some(dc) = trade.dte_at_close {
                Span::styled(format!("   DTE@close: {}", dc), Style::default().fg(C_GRAY))
            } else { Span::raw("") },
        ]));

        // Debit / IV@close (crush) / Δ@close
        {
            let mut s2: Vec<Span> = vec![Span::styled("  ", Style::default())];
            if let Some(dp) = trade.debit_paid {
                s2.push(Span::styled("Debit: ", Style::default().fg(C_GRAY)));
                s2.push(Span::styled(format!("${:.4}  ", dp), Style::default().fg(C_WHITE)));
            }
            if let Some(iv) = trade.iv_at_close {
                s2.push(Span::styled("IV@close: ", Style::default().fg(C_GRAY)));
                s2.push(Span::styled(format!("{:.1}%", iv), Style::default().fg(C_CYAN)));
                if let Some(entry_iv) = trade.implied_volatility {
                    let crush = (entry_iv - iv) * 100.0;
                    let cc = if crush > 0.0 { C_GREEN } else { C_RED };
                    s2.push(Span::styled(format!(" (crush {:+.1}pp)  ", crush), Style::default().fg(cc)));
                } else { s2.push(Span::raw("  ")); }
            }
            if let Some(d) = trade.delta_at_close {
                s2.push(Span::styled("Δ@close: ", Style::default().fg(C_GRAY)));
                s2.push(Span::styled(format!("{:.2}", d), Style::default().fg(C_CYAN)));
            }
            if s2.len() > 1 { left_lines.push(Line::from(s2)); }
        }

        // Θ@close / Γ@close / V@close
        {
            let has_close_greeks = trade.theta_at_close.is_some() || trade.gamma_at_close.is_some() || trade.vega_at_close.is_some();
            if has_close_greeks {
                let mut sg: Vec<Span> = vec![Span::styled("  ", Style::default())];
                if let Some(th) = trade.theta_at_close { sg.push(Span::styled(format!("Θ@close: {:.4}  ", th), Style::default().fg(C_GREEN))); }
                if let Some(g)  = trade.gamma_at_close { sg.push(Span::styled(format!("Γ@close: {:.4}  ", g),  Style::default().fg(C_WHITE))); }
                if let Some(v)  = trade.vega_at_close  { sg.push(Span::styled(format!("V@close: {:.4}", v),   Style::default().fg(C_YELLOW))); }
                left_lines.push(Line::from(sg));
            }
        }

        // P&L / % of max / ROC / velocity
        if let Some(pnl) = trade.pnl {
            left_lines.push(Line::from(vec![
                Span::styled("  P&L: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("${:+.2}", pnl), Style::default().fg(pc).add_modifier(Modifier::BOLD)),
                Span::styled(
                    pct_max.map_or(String::new(), |p| format!("  ({:.1}% of max)", p)),
                    Style::default().fg(C_WHITE),
                ),
                Span::styled(
                    roc.map_or(String::new(), |r| format!("   ROC: {:.1}%", r)),
                    Style::default().fg(if roc.unwrap_or(0.0) > 0.0 { C_GREEN } else { C_RED }),
                ),
                Span::styled(
                    pnl_per_day.map_or(String::new(), |v| format!("   Vel: ${:.0}/d", v)),
                    Style::default().fg(pc),
                ),
            ]));
        }

        // Underlying @ close
        if let Some(uc) = trade.underlying_price_at_close {
            left_lines.push(Line::from(vec![
                Span::styled("  S@close: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("${:.2}", uc), Style::default().fg(C_WHITE)),
            ]));
        }
    }

    // Cache playbook lookup — used in both MANAGEMENT and PLAYBOOK COMPLIANCE blocks
    let trade_playbook: Option<&crate::models::PlaybookStrategy> = trade.playbook_id
        .and_then(|pid| playbooks.iter().find(|p| p.id == pid));

    // ── RIGHT COLUMN: MANAGEMENT ─────────────────────────────────────────────
    {
        let has_mgmt = trade.is_open()
            || trade.management_rule.is_some()
            || trade.target_profit_pct.is_some()
            || trade.trade_grade.is_some()
            || trade.commission.is_some()
            || trade.is_earnings_play
            || trade.is_tested
            || !trade.tags.is_empty()
            || trade.entry_reason.is_some()
            || trade.notes.as_ref().map(|n| !n.is_empty()).unwrap_or(false);

        if has_mgmt {
            right_lines.push(Line::from(vec![Span::styled(
                format!(" ── MANAGEMENT {}", "─".repeat(53)),
                sep_style,
            )]));

            // Rule / Target / Grade on one line
            {
                let mut m1: Vec<Span> = vec![Span::styled(" ", Style::default())];
                // Use trade's own rule; fall back to playbook rule if available
                let effective_rule: Option<String> = trade.management_rule.clone().or_else(|| {
                    trade_playbook
                        .and_then(|pb| pb.entry_criteria.as_ref())
                        .and_then(|ec| ec.management_rule.clone())
                });
                if let Some(m) = &effective_rule {
                    m1.push(Span::styled("Rule: ", Style::default().fg(C_GRAY)));
                    m1.push(Span::styled(format!("{}  ", m), Style::default().fg(C_CYAN)));
                }
                if let Some(tgt) = trade.target_profit_pct {
                    m1.push(Span::styled("Target: ", Style::default().fg(C_GRAY)));
                    m1.push(Span::styled(format!("{:.0}%  ", tgt), Style::default().fg(C_WHITE)));
                }
                if let Some(grade) = &trade.trade_grade {
                    let gc = match grade.as_str() {
                        "A" => C_GREEN,
                        "B" => Color::Rgb(132, 204, 22),
                        "C" => C_YELLOW,
                        "D" => Color::Rgb(249, 115, 22),
                        _   => C_RED,
                    };
                    m1.push(Span::styled("Grade: ", Style::default().fg(C_GRAY)));
                    m1.push(Span::styled(grade.clone(), Style::default().fg(gc).add_modifier(Modifier::BOLD)));
                    if let Some(gn) = &trade.grade_notes {
                        m1.push(Span::styled(format!("  — {}", gn), Style::default().fg(C_GRAY)));
                    }
                }
                if m1.len() > 1 { right_lines.push(Line::from(m1)); }
            }

            // Commission / BPR / Rolls / flags
            {
                let mut m2: Vec<Span> = vec![Span::styled(" ", Style::default())];
                if let Some(c)  = trade.commission  { m2.push(Span::styled(format!("Comm: ${:.2}  ", c),  Style::default().fg(C_GRAY))); }
                if let Some(b)  = trade.bpr         { m2.push(Span::styled(format!("BPR: ${:.0}  ", b),   Style::default().fg(C_YELLOW))); }
                if trade.roll_count > 0             { m2.push(Span::styled(format!("Rolls: {}  ", trade.roll_count), Style::default().fg(C_YELLOW))); }
                if trade.is_earnings_play           { m2.push(Span::styled("⚡ Earnings  ", Style::default().fg(C_YELLOW))); }
                if trade.is_tested                  { m2.push(Span::styled("⚠ Tested  ", Style::default().fg(C_RED))); }
                if m2.len() > 1 { right_lines.push(Line::from(m2)); }
            }

            // Execution quality
            {
                let mut eq: Vec<Span> = vec![Span::styled(" ", Style::default())];
                if let Some(ba) = trade.bid_ask_spread_at_entry {
                    eq.push(Span::styled("Bid-Ask: ", Style::default().fg(C_GRAY)));
                    eq.push(Span::styled(format!("${:.2}  ", ba), Style::default().fg(C_CYAN)));
                }
                if let Some(fvm) = trade.fill_vs_mid {
                    let fill_color = if fvm < 0.0 { C_RED } else { C_GREEN };
                    eq.push(Span::styled("Fill vs Mid: ", Style::default().fg(C_GRAY)));
                    eq.push(Span::styled(format!("{:+.2}  ", fvm), Style::default().fg(fill_color)));
                }
                if eq.len() > 1 { right_lines.push(Line::from(eq)); }
            }

            // Assignment info
            if trade.was_assigned {
                let mut as_line: Vec<Span> = vec![
                    Span::styled(" ASSIGNED  ", Style::default().fg(Color::White).bg(C_RED).add_modifier(Modifier::BOLD)),
                ];
                if let Some(sh) = trade.assigned_shares {
                    as_line.push(Span::styled(format!("  {} shares", sh), Style::default().fg(C_WHITE)));
                }
                if let Some(cb) = trade.cost_basis {
                    as_line.push(Span::styled(format!("  @ ${:.2}/sh", cb), Style::default().fg(C_YELLOW)));
                }
                right_lines.push(Line::from(as_line));
                right_lines.push(Line::from(vec![
                    Span::styled(" Assigned — stock P&L not included", Style::default().fg(C_YELLOW)),
                ]));
            }

            // Entry reason
            if let Some(er) = &trade.entry_reason {
                right_lines.push(Line::from(vec![
                    Span::styled(" Entry reason: ", Style::default().fg(C_GRAY)),
                    Span::styled(er.clone(), Style::default().fg(C_WHITE)),
                ]));
            }

            // Rolled from
            if let Some(rid) = trade.rolled_from_id {
                right_lines.push(Line::from(vec![
                    Span::styled(" ↩ Rolled from: ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("Trade #{}", rid), Style::default().fg(C_BLUE)),
                ]));
            }

            // Tags
            if !trade.tags.is_empty() {
                let mut ts = vec![Span::styled(" Tags: ", Style::default().fg(C_GRAY))];
                for tag in &trade.tags {
                    ts.push(Span::styled(format!("[{}] ", tag), Style::default().bg(C_DARK).fg(C_CYAN)));
                }
                right_lines.push(Line::from(ts));
            }

            // Notes
            if let Some(notes) = &trade.notes {
                if !notes.is_empty() {
                    right_lines.push(Line::from(vec![Span::styled(" Notes:", Style::default().fg(C_GRAY))]));
                    for ln in notes.split('\n') {
                        right_lines.push(Line::from(vec![
                            Span::styled(format!("   {}", ln), Style::default().fg(Color::Rgb(203, 213, 225))),
                        ]));
                    }
                }
            }
            // Exit Notes
            if let Some(cn) = &trade.close_notes {
                if !cn.is_empty() {
                    right_lines.push(Line::from(vec![Span::styled(" Exit Notes:", Style::default().fg(C_GRAY))]));
                    for ln in cn.split('\n') {
                        right_lines.push(Line::from(vec![
                            Span::styled(format!("   {}", ln), Style::default().fg(Color::Rgb(203, 213, 225))),
                        ]));
                    }
                }
            }
        }
    }

    // ── Compute active vs rolled legs (used by both Campaign and Legs blocks) ─
    let trade_is_open = trade.exit_date.is_none();
    let some_legs_closed = trade.legs.iter().any(|l| l.close_premium.is_some());
    let (active_legs, rolled_legs): (Vec<_>, Vec<_>) = if trade_is_open && some_legs_closed {
        trade.legs.iter().partition(|l| l.close_premium.is_none())
    } else {
        (trade.legs.iter().collect(), vec![])
    };

    // ── RIGHT COLUMN: Per-leg breakdown (first — always visible without scrolling) ──
    if !trade.legs.is_empty() {
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(vec![
            Span::styled(
                format!(" Legs ({}):", active_legs.len()),
                Style::default().fg(C_GRAY).add_modifier(Modifier::BOLD),
            ),
        ]));
        for leg in &active_legs {
            let lbl = match leg.leg_type {
                LegType::ShortPut  => "Short Put ",
                LegType::LongPut   => "Long Put  ",
                LegType::ShortCall => "Short Call",
                LegType::LongCall  => "Long Call ",
            };
            let close_str = leg.close_premium
                .map(|cp| format!("  BTC: ${:.4}", cp))
                .unwrap_or_default();
            let qty = leg.quantity.unwrap_or(1);
            let qty_str = if qty != 1 { format!(" ×{}", qty) } else { String::new() };
            let mut leg_line = vec![
                Span::styled(format!("   {} ", lbl), Style::default().fg(C_GRAY)),
                Span::styled(format!("${:.2}", leg.strike), Style::default().fg(C_WHITE)),
                Span::styled(qty_str, Style::default().fg(C_YELLOW)),
                Span::styled(format!("  prem: ${:.4}", leg.premium), Style::default().fg(C_CYAN)),
            ];
            if let Some(exp) = &leg.expiration_date {
                let clean_exp = exp.split('T').next().unwrap_or(exp);
                leg_line.push(Span::styled(format!("  exp: {}", clean_exp), Style::default().fg(C_GRAY)));
            }
            leg_line.push(Span::styled(close_str, Style::default().fg(C_RED)));
            right_lines.push(Line::from(leg_line));
        }
    }

    // ── RIGHT COLUMN: Campaign / Roll Chain ──────────────────────────────────
    if chain.len() > 1 {
        use crate::calculations::calculate_campaign_metrics;
        let cm = calculate_campaign_metrics(chain);
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(vec![
            Span::styled(" Campaign (Rolls: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{}", cm.roll_count), Style::default().fg(C_WHITE)),
            Span::styled("):   ", Style::default().fg(C_GRAY)),
            Span::styled("Net Credit: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:.2}   ", cm.net_credit), Style::default().fg(C_CYAN)),
            Span::styled("Realized P&L: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:+.2}", cm.total_pnl), Style::default().fg(if cm.total_pnl >= 0.0 { C_GREEN } else { C_RED })),
        ]));
        for (i, t) in chain.iter().enumerate() {
            let is_current = t.id == trade.id;
            let status_c = if is_current { C_YELLOW } else { C_GRAY };
            let pnl_c = t.pnl.map_or(C_GRAY, |p| if p >= 0.0 { C_GREEN } else { C_RED });
            right_lines.push(Line::from(vec![
                Span::styled(format!("   {}. ", i+1), Style::default().fg(C_GRAY)),
                Span::styled(format!("{} ", t.trade_date.format("%Y-%m-%d")), Style::default().fg(C_GRAY)),
                Span::styled(format!("{:<15}", t.strategy.badge()), Style::default().fg(status_c)),
                Span::styled(t.pnl.map_or("OPEN".to_string(), |p| format!("${:+.0}", p)), Style::default().fg(pnl_c)),
            ]));
        }

        // ── Rolled legs (inside Campaign) ────────────────────────────────────
        if !rolled_legs.is_empty() {
            right_lines.push(Line::from(vec![
                Span::styled("   Rolled legs:", Style::default().fg(C_GRAY).add_modifier(Modifier::DIM)),
            ]));
            for leg in &rolled_legs {
                let lbl = match leg.leg_type {
                    LegType::ShortPut  => "Short Put ",
                    LegType::LongPut   => "Long Put  ",
                    LegType::ShortCall => "Short Call",
                    LegType::LongCall  => "Long Call ",
                };
                let close_str = leg.close_premium
                    .map(|cp| format!("  BTC: ${:.4}", cp))
                    .unwrap_or_default();
                let dim = Style::default().fg(C_GRAY).add_modifier(Modifier::DIM);
                let mut leg_line = vec![
                    Span::styled(format!("     {} ", lbl), dim),
                    Span::styled(format!("${:.2}", leg.strike), dim),
                    Span::styled(format!("  prem: ${:.4}", leg.premium), dim),
                ];
                if let Some(exp) = &leg.expiration_date {
                    let clean_exp = exp.split('T').next().unwrap_or(exp);
                    leg_line.push(Span::styled(format!("  exp: {}", clean_exp), dim));
                }
                leg_line.push(Span::styled(close_str, dim));
                right_lines.push(Line::from(leg_line));
            }
        }
    }

    // ── RIGHT COLUMN: Playbook compliance ────────────────────────────────────
    if trade.is_open() {
        if let Some(pb) = trade_playbook {
            {
                right_lines.push(Line::from(""));
                right_lines.push(Line::from(vec![Span::styled(
                    format!(" ── PLAYBOOK: {} {}", pb.name, "─".repeat(40usize.saturating_sub(pb.name.len()))),
                    sep_style,
                )]));
                if let Some(ec) = &pb.entry_criteria {
                    let violations = crate::calculations::check_playbook_compliance(trade, ec);
                    if violations.is_empty() {
                        right_lines.push(Line::from(vec![
                            Span::styled(" Compliance: ", Style::default().fg(C_GRAY)),
                            Span::styled("✓ Compliant", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
                        ]));
                    } else {
                        right_lines.push(Line::from(vec![
                            Span::styled(" Compliance: ", Style::default().fg(C_GRAY)),
                            Span::styled(format!("⚠ {} violation(s):", violations.len()), Style::default().fg(C_RED).add_modifier(Modifier::BOLD)),
                        ]));
                        for v in &violations {
                            right_lines.push(Line::from(vec![
                                Span::styled(format!("   • {}: {} (actual: {})", v.field, v.rule, v.actual), Style::default().fg(C_RED)),
                            ]));
                        }
                    }

                    // Exit ladder — only render when at least one rule is set
                    let has_ladder = ec.dte_exit.is_some()
                        || ec.profit_target_pct.is_some()
                        || ec.stop_loss_pct.is_some()
                        || ec.management_rule.is_some()
                        || ec.when_to_avoid.is_some()
                        || ec.vix_min.is_some()
                        || ec.vix_max.is_some();
                    if has_ladder {
                        right_lines.push(Line::from(vec![Span::styled(
                            format!(" ── MANAGEMENT RULES {}", "─".repeat(47)),
                            sep_style,
                        )]));
                        if let Some(dte) = ec.dte_exit {
                            right_lines.push(Line::from(vec![
                                Span::styled("   Close at ", Style::default().fg(C_GRAY)),
                                Span::styled(format!("{} DTE", dte), Style::default().fg(C_YELLOW)),
                            ]));
                        }
                        if let Some(tgt) = ec.profit_target_pct {
                            right_lines.push(Line::from(vec![
                                Span::styled("   Take profit at ", Style::default().fg(C_GRAY)),
                                Span::styled(format!("{:.0}%", tgt), Style::default().fg(C_GREEN)),
                            ]));
                        }
                        if let Some(sl) = ec.stop_loss_pct {
                            right_lines.push(Line::from(vec![
                                Span::styled("   Stop loss at ", Style::default().fg(C_GRAY)),
                                Span::styled(format!("{:.0}% of credit", sl), Style::default().fg(C_RED)),
                            ]));
                        }
                        if let Some(rule) = &ec.management_rule {
                            right_lines.push(Line::from(vec![
                                Span::styled("   Rule: ", Style::default().fg(C_GRAY)),
                                Span::styled(rule.clone(), Style::default().fg(C_CYAN)),
                            ]));
                        }
                        if let Some(avoid) = &ec.when_to_avoid {
                            right_lines.push(Line::from(vec![
                                Span::styled("   Avoid: ", Style::default().fg(C_GRAY)),
                                Span::styled(avoid.clone(), Style::default().fg(Color::Rgb(249, 115, 22))),
                            ]));
                        }
                        if ec.vix_min.is_some() || ec.vix_max.is_some() {
                            let mut vix_spans = vec![Span::styled("   VIX gate: ", Style::default().fg(C_GRAY))];
                            if let Some(vmin) = ec.vix_min {
                                vix_spans.push(Span::styled(format!("≥{:.0}", vmin), Style::default().fg(C_WHITE)));
                            }
                            if ec.vix_min.is_some() && ec.vix_max.is_some() {
                                vix_spans.push(Span::styled(" – ", Style::default().fg(C_GRAY)));
                            }
                            if let Some(vmax) = ec.vix_max {
                                vix_spans.push(Span::styled(format!("≤{:.0}", vmax), Style::default().fg(C_WHITE)));
                            }
                            right_lines.push(Line::from(vix_spans));
                        }
                    }
                } else {
                    right_lines.push(Line::from(vec![
                        Span::styled(" Playbook: ", Style::default().fg(C_GRAY)),
                        Span::styled(&pb.name, Style::default().fg(C_CYAN)),
                        Span::styled(" (no criteria)", Style::default().fg(C_GRAY)),
                    ]));
                }
            }
        } else {
            // No playbook assigned — show minimal block so user knows it's available
            right_lines.push(Line::from(""));
            right_lines.push(Line::from(vec![Span::styled(
                format!(" ── PLAYBOOK {}", "─".repeat(55)),
                sep_style,
            )]));
            right_lines.push(Line::from(vec![
                Span::styled("   No playbook assigned", Style::default().fg(C_GRAY)),
            ]));
        }
    }

    // ── Render two-column layout ─────────────────────────────────────────────
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(C_BLUE))
        .title(Span::styled(
            " Trade Detail  (D:hide  ↑↓:scroll  ←→:trade) ",
            Style::default().fg(C_CYAN),
        ));
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    f.render_widget(
        Paragraph::new(left_lines).scroll((scroll, 0)),
        cols[0],
    );

    f.render_widget(
        Paragraph::new(right_lines)
            .scroll((scroll, 0))
            .block(Block::default().borders(Borders::LEFT).border_style(Style::default().fg(C_BLUE))),
        cols[1],
    );
}

/// Count how many lines the LEFT column of `draw_trade_detail` would push.
/// Covers: Header, ENTRY, RISK/STRUCTURE, EXIT.
pub fn count_detail_lines_left(
    trade:     &crate::models::Trade,
    _chain:    &[crate::models::Trade],
    _playbooks: &[crate::models::PlaybookStrategy],
) -> usize {
    let mut n = 0usize;

    // ── Header block: header + leg_desc + blank + ENTRY sep + date line
    n += 5;

    // Spot / IVR / VIX / IV line
    if trade.underlying_price.is_some() || trade.iv_rank.is_some()
        || trade.vix_at_entry.is_some() || trade.implied_volatility.is_some()
    {
        n += 1;
    }

    // Greeks / POP / P50 line
    if trade.delta.is_some() || trade.theta.is_some() || trade.gamma.is_some()
        || trade.vega.is_some() || trade.pop.is_some()
    {
        n += 1;
    }

    // OTM% line (strategy-dependent)
    if trade.underlying_price.map_or(false, |up| up > 0.0) {
        let has_otm = match trade.strategy {
            StrategyType::CashSecuredPut | StrategyType::ShortPutVertical
            | StrategyType::IronCondor | StrategyType::IronButterfly => {
                let k = trade.legs.iter().find(|l| l.leg_type == LegType::ShortPut)
                    .map(|l| l.strike).unwrap_or(trade.short_strike);
                k > 0.0
            }
            StrategyType::CoveredCall | StrategyType::ShortCallVertical => {
                let k = trade.legs.iter().find(|l| l.leg_type == LegType::ShortCall)
                    .map(|l| l.strike).unwrap_or(trade.short_strike);
                k > 0.0
            }
            _ => false,
        };
        if has_otm { n += 1; }
    }

    // B/A + Fill vs Mid line
    if trade.bid_ask_spread_at_entry.is_some() || trade.fill_vs_mid.is_some() {
        n += 1;
    }

    // ── RISK / STRUCTURE block: blank + separator + credit line + bpr/profit/loss line
    n += 4;
    if trade.bpr.is_some() { n += 1; } // BPR sizing line
    if trade.is_open() { n += 1; } // breakeven (open trades only)
    if trade.is_open() && trade.credit_received > 0.0 { n += 1; } // management targets

    // ── EXIT block (closed trades)
    if trade.exit_date.is_some() {
        n += 1; // blank
        n += 1; // EXIT separator
        n += 1; // exit date line
        // Debit / IV@close / Δ@close line
        if trade.debit_paid.is_some() || trade.iv_at_close.is_some() || trade.delta_at_close.is_some() {
            n += 1;
        }
        // Θ@close / Γ@close / V@close line
        if trade.theta_at_close.is_some() || trade.gamma_at_close.is_some() || trade.vega_at_close.is_some() {
            n += 1;
        }
        if trade.pnl.is_some() { n += 1; }                              // P&L line
        if trade.underlying_price_at_close.is_some() { n += 1; }        // S@close line
    }

    n
}

/// Count how many lines the RIGHT column of `draw_trade_detail` would push.
/// Covers: MANAGEMENT, Campaign/Roll Chain, Playbook compliance, Legs breakdown.
/// MUST mirror draw_trade_detail right-column rendering exactly.
pub fn count_detail_lines_right(
    trade:     &crate::models::Trade,
    chain:     &[crate::models::Trade],
    playbooks: &[crate::models::PlaybookStrategy],
) -> usize {
    let mut n = 0usize;

    // Cache playbook — mirrors renderer
    let trade_playbook: Option<&crate::models::PlaybookStrategy> = trade.playbook_id
        .and_then(|pid| playbooks.iter().find(|p| p.id == pid));

    // Compute active vs rolled legs — mirrors renderer partition
    let trade_is_open = trade.exit_date.is_none();
    let some_legs_closed = trade.legs.iter().any(|l| l.close_premium.is_some());
    let (active_legs, rolled_legs): (Vec<_>, Vec<_>) = if trade_is_open && some_legs_closed {
        trade.legs.iter().partition(|l| l.close_premium.is_none())
    } else {
        (trade.legs.iter().collect(), vec![])
    };

    // ── MANAGEMENT block — mirrors has_mgmt gate in renderer
    let has_mgmt = trade.is_open()
        || trade.management_rule.is_some()
        || trade.target_profit_pct.is_some()
        || trade.trade_grade.is_some()
        || trade.commission.is_some()
        || trade.is_earnings_play
        || trade.is_tested
        || !trade.tags.is_empty()
        || trade.entry_reason.is_some()
        || trade.notes.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
    if has_mgmt {
        n += 1; // MANAGEMENT separator
        // m1: effective_rule (own or playbook fallback) / target / grade
        let effective_rule = trade.management_rule.is_some() || trade_playbook
            .and_then(|pb| pb.entry_criteria.as_ref())
            .and_then(|ec| ec.management_rule.as_ref())
            .is_some();
        if effective_rule || trade.target_profit_pct.is_some() || trade.trade_grade.is_some() {
            n += 1;
        }
        // m2: comm / bpr / rolls / flags
        if trade.commission.is_some() || trade.bpr.is_some() || trade.roll_count > 0
            || trade.is_earnings_play || trade.is_tested
        {
            n += 1;
        }
        // execution quality line
        if trade.bid_ask_spread_at_entry.is_some() || trade.fill_vs_mid.is_some() {
            n += 1;
        }
        // assignment
        if trade.was_assigned { n += 1; }
        if trade.entry_reason.is_some() { n += 1; }
        if trade.rolled_from_id.is_some() { n += 1; }
        if !trade.tags.is_empty() { n += 1; }
        if let Some(ref notes) = trade.notes {
            if !notes.is_empty() {
                n += 1; // "Notes:" header
                n += notes.split('\n').count();
            }
        }
    }

    // ── LEGS block (rendered first in right column)
    if !trade.legs.is_empty() {
        n += 1; // blank
        n += 1; // "Legs (N):" header
        n += active_legs.len(); // only active (non-rolled) legs
    }

    // ── Campaign / Roll Chain
    if chain.len() > 1 {
        n += 1; // blank
        n += 1; // campaign summary line
        n += chain.len();
        // Rolled legs sub-section inside Campaign
        if !rolled_legs.is_empty() {
            n += 1; // "Rolled legs:" header
            n += rolled_legs.len();
        }
    }

    // ── PLAYBOOK block — mirrors renderer exactly
    if trade.is_open() {
        if let Some(pb) = trade_playbook {
            n += 1; // blank
            n += 1; // PLAYBOOK separator
            if let Some(ec) = &pb.entry_criteria {
                let violations = crate::calculations::check_playbook_compliance(trade, ec);
                n += 1; // compliance status line
                n += violations.len();
                // Exit ladder sub-block
                let has_ladder = ec.dte_exit.is_some()
                    || ec.profit_target_pct.is_some()
                    || ec.stop_loss_pct.is_some()
                    || ec.management_rule.is_some()
                    || ec.when_to_avoid.is_some()
                    || ec.vix_min.is_some()
                    || ec.vix_max.is_some();
                if has_ladder {
                    n += 1; // MANAGEMENT RULES separator
                    if ec.dte_exit.is_some()          { n += 1; }
                    if ec.profit_target_pct.is_some() { n += 1; }
                    if ec.stop_loss_pct.is_some()     { n += 1; }
                    if ec.management_rule.is_some()   { n += 1; }
                    if ec.when_to_avoid.is_some()     { n += 1; }
                    if ec.vix_min.is_some() || ec.vix_max.is_some() { n += 1; }
                }
            } else {
                n += 1; // "(no criteria)" line
            }
        } else {
            // "No playbook assigned" fallback
            n += 1; // blank
            n += 1; // PLAYBOOK separator
            n += 1; // "No playbook assigned"
        }
    }

    n
}

pub fn count_perf_overview_lines(
    stats: &crate::models::PortfolioStats,
    perf:  &crate::models::PerformanceStats,
    width: usize,
    collapsed: &[bool; 14],
) -> usize {
    perf_health_lines(stats, width, collapsed[0], false).len()
    + perf_returns_lines(stats, perf, width, collapsed[1], false).len()
    + 1  // trailing blank
}

pub fn count_perf_analytics_lines(
    stats: &crate::models::PortfolioStats,
    perf:  &crate::models::PerformanceStats,
    width: usize,
    collapsed: &[bool; 14],
    spy_monthly: &std::collections::HashMap<(i32, u32), f64>,
) -> usize {
    perf_advanced_lines(stats, width, collapsed[2], false).len()
    + perf_strategy_lines(perf, width, collapsed[4], false).len()
    + perf_ticker_lines(perf, width, collapsed[5], false).len()
    + perf_monthly_lines(perf, spy_monthly, width, collapsed[6], false, stats.account_size).len()
    + perf_ivr_lines(perf, width, collapsed[7], false).len()
    + perf_vix_lines(perf, width, collapsed[8], false).len()
    + perf_dte_lines(perf, width, collapsed[9], false).len()
    + perf_ivr_entry_lines(perf, width, collapsed[10], false).len()
    + perf_pnl_dist_lines(perf, width, collapsed[11], false).len()
    + perf_held_lines(perf, width, collapsed[12], false).len()
    + perf_commission_lines(perf, collapsed[13], false).len()
    + 1
}

/// Returns the scroll offset that positions the selected section header near the top of the
/// scrollable Paragraph for the given subtab + cursor position.
pub fn perf_header_scroll_for_cursor(
    cursor: usize,
    subtab: usize,
    collapsed: &[bool; 14],
    stats: &crate::models::PortfolioStats,
    perf: &crate::models::PerformanceStats,
    spy_monthly: &std::collections::HashMap<(i32, u32), f64>,
    width: usize,
) -> u16 {
    if subtab == 0 {
        match cursor {
            0 => 0,
            1 => perf_health_lines(stats, width, collapsed[0], false).len() as u16,
            _ => 0,
        }
    } else if subtab == 1 {
        // Charts sub-tab: only growth chart widget, no text scroll
        0
    } else {
        let advanced_len = perf_advanced_lines(stats, width, collapsed[2], false).len();
        let strategy_len = perf_strategy_lines(perf, width, collapsed[4], false).len();
        let ticker_len   = perf_ticker_lines(perf, width, collapsed[5], false).len();
        let monthly_len  = perf_monthly_lines(perf, spy_monthly, width, collapsed[6], false, stats.account_size).len();
        let ivr_len      = perf_ivr_lines(perf, width, collapsed[7], false).len();
        let vix_len      = perf_vix_lines(perf, width, collapsed[8], false).len();
        let dte_len      = perf_dte_lines(perf, width, collapsed[9], false).len();
        let ivr_entry_len = perf_ivr_entry_lines(perf, width, collapsed[10], false).len();
        let pnl_dist_len  = perf_pnl_dist_lines(perf, width, collapsed[11], false).len();
        let held_len      = perf_held_lines(perf, width, collapsed[12], false).len();
        let base = advanced_len + strategy_len + ticker_len + monthly_len + ivr_len;
        match cursor {
            0 => 0,
            1 => advanced_len as u16,
            2 => (advanced_len + strategy_len) as u16,
            3 => (advanced_len + strategy_len + ticker_len) as u16,
            4 => (advanced_len + strategy_len + ticker_len + monthly_len) as u16,
            5 => (base) as u16,
            6 => (base + vix_len) as u16,
            7 => (base + vix_len + dte_len) as u16,
            8 => (base + vix_len + dte_len + ivr_entry_len) as u16,
            9 => (base + vix_len + dte_len + ivr_entry_len + pnl_dist_len) as u16,
            10 => (base + vix_len + dte_len + ivr_entry_len + pnl_dist_len + held_len) as u16,
            _ => 0,
        }
    }
}

/// Count the total lines in the Risk Distribution panel (for scroll limit computation).
pub fn count_risk_lines(trades: &[crate::models::Trade], stats: &crate::models::PortfolioStats) -> usize {
    // Fixed lines: blank + Undefined(2) + Defined(2) + blank + Target + Drift + WinRate + Theta/day
    // + Theta/NetLiq + BWD + Theta/Delta + MaxDD + AvgROC = ~14 fixed lines
    // + Open Risk Summary: blank + up to 2 lines (def max loss + undef BPR) = ~3 extra
    // + Sector section: divider(blank+rule+header+blank=4) + per-sector lines + empty fallback
    let mut count: usize = 17; // 14 base + 3 for Open Risk Summary block
    // Build sector -> ticker count map to estimate wrapped ticker lines
    let mut sector_tickers: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for t in trades.iter().filter(|t| t.is_open()) {
        if let (Some(_), Some(sec)) = (t.bpr, t.sector.as_deref()) {
            // Each ticker is avg ~7 chars including ", " separator
            let ticker_ref: &str = t.ticker.as_str();
            sector_tickers.entry(sec).and_modify(|e| { let _ = ticker_ref; *e += 1; }).or_insert(1);
        }
    }
    count += 4; // divider block
    if sector_tickers.is_empty() {
        count += 1;
    } else {
        const ASSUMED_TICKER_WIDTH: usize = 40; // conservative panel inner width estimate
        const AVG_TICKER_CHARS: usize = 7;      // avg chars per "TICK, " token
        for ticker_count in sector_tickers.values() {
            let ticker_lines = (ticker_count * AVG_TICKER_CHARS).div_ceil(ASSUMED_TICKER_WIDTH).max(1);
            count += 1 + ticker_lines + 1; // header + wrapped ticker lines + bar
        }
    }
    let _ = stats;
    count
}

pub fn count_thesis_lines(text: &str, inner_width: usize) -> usize {
    let wrapped = word_wrap(text, inner_width).len();
    // Header lines are rendered as "\n{ln}" — the \n adds 1 extra blank visual line each
    let header_extras = text.split('\n').filter(|ln| {
        let tr = ln.trim();
        tr.ends_with(':') || (tr.starts_with(|c: char| c.is_uppercase()) && tr.len() > 40)
    }).count();
    wrapped + header_extras
}

// ── Playbook ──────────────────────────────────────────────────────────────────

fn draw_playbook(
    f:                       &mut Frame,
    area:                    Rect,
    playbooks:               &[PlaybookStrategy],
    state:                   &mut ListState,
    thesis_scroll:           u16,
    app_mode:                AppMode,
    playbook_edit_fields:    &[EditField],
    playbook_edit_field_idx: usize,
    playbook_edit_scroll:    u16,
    thesis_edit_buf:         &str,
    perf_stats:              &crate::models::PerformanceStats,
    playbook_analytics:      &[crate::models::PlaybookAnalytics],
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(area);

    let items: Vec<ListItem> = playbooks.iter().map(|pb| {
        let badge = pb.spread_type.as_deref()
            .map(|st| format!("[{}]", crate::models::StrategyType::from_str(st).badge()))
            .unwrap_or_else(|| "    ".to_string());
        let name_line = Line::from(vec![
            Span::styled(
                format!(" {} ", badge),
                Style::default().fg(pb.spread_type.as_deref().map(badge_color).unwrap_or(C_GRAY)),
            ),
            Span::styled(pb.name.clone(), Style::default().fg(C_WHITE)),
        ]);
        // Stats line: win rate, P&L, count from strategy_breakdown
        let stats_line = if let Some(st) = pb.spread_type.as_deref() {
            if let Some(sb) = perf_stats.strategy_breakdown.iter().find(|sb| sb.strategy.as_str() == st) {
                let pnl_color = if sb.total_pnl >= 0.0 { C_GREEN } else { C_RED };
                let wr_color  = if sb.win_rate >= 65.0 { C_GREEN } else if sb.win_rate >= 50.0 { C_YELLOW } else { C_RED };
                Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(format!("{:.0}%wr ", sb.win_rate), Style::default().fg(wr_color)),
                    Span::styled(format!("{:+.0} ", sb.total_pnl), Style::default().fg(pnl_color)),
                    Span::styled(format!("({})", sb.trades), Style::default().fg(C_GRAY)),
                ])
            } else {
                Line::from(vec![Span::styled("  no data", Style::default().fg(C_GRAY))])
            }
        } else {
            Line::from(vec![Span::styled("  no data", Style::default().fg(C_DARK))])
        };
        ListItem::new(vec![name_line, stats_line])
    }).collect();

    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
                .title(Span::styled(" Strategies ", Style::default().fg(C_CYAN))))
            .highlight_style(
                Style::default().bg(Color::Rgb(30, 58, 138)).fg(C_WHITE).add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ "),
        chunks[0],
        state,
    );

    if app_mode == AppMode::EditThesis {
        let line_count = thesis_edit_buf.split('\n').count();
        // Scroll so the last line (cursor) is always visible
        let inner_h = chunks[1].height.saturating_sub(2) as usize;
        let scroll = if line_count > inner_h { (line_count - inner_h) as u16 } else { 0 };
        let lines: Vec<Line> = thesis_edit_buf
            .split('\n')
            .enumerate()
            .map(|(i, ln)| {
                if i == line_count - 1 {
                    Line::from(vec![Span::styled(
                        format!("{}▌", ln),
                        Style::default().fg(C_WHITE),
                    )])
                } else {
                    Line::from(Span::styled(ln, Style::default().fg(C_WHITE)))
                }
            })
            .collect();
        f.render_widget(
            Paragraph::new(lines)
                .scroll((scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(C_YELLOW))
                        .title(Span::styled(
                            " Thesis  (Ctrl+S: Save   Esc: Cancel) ",
                            Style::default().fg(C_YELLOW),
                        )),
                ),
            chunks[1],
        );
    } else if app_mode == AppMode::EditPlaybook {
        draw_edit_pane(f, chunks[1], playbook_edit_fields, playbook_edit_field_idx, playbook_edit_scroll, None);
    } else if let Some(idx) = state.selected() {
        if let Some(pb) = playbooks.get(idx) {
            let dc = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Length(12), Constraint::Min(0)])
                .split(chunks[1]);

            if let Some(ec) = &pb.entry_criteria {
                let mut bs: Vec<Span> = Vec::new();
                if ec.min_ivr.is_some() || ec.max_ivr.is_some() {
                    bs.push(Span::styled(format!(" IVR {}–{} ", ec.min_ivr.unwrap_or(0.0), ec.max_ivr.unwrap_or(100.0)), Style::default().bg(C_DARK).fg(C_WHITE)));
                    bs.push(Span::raw("  "));
                }
                if ec.min_delta.is_some() || ec.max_delta.is_some() {
                    bs.push(Span::styled(format!(" Δ {}–{} ", ec.min_delta.unwrap_or(0.0), ec.max_delta.unwrap_or(50.0)), Style::default().bg(C_DARK).fg(C_CYAN)));
                    bs.push(Span::raw("  "));
                }
                if ec.min_dte.is_some() || ec.max_dte.is_some() {
                    bs.push(Span::styled(format!(" {}–{} DTE ", ec.min_dte.unwrap_or(0), ec.max_dte.unwrap_or(0)), Style::default().bg(C_DARK).fg(C_YELLOW)));
                    bs.push(Span::raw("  "));
                }
                if let Some(a) = ec.max_allocation_pct {
                    bs.push(Span::styled(format!(" ≤{}% alloc ", a), Style::default().bg(C_DARK).fg(Color::Magenta)));
                    bs.push(Span::raw("  "));
                }
                if let Some(t) = ec.target_profit_pct {
                    bs.push(Span::styled(format!(" {}% tgt ", t), Style::default().bg(C_DARK).fg(C_GREEN)));
                    bs.push(Span::raw("  "));
                }
                if let Some(rule) = &ec.management_rule {
                    let lbl = match rule.as_str() {
                        "dte_exit_21"      => "21 DTE Exit",
                        "dte_exit_14"      => "14 DTE Exit",
                        "profit_target_50" => "50% Profit Tgt",
                        "profit_target_25" => "25% Profit Tgt",
                        "expiration"       => "Hold to Expiry",
                        "stop_loss"        => "Stop Loss",
                        _                  => rule.as_str(),
                    };
                    bs.push(Span::styled(format!(" {} ", lbl), Style::default().bg(C_BLUE).fg(C_WHITE)));
                }

                // Second line: new tastytrade-focused criteria
                let mut bs2: Vec<Span> = Vec::new();
                if let Some(p) = ec.min_pop {
                    bs2.push(Span::styled(format!(" POP ≥{:.0}% ", p), Style::default().bg(C_DARK).fg(C_GREEN)));
                    bs2.push(Span::raw("  "));
                }
                if ec.vix_min.is_some() || ec.vix_max.is_some() {
                    let v_lo = ec.vix_min.map(|v| format!("{:.0}", v)).unwrap_or("—".to_string());
                    let v_hi = ec.vix_max.map(|v| format!("{:.0}", v)).unwrap_or("—".to_string());
                    bs2.push(Span::styled(format!(" VIX {}–{} ", v_lo, v_hi), Style::default().bg(C_DARK).fg(C_YELLOW)));
                    bs2.push(Span::raw("  "));
                }
                if let Some(b) = ec.max_bpr_pct {
                    bs2.push(Span::styled(format!(" BPR ≤{:.1}% ", b), Style::default().bg(C_DARK).fg(Color::Magenta)));
                }

                let mut pg_lines: Vec<Line> = vec![Line::from(""), Line::from(bs)];
                if !bs2.is_empty() {
                    pg_lines.push(Line::from(bs2));
                }
                if let Some(n) = &ec.notes {
                    pg_lines.push(Line::from(""));
                    pg_lines.push(Line::from(vec![
                        Span::styled("  ⚠ ", Style::default().fg(C_YELLOW)),
                        Span::styled(n.clone(), Style::default().fg(C_GRAY)),
                    ]));
                }

                f.render_widget(
                    Paragraph::new(pg_lines)
                        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
                            .title(Span::styled(format!(" {} — Entry Checklist ", pb.name), Style::default().fg(C_CYAN)))),
                    dc[0],
                );
            } else {
                f.render_widget(
                    Paragraph::new(" No entry criteria defined.")
                        .style(Style::default().fg(C_GRAY))
                        .block(Block::default().borders(Borders::ALL)
                            .title(Span::styled(format!(" {} ", pb.name), Style::default().fg(C_CYAN)))),
                    dc[0],
                );
            }

            // ── Strategy stats panel (L10: matched vs unmatched win rate; L11: top violations)
            {
                let sb = perf_stats.strategy_breakdown.iter()
                    .find(|sb| sb.strategy.as_str() == pb.spread_type.as_deref().unwrap_or(""));
                let pa = playbook_analytics.iter().find(|a| a.playbook_id == pb.id);
                let iw = dc[1].width.saturating_sub(2) as usize;

                let mut stats_lines: Vec<Line> = Vec::new();

                // Row 1-4: existing stats
                if let Some(sb) = sb {
                    let pnl_color = if sb.total_pnl >= 0.0 { C_GREEN } else { C_RED };
                    let roc_color = if sb.avg_roc  >= 0.0 { C_GREEN } else { C_RED };
                    stats_lines.push(stat_row("Win Rate:", &format!("{:.0}%", sb.win_rate),         C_WHITE,   iw));
                    stats_lines.push(stat_row("Avg R:R:",  &format!("1:{:.1}", sb.avg_roc / 100.0), roc_color, iw));
                    stats_lines.push(stat_row("Usage:",    &format!("{} trades", sb.trades),        C_WHITE,   iw));
                    stats_lines.push(stat_row("Total P&L:",&format!("${:.0}", sb.total_pnl),        pnl_color, iw));
                } else {
                    stats_lines.push(stat_row("Win Rate:", "—",        C_GRAY, iw));
                    stats_lines.push(stat_row("Avg R:R:",  "—",        C_GRAY, iw));
                    stats_lines.push(stat_row("Usage:",    "0 trades", C_GRAY, iw));
                    stats_lines.push(stat_row("Total P&L:","—",        C_GRAY, iw));
                }

                // L10: matched vs unmatched win rate comparison
                stats_lines.push(Line::from(vec![
                    Span::styled("─── Playbook Compliance Win Rate ".to_string(), Style::default().fg(C_CYAN)),
                ]));
                if let Some(pa) = pa {
                    let m_wr_color = if pa.matched_win_rate >= 65.0 { C_GREEN } else if pa.matched_win_rate >= 50.0 { C_YELLOW } else { C_RED };
                    let u_wr_color = if pa.unmatched_win_rate >= 65.0 { C_GREEN } else if pa.unmatched_win_rate >= 50.0 { C_YELLOW } else { C_RED };
                    if pa.matched_trades > 0 {
                        stats_lines.push(stat_row(
                            &format!("  ✓ Followed ({}):", pa.matched_trades),
                            &format!("{:.0}%", pa.matched_win_rate),
                            m_wr_color, iw,
                        ));
                    } else {
                        stats_lines.push(stat_row("  ✓ Followed:", "no trades", C_GRAY, iw));
                    }
                    if pa.unmatched_trades > 0 {
                        stats_lines.push(stat_row(
                            &format!("  ✗ Not linked ({}):", pa.unmatched_trades),
                            &format!("{:.0}%", pa.unmatched_win_rate),
                            u_wr_color, iw,
                        ));
                    } else {
                        stats_lines.push(stat_row("  ✗ Not linked:", "no data", C_GRAY, iw));
                    }

                    // L11: top violation
                    if !pa.top_violations.is_empty() {
                        stats_lines.push(Line::from(vec![
                            Span::styled("  ⚠ Top violation: ".to_string(), Style::default().fg(C_YELLOW)),
                            Span::styled(
                                format!("{} ({}×)", pa.top_violations[0].0, pa.top_violations[0].1),
                                Style::default().fg(C_WHITE),
                            ),
                        ]));
                    }
                } else {
                    stats_lines.push(stat_row("  ✓ Followed:", "—", C_GRAY, iw));
                    stats_lines.push(stat_row("  ✗ Not linked:", "—", C_GRAY, iw));
                }

                f.render_widget(
                    Paragraph::new(stats_lines)
                        .block(Block::default().borders(Borders::ALL)
                            .border_style(Style::default().fg(C_BLUE))),
                    dc[1],
                );
            }

            let desc = pb.description.as_deref().unwrap_or("No description provided.");
            let desc_lines: Vec<Line> = desc.split('\n').map(|ln| {
                let tr = ln.trim();
                if tr.ends_with(':') || (tr.starts_with(|c: char| c.is_uppercase()) && tr.len() > 40) {
                    Line::from(vec![Span::styled(
                        format!("\n{}", ln),
                        Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
                    )])
                } else {
                    Line::from(ln)
                }
            }).collect();

            f.render_widget(
                Paragraph::new(desc_lines)
                    .scroll((thesis_scroll, 0))
                    .wrap(Wrap { trim: false })
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
                        .title(Span::styled(" Thesis (↑/↓ scroll) ", Style::default().fg(C_CYAN)))),
                dc[2],
            );
        }
    } else {
        f.render_widget(
            Paragraph::new(" Select a strategy from the list.")
                .style(Style::default().fg(C_GRAY))
                .block(Block::default().borders(Borders::ALL)),
            chunks[1],
        );
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn stat_row(label: &str, value: &str, value_color: Color, inner_w: usize) -> Line<'static> {
    let pad = inner_w.saturating_sub(label.len() + value.len());
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(C_GRAY)),
        Span::raw(" ".repeat(pad)),
        Span::styled(value.to_string(), Style::default().fg(value_color)),
    ])
}

/// Word-wrap `text` to lines of at most `width` chars, respecting existing newlines.
fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut result = Vec::new();
    
    for line in text.split('\n') {
        if line.is_empty() {
            result.push(String::new());
            continue;
        }
        
        let mut current_line = String::new();
        for word in line.split_whitespace() {
            if current_line.is_empty() {
                current_line.push_str(word);
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                result.push(current_line);
                current_line = word.to_string();
            }
        }
        result.push(current_line);
    }
    result
}

// ── Column visibility picker popup (L9) ──────────────────────────────────────

fn draw_col_picker_popup(f: &mut Frame, area: Rect, col_visibility: &[bool; 22]) {
    const COL_NAMES: [&str; 22] = [
        "Date","Ticker","Spot","ER","Str","Qty","Credit","GTC","BE",
        "BPR","BPR%","MaxPft","P&L","ROC%","$V/d","DTE","Exit","Held","Status","OTM%","EM","Mgmt",
    ];
    // Keys: 1-9 for columns 0-8, a-l for columns 9-20, m for Mgmt (21)
    const KEY_LABELS: [&str; 22] = [
        "1","2","3","4","5","6","7","8","9",
        "a","b","c","d","e","f","g","h","i","j","k","l","m",
    ];

    let popup_w = 36u16;
    let popup_h = 27u16;
    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect { x, y, width: popup_w.min(area.width), height: popup_h.min(area.height) };

    f.render_widget(Clear, popup_area);

    let mut text_lines: Vec<Line> = vec![
        Line::from(vec![Span::styled("  Column Visibility", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("  Press key to toggle, v/Esc to close", Style::default().fg(Color::DarkGray))]),
        Line::from(""),
    ];

    for i in 0..22 {
        let check = if col_visibility[i] { "✓" } else { "✗" };
        let check_style = if col_visibility[i] {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };
        text_lines.push(Line::from(vec![
            Span::styled(format!("  [{}] ", KEY_LABELS[i]), Style::default().fg(Color::Yellow)),
            Span::styled(check.to_string(), check_style),
            Span::styled(format!(" {}", COL_NAMES[i]), Style::default().fg(Color::White)),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Columns [v] ");

    let para = Paragraph::new(text_lines).block(block);
    f.render_widget(para, popup_area);
}

// ── Analyze pane (payoff chart) ───────────────────────────────────────────────

fn draw_analyze_pane(f: &mut Frame, area: Rect, trade: &Trade) {
    let spread_type = trade.spread_type();

    // Strategies where at-expiration P&L is IV-dependent and cannot be charted statically.
    // Calendar spreads are charted via Black-Scholes estimation (see below).
    let is_fallback = matches!(
        spread_type,
        "pmcc" | "long_diagonal_spread" | "short_diagonal_spread"
    );

    let title = format!(
        " {} [{}] — Payoff at Expiration  (Esc:close) ",
        trade.ticker,
        trade.strategy.badge()
    );
    let badge_col = badge_color(spread_type);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(C_BLUE))
        .title(Span::styled(title, Style::default().fg(badge_col).add_modifier(Modifier::BOLD)));

    // Compute inner dimensions before consuming the block
    let inner_w = area.width.saturating_sub(2);
    let inner_h = area.height.saturating_sub(2);

    if inner_w < 20 || inner_h < 6 {
        f.render_widget(Paragraph::new("").block(block), area);
        return;
    }

    if is_fallback {
        let msg: Vec<Line<'static>> = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Payoff chart not available for this strategy.",
                Style::default().fg(C_YELLOW),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Calendar / diagonal spreads have IV-dependent P&L",
                Style::default().fg(C_GRAY),
            )),
            Line::from(Span::styled(
                "  at expiration and cannot be charted statically.",
                Style::default().fg(C_GRAY),
            )),
        ];
        f.render_widget(Paragraph::new(msg).block(block), area);
        return;
    }

    // ── Stats header ─────────────────────────────────────────────────────────
    // Helper: extract (remaining_dte_years, iv_decimal) for calendar spreads
    let calendar_params = || -> (f64, f64) {
        let remaining_dte = trade.back_month_expiration
            .map(|bme| {
                let secs = (bme.timestamp() - trade.expiration_date.timestamp()).max(0) as f64;
                secs / (86_400.0 * 365.25)
            })
            .unwrap_or(30.0 / 365.25);
        let iv = trade.implied_volatility
            .map(|v| if v > 2.0 { v / 100.0 } else { v })
            .unwrap_or(0.25);
        (remaining_dte, iv)
    };

    let max_profit = if spread_type == "calendar_spread" {
        // Peak P&L is at the strike (top of the tent shape); estimate via Black-Scholes
        let (rem_dte, iv) = calendar_params();
        let strike = trade.legs.iter().map(|l| l.strike).find(|&s| s > 0.0).unwrap_or(100.0);
        calculate_calendar_payoff_at_price(
            &trade.legs, trade.credit_received, strike, rem_dte, iv,
        ) * trade.quantity as f64
    } else {
        crate::calculations::calculate_max_profit_from_legs(&trade.legs, trade.credit_received, trade.quantity, spread_type)
    };
    let max_loss   = calculate_max_loss_from_legs(
        &trade.legs, trade.credit_received, trade.quantity, spread_type,
    );
    let breakevens = calculate_breakevens(&trade.legs, spread_type, Some(trade.credit_received));
    let pop        = trade.pop.unwrap_or_else(|| estimate_pop(trade));

    let profit_str = if max_profit > 0.0 { format!("+${:.0}", max_profit) } else { "\u{2014}".to_string() };
    let loss_str   = if max_loss > 0.0 {
        format!("-${:.0}", max_loss)
    } else if spread_type == "put_broken_wing_butterfly" {
        "chk strikes".to_string()
    } else {
        "—".to_string()
    };
    let be_str     = if breakevens.is_empty() {
        "—".to_string()
    } else {
        breakevens.iter().map(|b| format!("${:.2}", b)).collect::<Vec<_>>().join(" / ")
    };
    let underlying_str = trade.underlying_price
        .map(|p| format!("  Underlying: ${:.2}", p))
        .unwrap_or_default();

    // All spans use &'static str or owned String → Vec<Line<'static>> is valid
    let header_lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled("  Max Profit: ", Style::default().fg(C_GRAY)),
            Span::styled(profit_str, Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
            Span::styled("   Max Loss: ", Style::default().fg(C_GRAY)),
            Span::styled(loss_str, Style::default().fg(C_RED).add_modifier(Modifier::BOLD)),
            Span::styled("   POP: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.0}%", pop), Style::default().fg(C_CYAN)),
        ]),
        Line::from(vec![
            Span::styled("  Breakeven: ", Style::default().fg(C_GRAY)),
            Span::styled(be_str, Style::default().fg(C_YELLOW)),
            Span::styled(underlying_str, Style::default().fg(C_CYAN)),
        ]),
        Line::from(""),
    ];
    let header_height = header_lines.len() as u16;

    // ── X-axis label rows ─────────────────────────────────────────────────────
    const Y_AXIS_W: usize = 9;
    let plot_w = (inner_w as usize).saturating_sub(Y_AXIS_W).max(10);

    let legs = &trade.legs;
    let (lo, hi) = {
        let strikes: Vec<f64> = legs.iter().map(|l| l.strike).filter(|&s| s > 0.0).collect();
        if strikes.is_empty() {
            (0.0_f64, 100.0_f64)
        } else {
            let mn = strikes.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = strikes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            if (mx - mn).abs() < 0.01 { (mn * 0.80, mx * 1.20) } else { (mn * 0.88, mx * 1.12) }
        }
    };
    let price_range = (hi - lo).max(1.0);
    let price_to_col = |p: f64| -> usize {
        (((p - lo) / price_range) * (plot_w - 1) as f64).round() as usize
    };

    // Marker map: col -> (symbol &'static str, color)
    let mut label_map: std::collections::BTreeMap<usize, (&'static str, Color)> =
        std::collections::BTreeMap::new();
    // Also keep a separate price map for labels (same col -> price)
    let mut price_at_col: std::collections::BTreeMap<usize, f64> =
        std::collections::BTreeMap::new();

    for leg in legs {
        if leg.strike > 0.0 {
            let col = price_to_col(leg.strike).min(plot_w - 1);
            let (sym, color): (&'static str, Color) = match leg.leg_type {
                LegType::ShortPut   => ("SP", C_RED),
                LegType::LongPut    => ("LP", C_GREEN),
                LegType::ShortCall  => ("SC", C_RED),
                LegType::LongCall   => ("LC", C_GREEN),
            };
            label_map.entry(col).or_insert((sym, color));
            price_at_col.entry(col).or_insert(leg.strike);
        }
    }
    for &be in &breakevens {
        let col = price_to_col(be).min(plot_w - 1);
        label_map.entry(col).or_insert(("BE", C_YELLOW));
        price_at_col.entry(col).or_insert(be);
    }
    // L5: ±1SD expected move markers
    let em_info: Option<(f64, f64, f64)> = trade.underlying_price.and_then(|up| {
        let iv = trade.implied_volatility
            .map(|v| if v > 2.0 { v / 100.0 } else { v })
            .or_else(|| trade.iv_rank.map(|r| r / 100.0))?;
        let now = chrono::Utc::now();
        let days_left = (trade.expiration_date.signed_duration_since(now).num_seconds().max(0) as f64) / 86_400.0;
        if days_left < 1.0 || iv <= 0.0 { return None; }
        let em = up * iv * (days_left / 365.0).sqrt();
        Some((up - em, up + em, em))
    });
    if let Some((lo_1sd, hi_1sd, _)) = em_info {
        let col_lo = price_to_col(lo_1sd.max(lo)).min(plot_w - 1);
        let col_hi = price_to_col(hi_1sd.min(hi)).min(plot_w - 1);
        label_map.entry(col_lo).or_insert(("|", Color::Magenta));
        label_map.entry(col_hi).or_insert(("|", Color::Magenta));
        price_at_col.entry(col_lo).or_insert(lo_1sd);
        price_at_col.entry(col_hi).or_insert(hi_1sd);
    }
    if let Some(up) = trade.underlying_price {
        let col = price_to_col(up).min(plot_w - 1);
        label_map.insert(col, ("●", C_CYAN));
        price_at_col.insert(col, up);
    }

    // Shared y-axis padding used by both rows
    let y_pad = " ".repeat(Y_AXIS_W);

    // Unified x-axis layout: compute placement blocks so price and label stay vertically aligned.
    // One block per marker: width = max(price_len, label_len), both rows anchored to same block_start.
    // (block_start, block_width, price_str, price_pad_left, sym, label_pad_left, color)
    let mut blocks: Vec<(usize, usize, String, usize, &'static str, usize, Color)> = Vec::new();
    let mut last_end: usize = 0;
    for (&col, &price) in &price_at_col {
        let (sym, color) = label_map.get(&col).copied().unwrap_or(("?", C_GRAY));
        let price_str = format!("{:.0}", price);
        let p_len = price_str.len();
        let s_len = sym.chars().count(); // display columns, not bytes (handles ● etc.)
        let block_w = p_len.max(s_len);
        // Ideal block center = label's natural midpoint
        let ideal_center = col as i64 + s_len as i64 / 2;
        let ideal_start = (ideal_center - block_w as i64 / 2).max(0) as usize;
        let block_start = ideal_start.max(last_end);
        if block_start >= plot_w { continue; }
        let price_pad = (block_w - p_len) / 2;
        let label_pad = (block_w - s_len) / 2;
        last_end = block_start + block_w;
        blocks.push((block_start, block_w, price_str, price_pad, sym, label_pad, color));
    }

    // Price row — rendered from blocks
    let mut x1_spans: Vec<Span<'static>> = vec![Span::raw(y_pad.clone())];
    let mut cursor = 0usize;
    for &(start, bw, ref price_str, price_pad, _, _, color) in &blocks {
        if start > cursor { x1_spans.push(Span::raw(" ".repeat(start - cursor))); }
        if price_pad > 0  { x1_spans.push(Span::raw(" ".repeat(price_pad))); }
        let avail = (plot_w.saturating_sub(start + price_pad)).min(price_str.len());
        if avail > 0 {
            x1_spans.push(Span::styled(price_str[..avail].to_string(), Style::default().fg(color)));
        }
        let tail = bw.saturating_sub(price_pad + price_str.len());
        if tail > 0 { x1_spans.push(Span::raw(" ".repeat(tail))); }
        cursor = start + bw;
    }
    if cursor < plot_w { x1_spans.push(Span::raw(" ".repeat(plot_w - cursor))); }

    // Label row — rendered from same blocks
    let mut x2_spans: Vec<Span<'static>> = vec![Span::raw(y_pad.clone())];
    let mut cursor = 0usize;
    for &(start, bw, _, _, sym, label_pad, color) in &blocks {
        if start > cursor { x2_spans.push(Span::raw(" ".repeat(start - cursor))); }
        if label_pad > 0  { x2_spans.push(Span::raw(" ".repeat(label_pad))); }
        let s_cols = sym.chars().count(); // display width (not bytes — handles ● etc.)
        let avail_cols = plot_w.saturating_sub(start + label_pad);
        if avail_cols >= s_cols {
            // Only push the full symbol — never byte-slice multi-byte chars
            x2_spans.push(Span::styled(sym.to_string(), Style::default().fg(color)));
        }
        let tail = bw.saturating_sub(label_pad + s_cols);
        if tail > 0 { x2_spans.push(Span::raw(" ".repeat(tail))); }
        cursor = start + bw;
    }
    if cursor < plot_w { x2_spans.push(Span::raw(" ".repeat(plot_w - cursor))); }

    // ── Layout: split inner area vertically ──────────────────────────────────
    let inner = block.inner(area);
    f.render_widget(block, area);

    let x_rows: u16 = if em_info.is_some() { 4 } else { 2 };
    let chunks = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(4),
        Constraint::Length(x_rows),
    ]).split(inner);
    let (header_area, chart_area, xaxis_area) = (chunks[0], chunks[1], chunks[2]);

    // Render header
    f.render_widget(Paragraph::new(header_lines), header_area);

    // ── ASCII grid chart renderer ─────────────────────────────────────────────
    // Precompute calendar params once if needed
    let (cal_rem_dte, cal_iv) = if spread_type == "calendar_spread" {
        calendar_params()
    } else {
        (0.0, 0.0)
    };

    // Per-column payoff values (quantity-adjusted)
    let chart_cols = plot_w;
    let chart_rows_h = chart_area.height as usize;
    let col_denom = (chart_cols - 1).max(1) as f64;
    let payoffs: Vec<f64> = (0..chart_cols).map(|col| {
        let p = lo + (hi - lo) * col as f64 / col_denom;
        let raw = if spread_type == "calendar_spread" {
            calculate_calendar_payoff_at_price(&trade.legs, trade.credit_received, p, cal_rem_dte, cal_iv)
        } else {
            calculate_payoff_at_price(&trade.legs, trade.credit_received, p)
        };
        raw * trade.quantity as f64
    }).collect();

    // Y-axis scale
    let raw_y_max = max_profit.max(0.1);
    let raw_y_min = (if max_loss > 0.0 { -max_loss } else {
        payoffs.iter().cloned().fold(f64::INFINITY, f64::min)
    }).min(-0.1);
    let y_pad_amt = (raw_y_max - raw_y_min) * 0.10;
    let y_top = raw_y_max + y_pad_amt;
    let y_bot = raw_y_min - y_pad_amt;

    let pnl_to_row = |pnl: f64| -> usize {
        let frac = (pnl - y_bot) / (y_top - y_bot);
        let r = ((chart_rows_h as f64 - 1.0) * (1.0 - frac)).round() as i64;
        r.clamp(0, chart_rows_h as i64 - 1) as usize
    };

    let pnl_rows: Vec<usize> = payoffs.iter().map(|&p| pnl_to_row(p)).collect();
    let zero_row = pnl_to_row(0.0);
    let max_profit_row = pnl_to_row(max_profit);
    let max_loss_row: Option<usize> = if max_loss > 0.0 { Some(pnl_to_row(-max_loss)) } else { None };

    // Strike columns: col → color
    let mut strike_cols: std::collections::HashMap<usize, Color> = std::collections::HashMap::new();
    for leg in legs {
        if leg.strike > 0.0 {
            let col = price_to_col(leg.strike).min(chart_cols - 1);
            let color = match leg.leg_type {
                LegType::ShortPut | LegType::ShortCall => C_RED,
                LegType::LongPut  | LegType::LongCall  => C_GREEN,
            };
            strike_cols.entry(col).or_insert(color);
        }
    }

    // ±1σ columns
    let mut sd_cols: std::collections::HashSet<usize> = std::collections::HashSet::new();
    if let Some((lo_1sd, hi_1sd, _)) = em_info {
        sd_cols.insert(price_to_col(lo_1sd.max(lo)).min(chart_cols - 1));
        sd_cols.insert(price_to_col(hi_1sd.min(hi)).min(chart_cols - 1));
    }

    // Underlying spot column
    let spot_col: Option<usize> = trade.underlying_price
        .map(|up| price_to_col(up).min(chart_cols - 1));

    // Build grid rows
    let mut grid_lines: Vec<Line<'static>> = Vec::with_capacity(chart_rows_h);
    for row in 0..chart_rows_h {
        let mut spans: Vec<Span<'static>> = Vec::new();

        // Y-axis label (Y_AXIS_W = 9 chars)
        let (lbl, lbl_color) = if row == zero_row {
            ("    $0   ".to_string(), C_GRAY)
        } else if row == max_profit_row {
            (format!("{:>8} ", format!("+${:.0}", max_profit)), C_GREEN)
        } else if let Some(ml_row) = max_loss_row {
            if row == ml_row {
                (format!("{:>8} ", format!("-${:.0}", max_loss)), C_RED)
            } else {
                (" ".repeat(Y_AXIS_W), C_DARK)
            }
        } else {
            (" ".repeat(Y_AXIS_W), C_DARK)
        };
        spans.push(Span::styled(lbl, Style::default().fg(lbl_color)));

        // Grid cells
        for col in 0..chart_cols {
            let pnl = payoffs[col];
            let pnl_row = pnl_rows[col];
            let in_profit_bar = pnl > 0.0 && row >= pnl_row && row < zero_row;
            let in_loss_bar   = pnl < 0.0 && row > zero_row && row <= pnl_row;
            let is_strike = strike_cols.contains_key(&col);
            let is_sd     = sd_cols.contains(&col);
            let is_spot   = spot_col == Some(col);

            let (ch, color): (&'static str, Color) = if in_profit_bar {
                ("█", C_GREEN)
            } else if in_loss_bar {
                ("█", C_RED)
            } else if row == zero_row && is_strike {
                ("┼", *strike_cols.get(&col).unwrap_or(&C_GRAY))
            } else if row == zero_row && is_sd {
                ("┼", Color::Magenta)
            } else if row == zero_row && is_spot {
                ("┼", C_CYAN)
            } else if row == zero_row {
                ("─", C_GRAY)
            } else if is_strike {
                ("│", *strike_cols.get(&col).unwrap_or(&C_GRAY))
            } else if is_sd {
                ("│", Color::Magenta)
            } else if is_spot {
                ("·", C_CYAN)
            } else {
                (" ", C_DARK)
            };
            spans.push(Span::styled(ch, Style::default().fg(color)));
        }
        grid_lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(grid_lines), chart_area);

    // ── X-axis marker + price label rows ─────────────────────────────────────
    let mut xaxis_lines: Vec<Line<'static>> = Vec::new();
    xaxis_lines.push(Line::from(x1_spans));
    xaxis_lines.push(Line::from(x2_spans));
    if let Some((_, _, em)) = em_info {
        xaxis_lines.push(Line::from(vec![
            Span::styled("  ±1SD Exp Move: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("±${:.2}", em), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::styled("   ", Style::default()),
            Span::styled("SP", Style::default().fg(C_RED)),
            Span::styled("/", Style::default().fg(C_GRAY)),
            Span::styled("SC", Style::default().fg(C_RED)),
            Span::styled(": short   ", Style::default().fg(C_GRAY)),
            Span::styled("LP", Style::default().fg(C_GREEN)),
            Span::styled("/", Style::default().fg(C_GRAY)),
            Span::styled("LC", Style::default().fg(C_GREEN)),
            Span::styled(": long", Style::default().fg(C_GRAY)),
        ]));
        xaxis_lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("|", Style::default().fg(Color::Magenta)),
            Span::styled(": ±1σ   ", Style::default().fg(C_GRAY)),
            Span::styled("●", Style::default().fg(C_CYAN)),
            Span::styled(": price   ", Style::default().fg(C_GRAY)),
            Span::styled("BE", Style::default().fg(C_YELLOW)),
            Span::styled(": breakeven", Style::default().fg(C_GRAY)),
        ]));
    }
    f.render_widget(Paragraph::new(xaxis_lines), xaxis_area);
}

fn badge_color(spread_type: &str) -> Color {
    match spread_type {
        "short_put_vertical" | "short_call_vertical"  => C_CYAN,
        "iron_condor" | "iron_butterfly"               => C_BLUE,
        "strangle" | "straddle"                        => Color::Magenta,
        "cash_secured_put"                             => C_GREEN,
        "covered_call"                                 => Color::Rgb(132, 204, 22),
        "calendar_spread" | "pmcc"                     => C_YELLOW,
        "long_diagonal_spread" | "short_diagonal_spread" => Color::Rgb(249, 115, 22),
        "long_call_vertical" | "long_put_vertical"    => C_CYAN,
        "zebra"                                        => Color::Rgb(168, 85, 247),  // purple
        "put_broken_wing_butterfly"                    => Color::Rgb(251, 146, 60),  // warm orange
        _                                              => C_GRAY,
    }
}

fn draw_equity_curve(f: &mut Frame, area: Rect, trades: &[&Trade]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(C_BLUE))
        .title(Span::styled(" Equity Curve (Closed P&L) ", Style::default().fg(C_CYAN)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if trades.is_empty() || inner.width < 14 || inner.height < 3 {
        f.render_widget(
            Paragraph::new(" No closed trades yet.").style(Style::default().fg(C_GRAY)),
            inner,
        );
        return;
    }

    // 1. Build cumulative PnL series with STEP logic
    let mut running = 0.0_f64;
    let mut data = Vec::with_capacity(trades.len() * 2 + 1);
    data.push((0.0, 0.0));

    let mut min_pnl = 0.0;
    let mut max_pnl = 0.0;

    for (i, t) in trades.iter().enumerate() {
        let x_start = i as f64;
        let x_end = (i + 1) as f64;
        let pnl = t.pnl.unwrap_or(0.0);
        
        data.push((x_start, running));
        running += pnl;
        data.push((x_end, running));

        if running < min_pnl { min_pnl = running; }
        if running > max_pnl { max_pnl = running; }
    }

    let total_trades = trades.len() as f64;
    let y_min = min_pnl.min(0.0);
    let y_max = max_pnl.max(0.1);
    let y_range = y_max - y_min;
    let y_pad = y_range * 0.15;

    let fmt_y = |v: f64| -> String {
        let abs_v = v.abs();
        let s = if abs_v >= 1_000_000.0 { format!("${:.1}M", v / 1_000_000.0) }
                else if abs_v >= 1_000.0  { format!("${:.1}k", abs_v / 1_000.0) }
                else                       { format!("${:.0}",  abs_v) };
        if v >= 0.0 { format!("+{}", s) } else { format!("-{}", s) }
    };

    let main_line = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(if running >= 0.0 { C_GREEN } else { C_RED }))
        .data(&data);

    // Build date labels for x-axis timeline
    let fmt_date = |t: &Trade| -> String {
        let dt = t.exit_date.unwrap_or(t.trade_date);
        dt.format("%b '%y").to_string()
    };
    let first_label  = fmt_date(trades[0]);
    let last_label   = fmt_date(trades[trades.len() - 1]);
    let x_labels = if trades.len() >= 3 {
        let mid_idx = trades.len() / 2;
        vec![
            Span::styled(first_label, Style::default().fg(C_GRAY)),
            Span::styled(fmt_date(trades[mid_idx]), Style::default().fg(C_GRAY)),
            Span::styled(last_label, Style::default().fg(C_GRAY)),
        ]
    } else {
        vec![
            Span::styled(first_label, Style::default().fg(C_GRAY)),
            Span::styled(last_label, Style::default().fg(C_GRAY)),
        ]
    };

    let chart = Chart::new(vec![main_line])
        .x_axis(Axis::default()
            .bounds([0.0, total_trades])
            .labels(x_labels))
        .y_axis(Axis::default()
            .bounds([y_min - y_pad, y_max + y_pad])
            .labels(vec![
                Span::styled(fmt_y(y_min), Style::default().fg(C_RED)),
                Span::styled("$0", Style::default().fg(C_GRAY)),
                Span::styled(fmt_y(y_max), Style::default().fg(C_GREEN)),
            ]));

    f.render_widget(chart, inner);

    let stats_area = Rect::new(inner.x + 2, inner.y + inner.height.saturating_sub(1), inner.width.saturating_sub(4), 1);
    let final_pnl = running;
    let now_color = if final_pnl >= 0.0 { C_GREEN } else { C_RED };
    
    let stats_line = Line::from(vec![
        Span::styled(" Low: ", Style::default().fg(C_GRAY)),
        Span::styled(fmt_y(min_pnl), Style::default().fg(C_RED)),
        Span::styled("  High: ", Style::default().fg(C_GRAY)),
        Span::styled(fmt_y(max_pnl), Style::default().fg(C_GREEN)),
        Span::styled("  Now: ", Style::default().fg(C_GRAY)),
        Span::styled(fmt_y(final_pnl), Style::default().fg(now_color).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(stats_line), stats_area);
}

// ── Performance Tab ───────────────────────────────────────────────────────────

fn perf_section_header(title: &str, width: usize, collapsed: bool, num: Option<u8>, selected: bool) -> Line<'static> {
    let toggle = if selected { "►" } else if collapsed { "▶" } else { "▼" };
    let hdr_color = if selected { C_YELLOW } else { C_CYAN };

    // Number badge: highlighted pill at left edge for instant jump-key recognition
    let (num_badge, badge_len) = match num {
        Some(n) => {
            let label = if n < 10 { format!("[{}]", n) } else { "[0]".to_string() };
            let badge_bg = if selected { Color::Rgb(92, 64, 0) } else { Color::Rgb(30, 58, 138) };
            (Span::styled(
                format!(" {} ", label),
                Style::default().fg(Color::Yellow).bg(badge_bg).add_modifier(Modifier::BOLD),
            ), 5usize)  // " [N] " = 5 display chars
        }
        None => (Span::raw(""), 0usize),
    };

    let prefix_len = badge_len + 9 + title.len(); // " [N] " + " ► ━━━ " + title + " "
    let bar_len = width.saturating_sub(prefix_len).max(2);
    let bar = "━".repeat(bar_len);

    Line::from(vec![
        num_badge,
        Span::styled(format!(" {} ━━━ ", toggle), Style::default().fg(hdr_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", title), Style::default().fg(hdr_color).add_modifier(Modifier::BOLD)),
        Span::styled(bar, Style::default().fg(C_DARK)),
    ])
}

fn perf_health_lines(stats: &PortfolioStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("🛡 PORTFOLIO HEALTH", width, collapsed, Some(1), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if stats.open_trades == 0 {
        lines.push(Line::from(vec![Span::styled("  No open positions.", Style::default().fg(C_GRAY))]));
        return lines;
    }
    let bwd_str = format!("{:+.2}", stats.net_beta_weighted_delta);
    let theta_str = format!("{:+.2}", stats.net_theta);
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Positions: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{}", stats.open_trades), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Net Δ: ", Style::default().fg(C_GRAY)),
        Span::styled(bwd_str, Style::default().fg(C_YELLOW)),
        Span::raw("   "),
        Span::styled("Net Θ: ", Style::default().fg(C_GRAY)),
        Span::styled(theta_str, Style::default().fg(if stats.net_theta >= 0.0 { C_GREEN } else { C_RED })),
    ]));
    let alloc_color = if stats.alloc_pct > 50.0 { C_RED } else if stats.alloc_pct > 30.0 { C_YELLOW } else { C_GREEN };
    let drift_color = if stats.drift.abs() > 5.0 { C_RED } else { C_GREEN };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("BPR: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.0}", stats.total_open_bpr), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Alloc: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}%", stats.alloc_pct), Style::default().fg(alloc_color)),
        Span::raw("   "),
        Span::styled("est. \u{0398}: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.0}", stats.unrealized_pnl), Style::default().fg(if stats.unrealized_pnl >= 0.0 { C_GREEN } else { C_RED })),
        Span::raw("   "),
        Span::styled("Avg POP: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}%", stats.avg_pop), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Drift: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.1}%", stats.drift), Style::default().fg(drift_color)),
    ]));
    let bar_width = (width.saturating_sub(30)).max(10).min(40);
    let undef_blocks = ((stats.undefined_risk_pct / 100.0) * bar_width as f64).round() as usize;
    let def_blocks = bar_width.saturating_sub(undef_blocks);
    let undef_color = if stats.undefined_risk_pct <= stats.target_undefined_pct { C_GREEN } else { C_RED };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Undef ", Style::default().fg(C_GRAY)),
        Span::styled("█".repeat(undef_blocks), Style::default().fg(undef_color)),
        Span::styled("░".repeat(def_blocks), Style::default().fg(C_DARK)),
        Span::styled(format!(" {:.0}%", stats.undefined_risk_pct), Style::default().fg(undef_color)),
        Span::raw("     "),
        Span::styled("Defined ", Style::default().fg(C_GRAY)),
        Span::styled("░".repeat(undef_blocks.min(bar_width / 2)), Style::default().fg(C_DARK)),
        Span::styled("█".repeat(def_blocks.min(bar_width / 2)), Style::default().fg(C_CYAN)),
        Span::styled(format!(" {:.0}%", stats.defined_risk_pct), Style::default().fg(C_CYAN)),
    ]));
    // Item 13: Portfolio Stress Test table
    fn fmt_signed_commas(v: f64) -> String {
        let sign = if v >= 0.0 { "+" } else { "-" };
        let abs = v.abs() as i64;
        let s = abs.to_string();
        let mut out = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 { out.push(','); }
            out.push(c);
        }
        format!("{}{}", sign, out.chars().rev().collect::<String>())
    }
    if !stats.stress_test.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled("  ── STRESS TEST (\u{03b2}-adj · expiry payoff) ──────────────────────".to_string(), Style::default().fg(C_GRAY))]));
        lines.push(Line::from(vec![Span::styled("  SPY Move   P&L at Exp   % of Acct   Worst Position".to_string(), Style::default().fg(C_WHITE))]));
        lines.push(Line::from(vec![Span::styled("  ──────────────────────────────────────────────────".to_string(), Style::default().fg(C_GRAY))]));
        for s in &stats.stress_test {
            let pnl_color = if s.total_pnl >= 0.0 { C_GREEN } else { C_RED };
            let pct_color = if s.pct_of_account >= 0.0 { C_GREEN } else { C_RED };
            let move_str = format!("  {:>5.0}%", s.spy_move_pct);
            let pnl_str = format!("   {:>12}", fmt_signed_commas(s.total_pnl));
            let pct_str = format!("   {:>+7.1}%", s.pct_of_account);
            let worst_color = if s.worst_pnl < 0.0 { C_RED } else { C_GREEN };
            let worst_spans: Vec<Span> = if s.worst_ticker == "—" {
                vec![Span::styled("    —", Style::default().fg(C_GRAY))]
            } else {
                vec![
                    Span::styled(format!("    {}", s.worst_ticker),
                        Style::default().fg(worst_color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" ({})", fmt_signed_commas(s.worst_pnl)),
                        Style::default().fg(worst_color)),
                ]
            };
            let mut row_spans = vec![
                Span::styled(move_str, Style::default().fg(C_WHITE)),
                Span::styled(pnl_str, Style::default().fg(pnl_color)),
                Span::styled(pct_str, Style::default().fg(pct_color)),
            ];
            row_spans.extend(worst_spans);
            lines.push(Line::from(row_spans));
        }
        lines.push(Line::from(vec![Span::styled(
            format!("  Expiry payoff · beta-adjusted · {}/{} positions priced",
                    stats.stress_priced_count, stats.stress_open_count),
            Style::default().fg(C_GRAY),
        )]));
    }
    lines
}

fn perf_returns_lines(stats: &PortfolioStats, perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📈 RETURNS", width, collapsed, Some(2), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if stats.closed_trades == 0 {
        lines.push(Line::from(vec![Span::styled("  No closed trades yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let pnl_color     = if stats.realized_pnl >= 0.0 { C_GREEN } else { C_RED };
    let wr_color      = if stats.win_rate >= 0.65 { C_GREEN } else if stats.win_rate >= 0.50 { C_YELLOW } else { C_RED };
    let pf_color      = if perf.profit_factor >= 1.5 { C_GREEN } else if perf.profit_factor >= 1.0 { C_YELLOW } else { C_RED };
    let ev_color      = if perf.expected_value >= 0.0 { C_GREEN } else { C_RED };
    let roc_color     = if stats.avg_roc >= 0.0 { C_GREEN } else { C_RED };
    let sharpe_color  = if perf.sharpe_ratio  >= 1.0 { C_GREEN } else if perf.sharpe_ratio  >= 0.0 { C_YELLOW } else { C_RED };
    let sortino_color = if perf.sortino_ratio >= 1.5 { C_GREEN } else if perf.sortino_ratio >= 0.75 { C_YELLOW } else { C_RED };
    let calmar_color  = if perf.calmar_ratio  >= 1.0 { C_GREEN } else if perf.calmar_ratio  >= 0.5  { C_YELLOW } else { C_RED };
    let dd_color      = if stats.max_drawdown > 0.0 { C_RED } else { C_GREEN };
    let streak_color  = if stats.current_streak >= 0 { C_GREEN } else { C_RED };
    let streak_str    = if stats.current_streak >= 0 {
        format!("+{}W", stats.current_streak)
    } else {
        format!("{}L", stats.current_streak.abs())
    };

    // ── P&L ──
    lines.push(Line::from(vec![Span::styled("  \u{2500}\u{2500} P&L \u{2500}\u{2500}", Style::default().fg(Color::Rgb(148, 163, 184)))]));

    // Row 1 — Core P&L
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Total P&L: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.2}", stats.realized_pnl), Style::default().fg(pnl_color)),
        Span::raw("   "),
        Span::styled("Win Rate: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}%", stats.win_rate * 100.0), Style::default().fg(wr_color)),
        Span::raw("   "),
        Span::styled("Profit Factor: ", Style::default().fg(C_GRAY)),
        Span::styled(
            if perf.profit_factor.is_infinite() { "\u{221e}".to_string() } else { format!("{:.2}", perf.profit_factor) },
            Style::default().fg(pf_color),
        ),
        Span::raw("   "),
        Span::styled("EV: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.0}/trade", perf.expected_value), Style::default().fg(ev_color)),
        Span::raw("   "),
        Span::styled("Avg ROC: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}%", stats.avg_roc), Style::default().fg(roc_color)),
    ]));

    // Row 2 — Trade outcomes + annualized ROC
    let rr = if perf.avg_loss > 0.0 { perf.avg_win / perf.avg_loss } else { 0.0 };
    let mut row2 = vec![
        Span::raw("  "),
        Span::styled("Avg Winner: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.0}", perf.avg_win), Style::default().fg(C_GREEN)),
        Span::raw("   "),
        Span::styled("Avg Loser: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("-{:.0}", perf.avg_loss), Style::default().fg(C_RED)),
        Span::raw("   "),
        Span::styled("Avg R:R: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("1:{:.2}", rr), Style::default().fg(C_WHITE)),
    ];
    if perf.avg_annualized_roc != 0.0 {
        let ann_color = if perf.avg_annualized_roc >= 0.0 { C_GREEN } else { C_RED };
        let capped = perf.avg_annualized_roc >= 299.9 || perf.avg_annualized_roc <= -299.9;
        row2.push(Span::raw("   "));
        row2.push(Span::styled("Avg Ann. ROC: ", Style::default().fg(C_GRAY)));
        row2.push(Span::styled(
            format!("{:+.1}%/yr{}", perf.avg_annualized_roc, if capped { " \u{2605}" } else { "" }),
            Style::default().fg(ann_color),
        ));
    }
    lines.push(Line::from(row2));

    // Kelly Criterion row (only shown when ≥10 closed trades)
    if let Some(kelly) = perf.kelly_fraction {
        let kc = if kelly > 2.0 { C_GREEN } else if kelly > 0.0 { C_YELLOW } else { C_RED };
        let value_span = if kelly <= 0.0 {
            Span::styled(format!("{:.1}%  (neg edge — reduce size)", kelly), Style::default().fg(C_RED))
        } else {
            let half_k = kelly / 2.0;
            let kelly_dollar = kelly / 100.0 * stats.account_size;
            Span::styled(
                format!("{:.1}%/trade  \u{2192}  ${:.0}  (\u{00bd}K: {:.1}%)", kelly, kelly_dollar, half_k),
                Style::default().fg(kc),
            )
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Kelly Opt: ", Style::default().fg(C_GRAY)),
            value_span,
        ]));
    }

    // Avg Credit per DTE (tastytrade $/DTE cadence metric)
    if let Some(cpd) = perf.avg_credit_per_dte {
        let cpd_color = if cpd >= 5.0 { C_GREEN } else if cpd >= 2.0 { C_YELLOW } else { C_GRAY };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Avg Credit/DTE: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:.2}/d", cpd), Style::default().fg(cpd_color)),
            Span::styled("   (net credit ÷ entry DTE, per trade)", Style::default().fg(C_GRAY)),
        ]));
    }

    // KPI-1: Avg Max Profit/Day — higher = faster theta decay (also higher gamma risk)
    if let Some(mpd) = perf.avg_max_profit_per_day {
        let mpd_color = if mpd >= 10.0 { C_GREEN } else if mpd >= 4.0 { C_YELLOW } else { C_GRAY };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Avg MaxPft/DTE: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:.2}/d", mpd), Style::default().fg(mpd_color)),
            Span::styled("   (max profit ÷ entry DTE, per trade)", Style::default().fg(C_GRAY)),
        ]));
    }

    // KPI-2: Avg IV Crush captured (earnings plays)
    if let Some(crush) = perf.avg_iv_crush {
        if crush.abs() > 0.1 {
            let crush_color = if crush >= 5.0 { C_GREEN } else if crush >= 2.0 { C_YELLOW } else { C_RED };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Avg IV Crush: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:+.1} pts", crush), Style::default().fg(crush_color)),
                Span::styled("   (entry IV − close IV, avg across trades)", Style::default().fg(C_GRAY)),
            ]));
        }
    }

    // KPI-3: Realized theta capture quality — how much of theoretical theta was captured
    // Values >100% = gamma/IV tailwinds; <100% = gap/jump events eroded theta gains
    if let Some(tcq) = perf.avg_theta_capture_quality {
        if tcq.abs() > 0.1 {
            let tcq_color = if tcq >= 80.0 { C_GREEN } else if tcq >= 50.0 { C_YELLOW } else { C_RED };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Theta Capture: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:.1}%", tcq), Style::default().fg(tcq_color)),
                Span::styled("   (<100% = gap/jump losses eroded theta)", Style::default().fg(C_GRAY)),
            ]));
        }
    }

    // ── Risk ──
    lines.push(Line::from(vec![Span::styled("  \u{2500}\u{2500} Risk \u{2500}\u{2500}", Style::default().fg(Color::Rgb(148, 163, 184)))]));

    // Row 3 — Risk ratios
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Sharpe: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.2}", perf.sharpe_ratio), Style::default().fg(sharpe_color)),
        Span::raw("   "),
        Span::styled("Sortino: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.2}", perf.sortino_ratio), Style::default().fg(sortino_color)),
        Span::raw("   "),
        Span::styled("Calmar: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.2}", perf.calmar_ratio), Style::default().fg(calmar_color)),
        Span::raw("   "),
        Span::styled("Max Drawdown: ", Style::default().fg(C_GRAY)),
        Span::styled(
            format!("-{:.0} ({:.1}%)", stats.max_drawdown, stats.max_drawdown_pct),
            Style::default().fg(dd_color),
        ),
    ]));

    // ── Activity ──
    lines.push(Line::from(vec![Span::styled("  \u{2500}\u{2500} Activity \u{2500}\u{2500}", Style::default().fg(Color::Rgb(148, 163, 184)))]));

    // Row 4 — Streaks + trade cadence
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Streak: ", Style::default().fg(C_GRAY)),
        Span::styled(streak_str, Style::default().fg(streak_color)),
        Span::raw("   "),
        Span::styled("MaxWin: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{}", stats.max_win_streak), Style::default().fg(C_GREEN)),
        Span::raw("   "),
        Span::styled("MaxLoss: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{}", stats.max_loss_streak), Style::default().fg(C_RED)),
        Span::raw("   "),
        Span::styled("Trades: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{} closed", stats.closed_trades), Style::default().fg(C_WHITE)),
        Span::raw(" / "),
        Span::styled(format!("{} open", stats.open_trades), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Per Week: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}", perf.trades_per_week), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Per Month: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}", perf.trades_per_month), Style::default().fg(C_WHITE)),
    ]));

    // ── Timing ──
    lines.push(Line::from(vec![Span::styled("  \u{2500}\u{2500} Timing \u{2500}\u{2500}", Style::default().fg(Color::Rgb(148, 163, 184)))]));

    // Row 5 — Timing + capture
    let dte_str = perf.avg_dte_at_close.map_or("\u{2014}".to_string(), |d| format!("{:.1}d", d));
    let pmc_str = perf.avg_pct_max_captured.map_or("\u{2014}".to_string(), |p| format!("{:.1}%", p));
    let mut row5 = vec![
        Span::raw("  "),
        Span::styled("Avg DTE at Close: ", Style::default().fg(C_GRAY)),
        Span::styled(dte_str, Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Avg % Max Captured: ", Style::default().fg(C_GRAY)),
        Span::styled(pmc_str, Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Avg Held: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.0}d", perf.avg_held_days), Style::default().fg(C_WHITE)),
    ];
    if let Some(pr) = perf.avg_premium_recapture {
        let pr_color = if pr >= 50.0 { C_GREEN } else if pr >= 30.0 { C_YELLOW } else { C_RED };
        row5.push(Span::raw("   "));
        row5.push(Span::styled("Avg Premium Recapture: ", Style::default().fg(C_GRAY)));
        row5.push(Span::styled(format!("{:.1}%", pr), Style::default().fg(pr_color)));
        row5.push(Span::styled("  (target \u{2265}50%)", Style::default().fg(C_GRAY)));
    }
    lines.push(Line::from(row5));
    // M5: target hit rate
    if perf.closed_count > 0 {
        let th_color = if perf.target_hit_pct >= 60.0 { C_GREEN } else if perf.target_hit_pct >= 40.0 { C_YELLOW } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Target Hit: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.1}%", perf.target_hit_pct), Style::default().fg(th_color)),
            Span::styled(
                format!("  ({}/{} closed at profit target)", perf.target_hit_count, perf.closed_count),
                Style::default().fg(Color::Rgb(100, 116, 139)),
            ),
        ]));
    }
    // Rolling win rate sparkline (belongs with returns)
    if !perf.rolling_win_rate.is_empty() {
        let current_wr = perf.rolling_win_rate.last().copied().unwrap_or(0.0);
        let wr_color = if current_wr >= 60.0 { C_GREEN } else if current_wr >= 45.0 { C_YELLOW } else { C_RED };
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("Rolling Win Rate ({}-trade): ", perf.rolling_window_used), Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.1}%", current_wr), Style::default().fg(wr_color)),
        ]));
        let spark_data: Vec<f64> = if perf.rolling_win_rate.len() > 40 {
            perf.rolling_win_rate[perf.rolling_win_rate.len() - 40..].to_vec()
        } else {
            perf.rolling_win_rate.clone()
        };
        let spark_chars = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        let spark_str: String = spark_data.iter().map(|&v| {
            let idx = ((v / 100.0) * 7.0).round().clamp(0.0, 7.0) as usize;
            spark_chars[idx]
        }).collect();
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(spark_str, Style::default().fg(wr_color)),
        ]));
    }
    if !perf.rolling_theta_capture.is_empty() {
        let current_tc = perf.rolling_theta_capture.last().copied().unwrap_or(0.0);
        let tc_color = if current_tc >= 80.0 && current_tc <= 120.0 { C_GREEN }
                       else if current_tc >= 50.0 { C_YELLOW } else { C_RED };
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("Rolling Theta Capture ({}-trade): ", perf.rolling_window_used), Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.0}%", current_tc), Style::default().fg(tc_color)),
        ]));
        let spark_data: Vec<f64> = if perf.rolling_theta_capture.len() > 40 {
            perf.rolling_theta_capture[perf.rolling_theta_capture.len() - 40..].to_vec()
        } else {
            perf.rolling_theta_capture.clone()
        };
        let spark_chars = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        let spark_str: String = spark_data.iter().map(|&v| {
            let idx = ((v.clamp(0.0, 200.0) / 200.0) * 7.0).round().clamp(0.0, 7.0) as usize;
            spark_chars[idx]
        }).collect();
        lines.push(Line::from(vec![Span::raw("  "), Span::styled(spark_str, Style::default().fg(tc_color))]));
    }

    // Drawdown sparkline (Item 9)
    if perf.peak_history.len() > 1 {
        let dd_series: Vec<f64> = perf.peak_history.iter().zip(perf.balance_history.iter())
            .map(|(&pk, &bal)| if pk > 0.0 { (pk - bal) / pk * 100.0 } else { 0.0 })
            .collect();
        let max_dd = dd_series.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let spark_chars = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        let spark_data = if dd_series.len() > 40 { dd_series[dd_series.len()-40..].to_vec() } else { dd_series };
        let spark_str: String = spark_data.iter().map(|&v| {
            let idx = ((v / max_dd) * 7.0).round().clamp(0.0, 7.0) as usize;
            spark_chars[idx]
        }).collect();
        let dd_color = if stats.max_drawdown_pct < 5.0 { C_GREEN } else if stats.max_drawdown_pct < 10.0 { C_YELLOW } else { C_RED };
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Drawdown Curve: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("max {:.1}%", stats.max_drawdown_pct), Style::default().fg(dd_color)),
        ]));
        lines.push(Line::from(vec![Span::raw("  "), Span::styled(spark_str, Style::default().fg(dd_color))]));
    }

    lines
}

fn perf_commission_lines(perf: &PerformanceStats, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    if perf.total_commissions <= 0.0 && perf.avg_commission_per_trade <= 0.0 {
        return vec![];
    }
    let mut lines = vec![
        Line::from(""),
        perf_section_header("\u{1F4B0} COMMISSION ANALYSIS", 80, collapsed, Some(10), selected),
    ];
    if collapsed { return lines; }
    let comm_color = if perf.commission_pct_of_gross > 10.0 { C_RED } else if perf.commission_pct_of_gross > 5.0 { C_YELLOW } else { C_GREEN };
    lines.push(Line::from(vec![
            Span::styled("  Total Paid: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:.2}", perf.total_commissions), Style::default().fg(C_RED)),
            Span::raw("   "),
            Span::styled("Avg/Trade: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:.2}", perf.avg_commission_per_trade), Style::default().fg(C_WHITE)),
            Span::raw("   "),
            Span::styled("% of Gross: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.1}%", perf.commission_pct_of_gross), Style::default().fg(comm_color)),
        ]));

    if let Some(fvm) = perf.avg_fill_vs_mid {
        let fvm_color = if fvm >= 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::styled("  Avg Fill vs Mid: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:+.4}", fvm), Style::default().fg(fvm_color)),
            Span::styled(if fvm >= 0.0 { "  (above mid \u{2014} good)" } else { "  (below mid \u{2014} slippage)" }, Style::default().fg(C_GRAY)),
        ]));
    }
    lines
}

fn perf_advanced_lines(stats: &PortfolioStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("⚙ ADVANCED METRICS", width, collapsed, Some(1), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));

    let growth = stats.balance - stats.account_size;
    let growth_pct = if stats.account_size > 0.0 { (growth / stats.account_size) * 100.0 } else { 0.0 };
    let growth_color = if growth >= 0.0 { C_GREEN } else { C_RED };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Balance: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.0}", stats.balance), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Starting: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.0}", stats.account_size), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Growth: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.0} ({:+.2}%)", growth, growth_pct), Style::default().fg(growth_color)),
    ]));

    let theta_color = if stats.net_theta >= 0.0 { C_GREEN } else { C_RED };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Net Theta/Day: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.2}", stats.net_theta), Style::default().fg(theta_color)),
        Span::raw("   "),
        Span::styled("Beta-Weighted Δ: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.2}", stats.net_beta_weighted_delta), Style::default().fg(C_YELLOW)),
        Span::raw("   "),
        Span::styled("Open BPR: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.0}", stats.total_open_bpr), Style::default().fg(C_WHITE)),
    ]));
    lines
}

fn draw_perf_growth_chart(f: &mut Frame, area: Rect, stats: &PortfolioStats, perf: &PerformanceStats, collapsed: bool, selected: bool) {
    let border_color = if selected { C_YELLOW } else { C_DARK };
    if collapsed {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(" 📊 ACCOUNT GROWTH ▶ (key 3) ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)));
        f.render_widget(block, area);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" 📊 ACCOUNT GROWTH ▼ (key 3) ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)));

    let history = &perf.balance_history;
    if history.len() < 2 {
        f.render_widget(
            Paragraph::new("  No closed trades yet.")
                .style(Style::default().fg(C_GRAY))
                .block(block),
            area,
        );
        return;
    }

    let _inner = block.inner(area);

    // M11: include unrealized point in y-range calculation if present
    let unreal = &perf.unrealized_history;
    let all_vals = history.iter().chain(unreal.iter());
    let min_val = all_vals.clone().cloned().fold(f64::INFINITY, f64::min);
    let max_val = all_vals.cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = (min_val * 0.998).floor();
    let y_max = (max_val * 1.002).ceil();

    let data_points: Vec<(f64, f64)> = history.iter().enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();

    let last_bal = *history.last().unwrap();
    let growth_color = if last_bal >= stats.account_size { C_GREEN } else { C_RED };

    // M11: second dataset showing realized + projected unrealized (theta decay)
    let unreal_points: Vec<(f64, f64)> = unreal.iter().enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();

    let mut datasets = vec![
        Dataset::default()
            .name(format!("Realized ${:.0} ({:+.1}%)", last_bal, if stats.account_size > 0.0 { (last_bal - stats.account_size) / stats.account_size * 100.0 } else { 0.0 }))
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(growth_color))
            .data(&data_points),
    ];
    if unreal_points.len() > data_points.len() {
        let last_unreal = unreal.last().cloned().unwrap_or(last_bal);
        datasets.push(
            Dataset::default()
                .name(format!("+θ est. ${:.0}", last_unreal))
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(C_YELLOW))
                .data(&unreal_points),
        );
    }

    let x_len = history.len().max(unreal.len());
    let x_labels = vec![
        Span::styled("1", Style::default().fg(C_GRAY)),
        Span::styled(format!("{}", x_len), Style::default().fg(C_GRAY)),
    ];
    let y_labels = vec![
        Span::styled(format!("${:.0}", y_min), Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.0}", y_max), Style::default().fg(C_GRAY)),
    ];

    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .title(Span::styled("Trades", Style::default().fg(C_GRAY)))
                .bounds([0.0, (x_len - 1) as f64])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .bounds([y_min, y_max])
                .labels(y_labels),
        );

    f.render_widget(chart, area);
}

fn perf_strategy_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("⚔ STRATEGY BREAKDOWN", width, collapsed, Some(2), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.strategy_breakdown.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No closed trades yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    // Header: col widths must match data spans exactly
    // name(24) trades(6) "  " win%(6) "  " pnl(10) "  " avgpnl(8) "  " avgroc(8) cw%(6) edte(4)
    lines.push(Line::from(vec![Span::styled(
        format!("  {:<24}{:>6}  {:>6}  {:>10}  {:>8}  {:>8}{:>6}{:>4}",
            "Strategy", "Trades", "Win%", "P&L", "Avg P&L", "Avg ROC", "C/W%", "eDTE"),
        Style::default().fg(C_GRAY),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", "\u{2500}".repeat(24+6+2+6+2+10+2+8+2+8+6+4)),
        Style::default().fg(C_DARK),
    )]));

    let mut total_trades = 0usize;
    let mut total_wins = 0usize;
    let mut total_pnl = 0.0_f64;

    for sb in &perf.strategy_breakdown {
        total_trades += sb.trades;
        total_wins += sb.wins;
        total_pnl += sb.total_pnl;

        let wr_color = if sb.win_rate >= 65.0 { C_GREEN } else if sb.win_rate >= 50.0 { C_YELLOW } else { C_RED };
        let pnl_color = if sb.total_pnl >= 0.0 { C_GREEN } else { C_RED };
        let roc_color = if sb.avg_roc >= 5.0 { C_GREEN } else if sb.avg_roc >= 0.0 { C_YELLOW } else { C_RED };
        let strat_name = sb.strategy.label().to_string();
        // cw_str = 6 chars always; dte_str = 4 chars always
        let cw_str = sb.avg_cw_ratio.map_or("     \u{2014}".to_string(), |r| format!(" {:>4.0}%", r));
        let dte_str = sb.avg_entry_dte.map_or("   \u{2014}".to_string(), |d| format!(" {:>3.0}", d));
        let cw_color = sb.avg_cw_ratio.map_or(C_GRAY, |r| if r >= 33.0 { C_GREEN } else if r >= 20.0 { C_YELLOW } else { C_RED });
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<24}", strat_name), Style::default().fg(C_CYAN)),
            Span::styled(format!("{:>6}", sb.trades), Style::default().fg(C_WHITE)),
            Span::raw("  "),
            Span::styled(format!("{:>5.0}%", sb.win_rate), Style::default().fg(wr_color)),
            Span::raw("  "),
            Span::styled(format!("{:>+10.0}", sb.total_pnl), Style::default().fg(pnl_color)),
            Span::raw("  "),
            Span::styled(format!("{:>+8.0}", sb.avg_pnl), Style::default().fg(pnl_color)),
            Span::raw("  "),
            Span::styled(format!("{:>7.1}%", sb.avg_roc), Style::default().fg(roc_color)),
            Span::styled(cw_str, Style::default().fg(cw_color)),
            Span::styled(dte_str, Style::default().fg(C_GRAY)),
        ]));
    }

    lines.push(Line::from(vec![Span::styled(
        format!("  {}", "\u{2500}".repeat(24+6+2+6+2+10+2+8+2+8+6+4)),
        Style::default().fg(C_DARK),
    )]));
    let total_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_pnl_total = if total_trades > 0 { total_pnl / total_trades as f64 } else { 0.0 };
    let total_pnl_color = if total_pnl >= 0.0 { C_GREEN } else { C_RED };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{:<24}", "TOTAL"), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:>6}", total_trades), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("{:>5.0}%", total_wr), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("{:>+10.0}", total_pnl), Style::default().fg(total_pnl_color).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("{:>+8.0}", avg_pnl_total), Style::default().fg(total_pnl_color).add_modifier(Modifier::BOLD)),
    ]));

    // Strategy P&L bar chart — cumulative P&L per strategy
    if perf.strategy_breakdown.len() > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled("  P&L by Strategy", Style::default().fg(C_GRAY))]));
        let max_abs_pnl = perf.strategy_breakdown.iter()
            .map(|sb| sb.total_pnl.abs())
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let bar_max = (width.saturating_sub(40)).max(8).min(30);
        for sb in &perf.strategy_breakdown {
            let bar_len = ((sb.total_pnl.abs() / max_abs_pnl) * bar_max as f64).round() as usize;
            let pnl_color = if sb.total_pnl >= 0.0 { C_GREEN } else { C_RED };
            let bar_char = if sb.total_pnl >= 0.0 { "█" } else { "░" };
            let bar = bar_char.repeat(bar_len);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<20}", sb.strategy.label()), Style::default().fg(C_CYAN)),
                Span::styled(format!("{:>+8.0}  ", sb.total_pnl), Style::default().fg(pnl_color)),
                Span::styled(bar, Style::default().fg(pnl_color)),
            ]));
        }
    }

    lines
}

fn perf_ticker_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("🎯 TICKER BREAKDOWN", width, collapsed, Some(3), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.ticker_breakdown.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No closed trades yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    // Header: name(24) trades(6) "  " win%(6) "  " pnl(10) "  " avgpnl(8) "  " avgroc(8) "  " ivr(6) dte(6)
    lines.push(Line::from(vec![Span::styled(
        format!("  {:<24}{:>6}  {:>6}  {:>10}  {:>8}  {:>8}  {:>6}{:>6}",
            "Ticker", "Trades", "Win%", "P&L", "Avg P&L", "Avg ROC", "AvgIVR", "AvgDTE"),
        Style::default().fg(C_GRAY),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", "\u{2500}".repeat(24+6+2+6+2+10+2+8+2+8+2+6+6)),
        Style::default().fg(C_DARK),
    )]));

    for tb in &perf.ticker_breakdown {
        let wr_color  = if tb.win_rate  >= 65.0 { C_GREEN } else if tb.win_rate  >= 50.0 { C_YELLOW } else { C_RED };
        let pnl_color = if tb.total_pnl >= 0.0  { C_GREEN } else { C_RED };
        let roc_color = if tb.avg_roc   >= 5.0  { C_GREEN } else if tb.avg_roc   >= 0.0  { C_YELLOW } else { C_RED };
        // ivr_str = 6 chars always; dte_str = 6 chars always
        let ivr_str   = tb.avg_ivr.map_or("     \u{2014}".to_string(), |v| format!("{:>6.0}", v));
        let ivr_color = tb.avg_ivr.map_or(C_GRAY, |v| if v >= 50.0 { C_GREEN } else if v >= 30.0 { C_YELLOW } else { C_GRAY });
        let dte_str   = tb.avg_entry_dte.map_or("     \u{2014}".to_string(), |v| format!("{:>5.0}d", v));
        let dte_color = tb.avg_entry_dte.map_or(C_GRAY, |v| if v >= 30.0 && v <= 60.0 { C_GREEN } else { C_YELLOW });
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<24}", tb.ticker), Style::default().fg(C_CYAN)),
            Span::styled(format!("{:>6}", tb.trades), Style::default().fg(C_WHITE)),
            Span::raw("  "),
            Span::styled(format!("{:>5.0}%", tb.win_rate), Style::default().fg(wr_color)),
            Span::raw("  "),
            Span::styled(format!("{:>+10.0}", tb.total_pnl), Style::default().fg(pnl_color)),
            Span::raw("  "),
            Span::styled(format!("{:>+8.0}", tb.avg_pnl), Style::default().fg(pnl_color)),
            Span::raw("  "),
            Span::styled(format!("{:>7.1}%", tb.avg_roc), Style::default().fg(roc_color)),
            Span::raw("  "),
            Span::styled(ivr_str, Style::default().fg(ivr_color)),
            Span::styled(dte_str, Style::default().fg(dte_color)),
        ]));
    }
    lines
}

fn perf_monthly_lines(
    perf: &PerformanceStats,
    spy_monthly: &std::collections::HashMap<(i32, u32), f64>,
    width: usize,
    collapsed: bool,
    selected: bool,
    account_size: f64,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📅 MONTHLY P&L", width, collapsed, Some(4), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.monthly_pnl.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No closed trades yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let max_abs = perf.monthly_pnl.iter().map(|m| m.pnl.abs()).fold(0.0_f64, f64::max).max(1.0);
    let bar_max = (width.saturating_sub(44)).max(10).min(40);
    let month_names = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun",
                       "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    for mp in &perf.monthly_pnl {
        let m_name = month_names.get(mp.month as usize).unwrap_or(&"???");
        let bar_len = ((mp.pnl.abs() / max_abs) * bar_max as f64).round() as usize;
        let pnl_color = if mp.pnl >= 0.0 { C_GREEN } else { C_RED };
        let bar_str = if mp.pnl >= 0.0 {
            format!("{}", "█".repeat(bar_len))
        } else {
            format!("{}{}", "░".repeat(bar_max.saturating_sub(bar_len)), "█".repeat(bar_len))
        };
        let spy_ret = spy_monthly.get(&(mp.year, mp.month)).copied();
        let spy_span = match spy_ret {
            Some(r) => Span::styled(
                format!("  SPY{:+.1}%", r),
                Style::default().fg(if r >= 0.0 { C_GREEN } else { C_RED }),
            ),
            None => Span::styled("  SPY  —  ".to_string(), Style::default().fg(C_GRAY)),
        };
        // Per-row alpha: portfolio return vs SPY this month
        let alpha_span = if account_size > 0.0 {
            match spy_ret {
                Some(spy_r) => {
                    let port_r = mp.pnl / account_size * 100.0;
                    let alpha = port_r - spy_r;
                    Span::styled(
                        format!("  α{:+.1}%", alpha),
                        Style::default().fg(if alpha >= 0.0 { C_GREEN } else { C_RED }),
                    )
                },
                None => Span::raw(""),
            }
        } else {
            Span::raw("")
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} {:4}", m_name, mp.year), Style::default().fg(C_GRAY)),
            Span::styled(format!(" ({:2}T) ", mp.trade_count), Style::default().fg(C_GRAY)),
            Span::styled(format!("{:>+8.0}   ", mp.pnl), Style::default().fg(pnl_color)),
            Span::styled(bar_str, Style::default().fg(pnl_color)),
            spy_span,
            alpha_span,
        ]));
    }

    // Portfolio vs SPY comparison sparklines
    if !perf.monthly_pnl.is_empty() && account_size > 0.0 {
        let port_returns: Vec<f64> = perf.monthly_pnl.iter().map(|mp| mp.pnl / account_size * 100.0).collect();
        let spy_returns: Vec<f64> = perf.monthly_pnl.iter().map(|mp| spy_monthly.get(&(mp.year, mp.month)).copied().unwrap_or(0.0)).collect();
        let alpha_series: Vec<f64> = port_returns.iter().zip(spy_returns.iter()).map(|(p, s)| p - s).collect();

        let global_max = port_returns.iter().chain(spy_returns.iter()).map(|v| v.abs()).fold(0.0_f64, f64::max).max(0.01);
        let spark_chars = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        let make_spark = |series: &[f64]| -> String {
            series.iter().map(|&v| {
                let norm = (v + global_max) / (2.0 * global_max);
                let idx = (norm * 7.0).round().clamp(0.0, 7.0) as usize;
                spark_chars[idx]
            }).collect()
        };
        let alpha_max = alpha_series.iter().map(|v| v.abs()).fold(0.0_f64, f64::max).max(0.01);
        let make_alpha_spark = |series: &[f64]| -> String {
            series.iter().map(|&v| {
                let norm = (v + alpha_max) / (2.0 * alpha_max);
                let idx = (norm * 7.0).round().clamp(0.0, 7.0) as usize;
                spark_chars[idx]
            }).collect()
        };

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled("  Portfolio vs SPY (monthly %)", Style::default().fg(C_GRAY))]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Port: ", Style::default().fg(C_CYAN)),
            Span::styled(make_spark(&port_returns), Style::default().fg(C_CYAN)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("SPY:  ", Style::default().fg(C_YELLOW)),
            Span::styled(make_spark(&spy_returns), Style::default().fg(C_YELLOW)),
        ]));
        let last_alpha = alpha_series.last().copied().unwrap_or(0.0);
        let alpha_color = if last_alpha >= 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("Alpha({:+.1}%): ", last_alpha), Style::default().fg(alpha_color)),
            Span::styled(make_alpha_spark(&alpha_series), Style::default().fg(alpha_color)),
        ]));
    }

    lines
}

fn perf_ivr_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📊 IV RANK AT ENTRY", width, collapsed, Some(5), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.ivr_buckets.iter().all(|b| b.trades == 0) {
        lines.push(Line::from(vec![Span::styled("  No IVR data yet — log iv_rank at entry to populate.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let bar_max = (width.saturating_sub(50)).max(8).min(25);
    lines.push(Line::from(vec![Span::styled(
        format!("  {:<14}{:<bar_max$} {:>6}  {:>5}  {:>8}", "IVR Range", "Bar", "Trades", "Win%", "Avg P&L", bar_max = bar_max + 1),
        Style::default().fg(C_GRAY),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", "\u{2500}".repeat(14 + bar_max + 1 + 6 + 2 + 5 + 2 + 8)),
        Style::default().fg(C_DARK),
    )]));

    let max_count = perf.ivr_buckets.iter().map(|b| b.trades).max().unwrap_or(1).max(1);
    for b in &perf.ivr_buckets {
        let bar_len = ((b.trades as f64 / max_count as f64) * bar_max as f64).round() as usize;
        let empty_len = bar_max.saturating_sub(bar_len);
        let wr_color = if b.win_rate >= 65.0 { C_GREEN } else if b.win_rate >= 50.0 { C_YELLOW } else { C_RED };
        let pnl_color = if b.avg_pnl >= 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<14}", b.label), Style::default().fg(C_CYAN)),
            Span::styled("\u{2588}".repeat(bar_len), Style::default().fg(C_BLUE)),
            Span::styled("\u{2591}".repeat(empty_len), Style::default().fg(C_DARK)),
            Span::raw(" "),
            Span::styled(format!("{:>6}", b.trades), Style::default().fg(C_WHITE)),
            Span::raw("  "),
            Span::styled(format!("{:>5.0}%", b.win_rate), Style::default().fg(wr_color)),
            Span::raw("  "),
            Span::styled(format!("{:>+8.0}", b.avg_pnl), Style::default().fg(pnl_color)),
        ]));
    }
    lines
}

fn perf_vix_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("🌡 VIX REGIME", width, collapsed, Some(6), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.vix_regimes.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No VIX regime data yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let bar_max = (width.saturating_sub(50)).max(8).min(25);
    lines.push(Line::from(vec![Span::styled(
        format!("  {:<14}{:<bar_max$} {:>6}  {:>5}  {:>8}", "VIX Regime", "Bar", "Trades", "Win%", "Avg P&L", bar_max = bar_max + 1),
        Style::default().fg(C_GRAY),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", "\u{2500}".repeat(14 + bar_max + 1 + 6 + 2 + 5 + 2 + 8)),
        Style::default().fg(C_DARK),
    )]));

    let max_count = perf.vix_regimes.iter().map(|b| b.trades).max().unwrap_or(1).max(1);
    for v in &perf.vix_regimes {
        let bar_len = ((v.trades as f64 / max_count as f64) * bar_max as f64).round() as usize;
        let empty_len = bar_max.saturating_sub(bar_len);
        let wr_color = if v.win_rate >= 65.0 { C_GREEN } else if v.win_rate >= 50.0 { C_YELLOW } else { C_RED };
        let pnl_color = if v.avg_pnl >= 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<14}", v.label), Style::default().fg(C_CYAN)),
            Span::styled("\u{2588}".repeat(bar_len), Style::default().fg(C_BLUE)),
            Span::styled("\u{2591}".repeat(empty_len), Style::default().fg(C_DARK)),
            Span::raw(" "),
            Span::styled(format!("{:>6}", v.trades), Style::default().fg(C_WHITE)),
            Span::raw("  "),
            Span::styled(format!("{:>5.0}%", v.win_rate), Style::default().fg(wr_color)),
            Span::raw("  "),
            Span::styled(format!("{:>+8.0}", v.avg_pnl), Style::default().fg(pnl_color)),
        ]));
    }
    lines
}

fn perf_dte_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("⏱ EXIT DTE ANALYSIS", width, collapsed, Some(7), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.dte_buckets.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No DTE data yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let bar_max = (width.saturating_sub(50)).max(8).min(25);
    lines.push(Line::from(vec![Span::styled(
        format!("  {:<14}{:<bar_max$} {:>6}  {:>5}  {:>8}", "DTE at Close", "Bar", "Trades", "Win%", "Avg P&L", bar_max = bar_max + 1),
        Style::default().fg(C_GRAY),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", "\u{2500}".repeat(14 + bar_max + 1 + 6 + 2 + 5 + 2 + 8)),
        Style::default().fg(C_DARK),
    )]));

    let max_count = perf.dte_buckets.iter().map(|b| b.trades).max().unwrap_or(1).max(1);
    for d in &perf.dte_buckets {
        let bar_len = ((d.trades as f64 / max_count as f64) * bar_max as f64).round() as usize;
        let empty_len = bar_max.saturating_sub(bar_len);
        let wr_color = if d.win_rate >= 65.0 { C_GREEN } else if d.win_rate >= 50.0 { C_YELLOW } else { C_RED };
        let pnl_color = if d.avg_pnl >= 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<14}", d.label), Style::default().fg(C_CYAN)),
            Span::styled("\u{2588}".repeat(bar_len), Style::default().fg(C_BLUE)),
            Span::styled("\u{2591}".repeat(empty_len), Style::default().fg(C_DARK)),
            Span::raw(" "),
            Span::styled(format!("{:>6}", d.trades), Style::default().fg(C_WHITE)),
            Span::raw("  "),
            Span::styled(format!("{:>5.0}%", d.win_rate), Style::default().fg(wr_color)),
            Span::raw("  "),
            Span::styled(format!("{:>+8.0}", d.avg_pnl), Style::default().fg(pnl_color)),
        ]));
    }
    lines
}

fn perf_ivr_entry_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📈 IVR ENTRY HISTOGRAM", width, collapsed, Some(8), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.ivr_entry_buckets.iter().all(|b| b.count == 0) {
        lines.push(Line::from(vec![Span::styled("  No IVR entry data yet — log iv_rank at entry to populate.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let max_count = perf.ivr_entry_buckets.iter().map(|b| b.count).max().unwrap_or(1).max(1);
    let bar_max = (width.saturating_sub(40)).max(10).min(30);

    for b in &perf.ivr_entry_buckets {
        let bar_len = ((b.count as f64 / max_count as f64) * bar_max as f64).round() as usize;
        let wr_color = if b.win_rate >= 65.0 { C_GREEN } else if b.win_rate >= 50.0 { C_YELLOW } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<14}", b.label), Style::default().fg(C_CYAN)),
            Span::styled("█".repeat(bar_len), Style::default().fg(C_BLUE)),
            Span::styled("░".repeat(bar_max.saturating_sub(bar_len)), Style::default().fg(C_DARK)),
            Span::styled(format!(" {:>3}", b.count), Style::default().fg(C_WHITE)),
            Span::raw("  "),
            Span::styled(format!("Win:{:.0}%", b.win_rate), Style::default().fg(wr_color)),
        ]));
    }
    lines
}

/// Item 5: Monthly P&L trend line chart — rendered in Overview when ≥3 months of data exist.
fn draw_perf_monthly_chart(f: &mut Frame, area: Rect, perf: &PerformanceStats) {
    let data_points: Vec<(f64, f64)> = perf.monthly_pnl.iter().enumerate()
        .map(|(i, m)| (i as f64, m.pnl))
        .collect();

    let min_pnl = perf.monthly_pnl.iter().map(|m| m.pnl).fold(f64::INFINITY, f64::min);
    let max_pnl = perf.monthly_pnl.iter().map(|m| m.pnl).fold(f64::NEG_INFINITY, f64::max);
    let y_min = (min_pnl * 1.1).min(min_pnl - 1.0);
    let y_max = (max_pnl * 1.1).max(max_pnl + 1.0);
    let n = perf.monthly_pnl.len();
    let last_pnl = perf.monthly_pnl.last().map(|m| m.pnl).unwrap_or(0.0);
    let line_color = if last_pnl >= 0.0 { C_GREEN } else { C_RED };

    let datasets = vec![
        Dataset::default()
            .name(format!("{} months", n))
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(line_color))
            .data(&data_points),
    ];

    let x_labels = vec![
        Span::styled("1", Style::default().fg(C_GRAY)),
        Span::styled(format!("{}", n), Style::default().fg(C_GRAY)),
    ];
    let y_labels = vec![
        Span::styled(format!("${:.0}", y_min), Style::default().fg(C_GRAY)),
        Span::styled("$0", Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.0}", y_max), Style::default().fg(C_GRAY)),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_DARK))
                .title(Span::styled(" 📅 MONTHLY P&L TREND ", Style::default().fg(C_CYAN))),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(C_DARK))
                .bounds([0.0, (n as f64 - 1.0).max(1.0)])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(C_DARK))
                .bounds([y_min, y_max])
                .labels(y_labels),
        );

    f.render_widget(chart, area);
}

fn perf_pnl_dist_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📊 P&L DISTRIBUTION", width, collapsed, Some(9), selected),
    ];
    if collapsed { return lines; }
    if perf.pnl_buckets.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No closed trades", Style::default().fg(C_GRAY))]));
        return lines;
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ◆ = normal distribution fit",
        Style::default().fg(Color::Rgb(100, 116, 139)),
    )]));
    lines.push(Line::from(""));
    let max_count = perf.pnl_buckets.iter().map(|b| b.count).max().unwrap_or(1).max(1);
    let max_normal = perf.pnl_buckets.iter().map(|b| b.normal_count).fold(0.0_f64, f64::max).max(1.0);
    let bar_width = 20usize;
    for bucket in &perf.pnl_buckets {
        let bar_len = (bucket.count * bar_width / max_count).max(if bucket.count > 0 { 1 } else { 0 });
        let norm_len = ((bucket.normal_count / max_normal) * bar_width as f64).round() as usize;
        let color = if bucket.label.contains('-') || bucket.label.starts_with('<') { C_RED } else { C_GREEN };
        // Build bar spans: actual bar + ◆ overlay, all character-index safe
        let marker_pos = norm_len.min(22);
        let actual_end = bar_len.min(22);
        // Part A: bar chars before marker
        let part_a = "▓".repeat(actual_end.min(marker_pos));
        // Part B: spaces between bar end and marker (if marker past bar end)
        let part_b = if marker_pos > actual_end { " ".repeat(marker_pos - actual_end) } else { String::new() };
        // Part C: marker itself (or empty if past end)
        let part_c = if marker_pos < 22 { "◆" } else { "" };
        // Part D: bar chars after marker (if bar extends past marker)
        let part_d = if actual_end > marker_pos { "▓".repeat(actual_end - marker_pos - 1) } else { String::new() };
        // Part E: trailing spaces to fill 22 chars
        let used = part_a.chars().count() + part_b.chars().count() + part_c.chars().count() + part_d.chars().count();
        let part_e = " ".repeat(22usize.saturating_sub(used));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<16}", bucket.label), Style::default().fg(C_GRAY)),
            Span::styled(part_a, Style::default().fg(color)),
            Span::styled(part_b, Style::default().fg(color)),
            Span::styled(part_c, Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(part_d, Style::default().fg(color)),
            Span::styled(part_e, Style::default()),
            Span::styled(format!("{:>3} trades", bucket.count), Style::default().fg(C_WHITE)),
            Span::styled(format!("  {:.1}%", bucket.pct), Style::default().fg(C_GRAY)),
        ]));
    }
    lines
}

fn perf_held_lines(perf: &PerformanceStats, width: usize, collapsed: bool, selected: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("⏳ TIME IN TRADE", width, collapsed, Some(10), selected),
    ];
    if collapsed { return lines; }
    lines.push(Line::from(""));
    if perf.held_buckets.iter().all(|b| b.trades == 0) {
        lines.push(Line::from(vec![Span::styled("  No hold-time data yet — close some trades to populate.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let max_count = perf.held_buckets.iter().map(|b| b.trades).max().unwrap_or(1).max(1);
    let bar_max = (width.saturating_sub(40)).max(10).min(30);

    for b in &perf.held_buckets {
        let bar_len = ((b.trades as f64 / max_count as f64) * bar_max as f64).round() as usize;
        let wr_color = if b.win_rate >= 65.0 { C_GREEN } else if b.win_rate >= 50.0 { C_YELLOW } else { C_RED };
        let pnl_color = if b.avg_pnl >= 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<10}", b.label), Style::default().fg(C_CYAN)),
            Span::styled("\u{2588}".repeat(bar_len), Style::default().fg(C_BLUE)),
            Span::styled("\u{2591}".repeat(bar_max.saturating_sub(bar_len)), Style::default().fg(C_DARK)),
            Span::styled(format!(" {:>3}", b.trades), Style::default().fg(C_WHITE)),
            Span::raw("  "),
            Span::styled(format!("Win:{:.0}%", b.win_rate), Style::default().fg(wr_color)),
            Span::raw("  "),
            Span::styled(format!("Avg:{:+.0}", b.avg_pnl), Style::default().fg(pnl_color)),
        ]));
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn draw_performance(
    f: &mut Frame,
    area: Rect,
    stats: &PortfolioStats,
    perf: &PerformanceStats,
    perf_subtab: usize,
    overview_scroll: u16,
    analytics_scroll: u16,
    spy_monthly: &std::collections::HashMap<(i32, u32), f64>,
    collapsed: &[bool; 14],
    perf_section_cursor: usize,
    under_tauri: bool,
) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(C_BLUE))
        .title(Span::styled(" ★ Performance ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let width = inner.width as usize;

    // Subtab bar (1 row)
    let tab_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let subtabs = Tabs::new(vec![" Overview ", " Charts ", " Analytics "])
        .select(perf_subtab)
        .style(Style::default().fg(C_GRAY))
        .highlight_style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD))
        .divider(Span::styled(" │ ", Style::default().fg(C_DARK)));
    f.render_widget(subtabs, tab_chunks[0]);

    let content_area = tab_chunks[1];

    // Determine which collapsed[] index the cursor points to
    const OVERVIEW_MAP:  [usize; 2] = [0, 1];
    const CHARTS_MAP:    [usize; 1] = [3];
    const ANALYTICS_MAP: [usize; 11] = [2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    let selected_gi: Option<usize> = match perf_subtab {
        0 => OVERVIEW_MAP.get(perf_section_cursor).copied(),
        1 => CHARTS_MAP.get(perf_section_cursor).copied(),
        _ => ANALYTICS_MAP.get(perf_section_cursor).copied(),
    };

    if perf_subtab == 0 {
        // OVERVIEW: health/returns text only (charts moved to Charts sub-tab)
        let mut lines: Vec<Line> = Vec::new();
        lines.extend(perf_health_lines(stats, width, collapsed[0], selected_gi == Some(0)));
        lines.extend(perf_returns_lines(stats, perf, width, collapsed[1], selected_gi == Some(1)));
        lines.push(Line::from(""));

        let para = Paragraph::new(lines).scroll((overview_scroll, 0));
        f.render_widget(para, content_area);
    } else if perf_subtab == 1 {
        // CHARTS: rendered as SVG in Tauri right panel (or ratatui ASCII when standalone)
        if under_tauri {
            let lines = vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "  Charts displayed in Tauri panel \u{2192}",
                    Style::default().fg(C_CYAN),
                )]),
                Line::from(vec![Span::styled(
                    "  (navigate away and back to refresh)",
                    Style::default().fg(C_GRAY),
                )]),
            ];
            f.render_widget(Paragraph::new(lines), content_area);
        } else {
        // ratatui ASCII fallback (standalone / no Tauri)
        let chart_h: u16 = if collapsed[3] { 3 } else { 12 };
        let monthly_chart_h: u16 = if !collapsed[3] && perf.monthly_pnl.len() >= 3 { 7 } else { 0 };
        let dd_section_h: u16 = if !collapsed[3] && perf.balance_history.len() > 2 { 5 } else { 0 };
        let wr_trend_h: u16 = if !collapsed[3] && perf.monthly_pnl.len() >= 3 { 5 } else { 0 };
        let scatter_h: u16 = if !collapsed[3] && perf.dte_roc_scatter.len() >= 3 { 14 } else { 0 };
        let bpr_h: u16 = if !collapsed[3] && perf.bpr_history.len() >= 3 { 10 } else { 0 };
        let sector_h: u16 = if !collapsed[3] && !perf.sector_trends.is_empty() && perf.monthly_pnl.len() >= 2 {
            (perf.sector_trends.len() as u16 + 4).min(12)
        } else { 0 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(chart_h),
                Constraint::Length(monthly_chart_h),
                Constraint::Length(dd_section_h),
                Constraint::Length(wr_trend_h),
                Constraint::Length(scatter_h),
                Constraint::Length(bpr_h),
                Constraint::Length(sector_h),
                Constraint::Min(0),
            ])
            .split(content_area);

        draw_perf_growth_chart(f, chunks[0], stats, perf, collapsed[3], selected_gi == Some(3));

        if !collapsed[3] && perf.monthly_pnl.len() >= 3 {
            draw_perf_monthly_chart(f, chunks[1], perf);
        }

        // Drawdown over time sparkline
        if !collapsed[3] && perf.balance_history.len() > 2 && stats.account_size > 0.0 {
            let dd_series: Vec<f64> = perf.peak_history.iter().zip(perf.balance_history.iter())
                .map(|(&pk, &bal)| ((bal - pk) / stats.account_size * 100.0).min(0.0))
                .collect();
            let spark_chars = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
            let min_dd = dd_series.iter().cloned().fold(0.0_f64, f64::min).min(-0.01);
            let spark: String = dd_series.iter().map(|&v| {
                // 0.0 = no drawdown → full bar; min_dd = worst → empty bar
                let norm = (v - min_dd) / min_dd.abs();
                let idx = (7.0 - (norm * 7.0).round()).clamp(0.0, 7.0) as usize;
                spark_chars[idx]
            }).collect();
            let current_dd = dd_series.last().copied().unwrap_or(0.0);
            let dd_color = if current_dd < -5.0 { C_RED } else if current_dd < -2.0 { C_YELLOW } else { C_GREEN };
            let dd_lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Drawdown History ", Style::default().fg(C_GRAY)),
                    Span::styled(format!("(current: {:.1}% of acct)", current_dd), Style::default().fg(dd_color)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(spark, Style::default().fg(C_RED)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  Worst: {:.1}%", min_dd), Style::default().fg(C_RED)),
                    Span::styled(format!("   Current: {:.1}%", current_dd), Style::default().fg(dd_color)),
                ]),
            ];
            f.render_widget(Paragraph::new(dd_lines), chunks[2]);
        }

        // H5: Monthly win rate trend sparkline
        if !collapsed[3] && perf.monthly_pnl.len() >= 3 {
            let spark_chars = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
            let wr_series: Vec<f64> = perf.monthly_pnl.iter()
                .filter(|m| m.trade_count >= 2)
                .map(|m| m.win_count as f64 / m.trade_count as f64 * 100.0)
                .collect();
            if wr_series.len() >= 2 {
                let latest = *wr_series.last().unwrap_or(&0.0);
                let wr_color = if latest >= 60.0 { C_GREEN } else if latest >= 45.0 { C_YELLOW } else { C_RED };
                let trend_color = if wr_series.len() >= 3 {
                    let recent_avg = wr_series[wr_series.len().saturating_sub(3)..].iter().sum::<f64>() / 3.0_f64.min(wr_series.len() as f64);
                    let older_avg  = wr_series[..wr_series.len().saturating_sub(3)].iter().cloned().sum::<f64>()
                        / (wr_series.len().saturating_sub(3)).max(1) as f64;
                    if wr_series.len() < 4 { wr_color } else if recent_avg >= older_avg { C_GREEN } else { C_RED }
                } else { wr_color };
                let spark: String = wr_series.iter().map(|&v| {
                    let norm = (v / 100.0).clamp(0.0, 1.0);
                    spark_chars[(norm * 7.0).round() as usize]
                }).collect();
                let wr_lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Win Rate Trend ", Style::default().fg(C_GRAY)),
                        Span::styled(format!("(latest: {:.0}%)", latest), Style::default().fg(wr_color)),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(spark, Style::default().fg(trend_color)),
                    ]),
                    Line::from(vec![
                        Span::styled(format!("  {:.0}%", wr_series.first().copied().unwrap_or(0.0)), Style::default().fg(C_GRAY)),
                        Span::styled(" ←older   newer→ ", Style::default().fg(Color::Rgb(148, 163, 184))),
                        Span::styled(format!("{:.0}%", latest), Style::default().fg(wr_color)),
                    ]),
                ];
                f.render_widget(Paragraph::new(wr_lines), chunks[3]);
            }
        }

        // M9: DTE@entry vs ROC% scatter
        if !collapsed[3] && perf.dte_roc_scatter.len() >= 3 {
            let pts = &perf.dte_roc_scatter;
            let plot_w = (content_area.width as usize).saturating_sub(10).clamp(20, 80);
            let plot_h = 9usize;
            let max_dte = pts.iter().map(|(d,_,_)| *d).max().unwrap_or(60).max(30) as f64;
            let min_roc = pts.iter().map(|(_,r,_)| *r).fold(f64::INFINITY, f64::min).min(-5.0);
            let max_roc = pts.iter().map(|(_,r,_)| *r).fold(f64::NEG_INFINITY, f64::max).max(5.0);

            // strategy → char + color
            let strat_marker = |s: &StrategyType| -> (char, Color) {
                match s {
                    StrategyType::IronCondor       => ('◆', C_CYAN),
                    StrategyType::IronButterfly    => ('◇', C_CYAN),
                    StrategyType::ShortPutVertical => ('▼', C_GREEN),
                    StrategyType::ShortCallVertical=> ('▲', C_RED),
                    StrategyType::CashSecuredPut   => ('●', Color::Rgb(74, 222, 128)),
                    StrategyType::CoveredCall      => ('○', C_YELLOW),
                    StrategyType::Strangle         => ('■', C_BLUE),
                    StrategyType::Straddle         => ('□', C_BLUE),
                    _                              => ('·', C_GRAY),
                }
            };

            // Build grid: (plot_h rows) × (plot_w cols), each cell = Option<(char, Color)>
            let mut grid: Vec<Vec<Option<(char, Color)>>> = vec![vec![None; plot_w]; plot_h];
            for (dte, roc, strat) in pts {
                let x = (((*dte as f64) / max_dte) * (plot_w - 1) as f64).round() as usize;
                let y_norm = (roc - min_roc) / (max_roc - min_roc);
                let y = ((1.0 - y_norm) * (plot_h - 1) as f64).round() as usize;
                let x = x.min(plot_w - 1);
                let y = y.min(plot_h - 1);
                grid[y][x] = Some(strat_marker(strat));
            }

            let zero_row = {
                let y_norm = (0.0 - min_roc) / (max_roc - min_roc);
                ((1.0 - y_norm) * (plot_h - 1) as f64).round() as usize
            }.min(plot_h - 1);

            let mut scatter_lines: Vec<Line> = Vec::new();
            scatter_lines.push(Line::from(""));
            scatter_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("DTE@Entry vs ROC%  ", Style::default().fg(C_GRAY)),
                Span::styled("◆IC  ▼SPV  ●CSP  ■STR  ▲SCV  ○CC", Style::default().fg(Color::Rgb(100, 116, 139))),
            ]));
            for (ri, row) in grid.iter().enumerate() {
                // Y-axis label
                let y_norm = 1.0 - ri as f64 / (plot_h - 1) as f64;
                let roc_val = min_roc + y_norm * (max_roc - min_roc);
                let y_lbl = format!("{:>+5.0}%│", roc_val);
                let mut spans: Vec<Span> = vec![
                    Span::raw("  "),
                    Span::styled(y_lbl, Style::default().fg(Color::Rgb(100, 116, 139))),
                ];
                for (ci, cell) in row.iter().enumerate() {
                    if let Some((ch, color)) = cell {
                        spans.push(Span::styled(ch.to_string(), Style::default().fg(*color)));
                    } else if ri == zero_row {
                        spans.push(Span::styled("─", Style::default().fg(Color::Rgb(71, 85, 105))));
                    } else {
                        let _ = ci;
                        spans.push(Span::raw(" "));
                    }
                }
                scatter_lines.push(Line::from(spans));
            }
            // X axis
            let x_axis: String = format!("       └{}", "─".repeat(plot_w));
            scatter_lines.push(Line::from(vec![Span::styled(x_axis, Style::default().fg(Color::Rgb(100, 116, 139)))]));
            scatter_lines.push(Line::from(vec![
                Span::styled(format!("        0{:>width$}{:.0}d", "DTE", max_dte as i32, width = plot_w.saturating_sub(4)),
                    Style::default().fg(Color::Rgb(100, 116, 139))),
            ]));
            f.render_widget(Paragraph::new(scatter_lines), chunks[4]);
        }

        // L5: BPR per trade chronologically — position sizing consistency
        if !collapsed[3] && perf.bpr_history.len() >= 3 {
            let bpr = &perf.bpr_history;
            let min_bpr = bpr.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_bpr = bpr.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let avg_bpr = bpr.iter().sum::<f64>() / bpr.len() as f64;
            // Pad y-axis 10% above/below so the line doesn't hug edges
            let y_min = (min_bpr * 0.9).floor();
            let y_max = (max_bpr * 1.1).ceil();

            let bpr_points: Vec<(f64, f64)> = bpr.iter().enumerate()
                .map(|(i, &v)| (i as f64, v))
                .collect();
            let avg_points: Vec<(f64, f64)> = vec![
                (0.0, avg_bpr),
                ((bpr.len() - 1) as f64, avg_bpr),
            ];

            let last_bpr = *bpr.last().unwrap();
            let trend_color = if last_bpr > avg_bpr * 1.2 {
                C_RED    // over-sizing recently
            } else if last_bpr < avg_bpr * 0.8 {
                C_YELLOW // under-sizing recently
            } else {
                C_GREEN  // consistent
            };

            let datasets = vec![
                Dataset::default()
                    .name(format!("BPR/trade  avg ${:.0}", avg_bpr))
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(trend_color))
                    .data(&bpr_points),
                Dataset::default()
                    .name(format!("avg ${:.0}", avg_bpr))
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(Color::Rgb(71, 85, 105)))
                    .data(&avg_points),
            ];

            let bpr_chart = Chart::new(datasets)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(C_BLUE))
                        .title(Span::styled(
                            format!(" 📐 POSITION SIZING  last: ${:.0}  avg: ${:.0}  [{} trades] ",
                                last_bpr, avg_bpr, bpr.len()),
                            Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD),
                        )),
                )
                .x_axis(
                    Axis::default()
                        .title(Span::styled("Trade #", Style::default().fg(C_GRAY)))
                        .bounds([0.0, (bpr.len() - 1) as f64])
                        .labels(vec![
                            Span::styled("1", Style::default().fg(C_GRAY)),
                            Span::styled(format!("{}", bpr.len()), Style::default().fg(C_GRAY)),
                        ]),
                )
                .y_axis(
                    Axis::default()
                        .bounds([y_min, y_max])
                        .labels(vec![
                            Span::styled(format!("${:.0}", y_min), Style::default().fg(C_GRAY)),
                            Span::styled(format!("${:.0}", y_max), Style::default().fg(C_GRAY)),
                        ]),
                );
            f.render_widget(bpr_chart, chunks[5]);
        }

        // L6: Sector Exposure Over Time — heatmap grid (rows=sectors, cols=months)
        if !collapsed[3] && !perf.sector_trends.is_empty() && perf.monthly_pnl.len() >= 2 {
            let months = &perf.monthly_pnl;
            let trends = &perf.sector_trends;
            // Show last N months that fit the width (each month col = 3 chars + 1 sep)
            let avail_w = chunks[6].width.saturating_sub(18) as usize; // 18 for sector label
            let max_months = (avail_w / 4).max(1).min(months.len());
            let month_slice = &months[months.len().saturating_sub(max_months)..];
            let offset = months.len().saturating_sub(max_months);

            let sector_colors = [
                Color::Rgb(99, 179, 237),   // cyan-blue
                Color::Rgb(154, 205, 150),  // green
                Color::Rgb(250, 176, 5),    // yellow
                Color::Rgb(240, 113, 103),  // red-orange
                Color::Rgb(192, 132, 252),  // purple
                Color::Rgb(251, 146, 60),   // orange
                Color::Rgb(148, 163, 184),  // gray
            ];
            let heat_chars = [' ', '\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}'];

            let mut lines: Vec<Line> = Vec::new();

            // Header: month abbreviations
            let month_names = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
            let mut hdr_spans: Vec<Span> = vec![
                Span::styled(format!("{:<16}", "SECTOR"), Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default()),
            ];
            for mp in month_slice {
                let abbr = month_names[(mp.month as usize).saturating_sub(1).min(11)];
                hdr_spans.push(Span::styled(format!("{:>3} ", abbr), Style::default().fg(C_GRAY)));
            }
            lines.push(Line::from(hdr_spans));

            // Find global max for scaling
            let global_max = trends.iter()
                .flat_map(|t| t.monthly_counts.iter())
                .cloned()
                .max()
                .unwrap_or(1)
                .max(1);

            for (i, trend) in trends.iter().enumerate() {
                let color = sector_colors[i % sector_colors.len()];
                // Truncate sector name to 15 chars
                let name = if trend.sector.len() > 15 {
                    format!("{:.15}", trend.sector)
                } else {
                    trend.sector.clone()
                };
                let mut row_spans: Vec<Span> = vec![
                    Span::styled(format!("{:<16}", name), Style::default().fg(color)),
                    Span::styled(" ", Style::default()),
                ];
                for m_idx in offset..months.len() {
                    let count = trend.monthly_counts.get(m_idx).copied().unwrap_or(0);
                    let heat_idx = if count == 0 {
                        0
                    } else {
                        (1 + (count * 3) / global_max).min(4)
                    };
                    let ch = heat_chars[heat_idx];
                    let cell_str = format!("{}{}{} ", ch, ch, ch);
                    row_spans.push(Span::styled(cell_str, Style::default().fg(color)));
                }
                // Total at end
                row_spans.push(Span::styled(
                    format!(" {:>2}tr", trend.total_trades),
                    Style::default().fg(Color::Rgb(100, 116, 139)),
                ));
                lines.push(Line::from(row_spans));
            }

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_BLUE))
                .title(Span::styled(
                    " SECTOR EXPOSURE  (darker = more trades) ",
                    Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD),
                ));
            let inner = block.inner(chunks[6]);
            f.render_widget(block, chunks[6]);
            f.render_widget(Paragraph::new(lines), inner);
        }
        } // end else (standalone ratatui charts)
    } else {
        // ANALYTICS: all sections scrollable
        let mut lines: Vec<Line> = Vec::new();
        lines.extend(perf_advanced_lines(stats, width, collapsed[2], selected_gi == Some(2)));
        lines.extend(perf_strategy_lines(perf, width, collapsed[4], selected_gi == Some(4)));
        lines.extend(perf_ticker_lines(perf, width, collapsed[5], selected_gi == Some(5)));
        lines.extend(perf_monthly_lines(perf, spy_monthly, width, collapsed[6], selected_gi == Some(6), stats.account_size));
        lines.extend(perf_ivr_lines(perf, width, collapsed[7], selected_gi == Some(7)));
        lines.extend(perf_vix_lines(perf, width, collapsed[8], selected_gi == Some(8)));
        lines.extend(perf_dte_lines(perf, width, collapsed[9], selected_gi == Some(9)));
        lines.extend(perf_ivr_entry_lines(perf, width, collapsed[10], selected_gi == Some(10)));
        lines.extend(perf_pnl_dist_lines(perf, width, collapsed[11], selected_gi == Some(11)));
        lines.extend(perf_held_lines(perf, width, collapsed[12], selected_gi == Some(12)));

        lines.extend(perf_commission_lines(perf, collapsed[13], selected_gi == Some(13)));

        lines.push(Line::from(""));

        let para = Paragraph::new(lines).scroll((analytics_scroll, 0));
        f.render_widget(para, content_area);
    }
}

// ── KPI Popup ─────────────────────────────────────────────────────────────────

fn draw_kpi_popup(f: &mut Frame, area: Rect, stats: &PortfolioStats, perf: &PerformanceStats, max_heat_pct: f64, scroll: u16, max_scroll: &mut u16, default_mgmt_dte: i32) {
    let w: u16 = 82.min(area.width.saturating_sub(4));
    let h: u16 = 42.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let dialog = Rect::new(x, y, w, h);
    f.render_widget(Clear, dialog);

    let bwd_source = if stats.spy_price.is_some() { "live SPY" } else { "no SPY price" };

    let vix_heat_target = stats.vix.map(|v| vix_max_heat(v));
    let heat_line = if let Some(vh) = vix_heat_target {
        format!(
            "${:.0} BPR / ${:.0} acct = {:.1}% alloc. VIX tgt: {:.0}%, eff cap: {:.0}%.",
            stats.total_open_bpr, stats.account_size, stats.alloc_pct, vh, max_heat_pct
        )
    } else {
        format!(
            "${:.0} BPR / ${:.0} acct = {:.1}% allocated (eff cap {:.0}%).",
            stats.total_open_bpr, stats.account_size, stats.alloc_pct, max_heat_pct
        )
    };
    let vix_line = stats.vix.map_or(
        "unavailable  —  <15→25%  15-20→30%  20-30→35%  30-40→40%  ≥40→50%".to_string(),
        |v| {
            let regime = if v >= 40.0 { "EXTREME" } else if v >= 30.0 { "HIGH" } else if v >= 20.0 { "ELEVATED" } else if v >= 15.0 { "NORMAL" } else { "CALM" };
            let vh = vix_max_heat(v);
            format!("{:.1} [{regime}] cap: {:.0}%.  Regimes: <15→25%  15-20→30%  20-30→35%  30-40→40%  ≥40→50%", v, vh)
        },
    );

    let lbl = |s: &'static str| Span::styled(s, Style::default().fg(C_YELLOW));
    let sub = |s: &'static str| Span::styled(s, Style::default().fg(C_GRAY));
    let pad = Span::styled("           ", Style::default().fg(C_YELLOW));

    let lines = vec![
        Line::from(vec![Span::styled("  ◆ KPI Reference  (tastytrade philosophy)", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("  ── SIDEBAR STATS ───────────────────────────────────────────────", Style::default().fg(C_GRAY))]),
        Line::from(vec![
            lbl("  Target    "),
            Span::styled(format!("undefined/defined BPR split = {:.0}/{:.0}  (tgt {:.0}/{:.0})", stats.undefined_risk_pct, stats.defined_risk_pct, stats.target_undefined_pct, 100.0 - stats.target_undefined_pct), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("tastytrade: ~75% undefined-risk (CSP/STR/CC), ~25% defined (verticals/IC)."),
        ]),
        Line::from(vec![
            lbl("  Drift     "),
            Span::styled(format!("how far undefined_pct is from target = {:+.1}%", stats.drift), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: near 0. Green ≤2%. Yellow ≤5%. Red >5% — rebalance risk type."),
        ]),
        Line::from(vec![
            lbl("  Win Rate  "),
            Span::styled(format!("closed winners / total closed = {:.1}%", stats.win_rate * 100.0), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: 60–70% for premium sellers. Higher = discipline. < 50% = edge issue."),
        ]),
        Line::from(vec![
            lbl("  Θ/day     "),
            Span::styled(format!("net daily theta on open positions = ${:+.2}", stats.net_theta), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: positive and growing with position size. Drives theta capture."),
        ]),
        Line::from(vec![
            lbl("  Θ/NetLiq  "),
            Span::styled(
                stats.theta_netliq_ratio.map_or("— (no open positions)".to_string(), |r| format!("{:.3}%  net_theta / account_size × 100", r)),
                Style::default().fg(C_WHITE),
            ),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: 0.1–0.3% daily (tastytrade target). Green=0.1-0.3% Yellow=<0.1% Red=>0.3%."),
        ]),
        Line::from(vec![
            lbl("  β-WΔ      "),
            Span::styled(format!("Σ(δ×β×price/SPY×qty×100) = {:+.1}Δ  [{}]", stats.net_beta_weighted_delta, bwd_source), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: near 0 (delta-neutral). Green ≤5. Yellow ≤15. Red >15."),
        ]),
        Line::from(vec![
            lbl("  Θ/|Δ|     "),
            Span::styled(
                stats.theta_delta_ratio.map_or("— (BWD near zero)".to_string(), |r| format!("{:.2}  theta earned per unit of directional risk", r)),
                Style::default().fg(C_WHITE),
            ),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: ≥1.0 (theta-dominated). 0.5–1.0 balanced. <0.5 = delta-dominated."),
        ]),
        Line::from(vec![
            lbl("  Max DD    "),
            Span::styled(format!("peak-to-trough decline = -${:.0}  ({:.1}%)", stats.max_drawdown, stats.max_drawdown_pct), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: < 5% ideal, < 10% acceptable. Alert at 5%. Cut size if hit."),
        ]),
        Line::from(vec![
            lbl("  Avg ROC   "),
            Span::styled(format!("avg return on capital (BPR) = {:.1}%  per closed trade", stats.avg_roc), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: 5–15% per trade. tastytrade target ~10% per occurrence on BPR."),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("  ── CARDS (TOP ROW) ─────────────────────────────────────────────", Style::default().fg(C_GRAY))]),
        Line::from(vec![
            lbl("  Balance   "),
            Span::styled(format!("account_size + realized_pnl = ${:.2}", stats.balance), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            lbl("  P&L       "),
            Span::styled(format!("realized P&L from closed trades = ${:+.2}", stats.realized_pnl), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            lbl("  R:R       "),
            Span::styled(
                if perf.avg_loss > 0.0 {
                    format!("avg_win / avg_loss = 1:{:.2}  (winners are {:.0}% the size of losers)",
                        perf.avg_win / perf.avg_loss,
                        perf.avg_win / perf.avg_loss * 100.0)
                } else { "— (no losing trades yet)".to_string() },
                Style::default().fg(C_WHITE),
            ),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: ≥ 1.0 means winners ≥ losers. Premium sellers often < 1 but offset by high win rate."),
        ]),
        Line::from(vec![
            lbl("  EV        "),
            Span::styled(
                format!("win_rate × avg_win − loss_rate × avg_loss = ${:+.0}/trade", perf.expected_value),
                Style::default().fg(C_WHITE),
            ),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: positive. Measures mathematical edge per trade. The holy grail of trading."),
        ]),
        Line::from(vec![
            lbl("  Unreal    "),
            Span::styled(format!("est. θ×days×100×qty = ${:+.0}  (theta heuristic, not mark-to-market)", stats.unrealized_pnl), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            lbl("            "),
            Span::styled("Time decay only — excludes directional move", Style::default().fg(C_GRAY)),
        ]),
        Line::from(vec![
            lbl("  BWD Card  "),
            Span::styled("beta-weighted delta of full portfolio (see β-WΔ above)", Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            lbl("  Vega      "),
            Span::styled(format!("net vega {:+.0}V  (/1% IV ≈ ${:+.0}).  Negative = normal for prem. sellers.", stats.net_vega, stats.net_vega), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Yellow = normal short vega. Red = long vega (unusual). LightRed = dangerously short."),
        ]),
        Line::from(vec![
            lbl("  Heat      "),
            Span::styled(heat_line, Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("VIX adaptive: lowers cap in calm markets, raises cap in high-VIX."),
        ]),
        Line::from(vec![
            lbl("  POP       "),
            Span::styled(format!("avg probability of profit = {:.1}%  across {} open positions", stats.avg_pop, stats.open_trades), Style::default().fg(C_WHITE)),
        ]),
        Line::from(vec![
            pad.clone(),
            sub("Best: ≥ 66% avg POP (sell at 1SD or better). Drives long-run win rate."),
        ]),
        Line::from(vec![
            lbl("  VIX       "),
            Span::styled(vix_line, Style::default().fg(C_WHITE)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("  ── RULES ──────────────────────────────────────────────────────", Style::default().fg(C_GRAY))]),
        Line::from(vec![
            Span::styled("  management: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("Close at 50% max profit  ·  Manage at {} DTE  ·  No earnings", default_mgmt_dte), Style::default().fg(C_GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  sizing:     ", Style::default().fg(C_GRAY)),
            Span::styled("Max 5% BPR/trade  ·  Heat < 50% total  ·  Undefined ≤ 75%", Style::default().fg(C_GRAY)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("  ↑/↓ or j/k scroll  ·  i/Esc close", Style::default().fg(C_GRAY))]),
    ];

    let content_lines = lines.len() as u16;
    let inner_h = h.saturating_sub(2);
    *max_scroll = content_lines.saturating_sub(inner_h);

    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_CYAN))
                    .title(Span::styled(" ◆ KPI Definitions  (i to close) ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))),
            ),
        dialog,
    );
}

// ── Performance KPI Info Popup ────────────────────────────────────────────────

fn draw_perf_kpi_popup(f: &mut Frame, area: Rect, scroll: u16, max_scroll: &mut u16) {
    let w: u16 = 84.min(area.width.saturating_sub(4));
    let h: u16 = area.height.saturating_sub(4).max(10);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let dialog = Rect::new(x, y, w, h);
    f.render_widget(Clear, dialog);

    let lbl = |s: &'static str| Span::styled(s, Style::default().fg(C_YELLOW));
    let sub = |s: &'static str| Span::styled(s, Style::default().fg(C_GRAY));
    let sp  = "               ";  // 15-char indent for continuation lines

    let lines: Vec<Line> = vec![
        Line::from(vec![Span::styled("  Performance KPI Guide  (tastytrade premium-selling lens)", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("  \u{2500}\u{2500} RETURNS \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", Style::default().fg(C_GRAY))]),
        Line::from(vec![lbl("  Total P&L    "), Span::styled("Realized P&L from all closed trades. Positive = profitable.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Win Rate     "), Span::styled("60\u{2013}70% ideal for premium sellers. Sell at 50% POP \u{2192} ~60%.", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("Over large sample. Higher = discipline.  Lower = sizing issue.")]),
        Line::from(vec![lbl("  Profit Fctr  "), Span::styled("> 1.5 good  \u{b7}  > 2.0 excellent  \u{b7}  < 1.0 = losing edge", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("Gross wins / gross losses. Drives long-run account growth.")]),
        Line::from(vec![lbl("  Exp. Value   "), Span::styled("Must be > $0/trade. Avg P&L across all closed trades.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Avg Winner   "), Span::styled("Avg profit on winning trades. Larger vs avg loser = better.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Avg Loser    "), Span::styled("Avg loss on losing trades. Smaller absolute value = better.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Avg R:R      "), Span::styled("avg_winner / avg_loser. Premium sellers often < 1.0 \u{2014}", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("offset by high win rate. EV = win_rate\u{00d7}winner \u{2212} loss_rate\u{00d7}loser.")]),
        Line::from(vec![lbl("  Avg ROC      "), Span::styled("5\u{2013}15% per trade on BPR. Target: 10% per occurrence.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Avg Ann. ROC "), Span::styled("25\u{2013}40% annualized target. ROC \u{00d7} (365 / days held).", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Streak       "), Span::styled("Current win/loss streak. MaxWin/MaxLoss = historical best/worst.", Style::default().fg(C_WHITE))]),
        Line::from(""),
        Line::from(vec![Span::styled("  \u{2500}\u{2500} RISK-ADJUSTED \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", Style::default().fg(C_GRAY))]),
        Line::from(vec![lbl("  Sharpe       "), Span::styled("> 1.0 good  \u{b7}  > 2.0 excellent.  Return / total vol.", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("Annualized: (avg \u{2212} rf) / \u{03c3} \u{00d7} \u{221a}252.  Penalizes upside swings.")]),
        Line::from(vec![lbl("  Sortino      "), Span::styled("> 1.5 good  \u{b7}  > 3.0 excellent.  Like Sharpe but only", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("downside volatility. Better for asymmetric premium sellers.")]),
        Line::from(vec![lbl("  Calmar       "), Span::styled("> 1.0 good  \u{b7}  > 3.0 excellent. Annual return / max DD.", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("Rewards high return with low drawdown \u{2014} prem. seller ideal.")]),
        Line::from(vec![lbl("  Max Drawdown "), Span::styled("< 5% ideal  \u{b7}  < 10% acceptable. Peak-to-trough decline.", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("Triggers drawdown alert at 5%. Circuit breaker: cut size.")]),
        Line::from(""),
        Line::from(vec![Span::styled("  \u{2500}\u{2500} MANAGEMENT \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", Style::default().fg(C_GRAY))]),
        Line::from(vec![lbl("  DTE at Close "), Span::styled("14\u{2013}21d target. Closing at 21 DTE avoids gamma risk.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Avg Held     "), Span::styled("15\u{2013}30d typical for 45-DTE entries closed at 50% profit.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  % Max Cap.   "), Span::styled("\u{2265} 50% target (the 50% profit rule). < 30% = holding too long.", Style::default().fg(C_WHITE))]),
        Line::from(vec![lbl("  Prem. Recap. "), Span::styled("\u{2265} 50% target. Theta captured vs max potential at close.", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("Low recapture = leaving money on table or closing winners late.")]),
        Line::from(vec![lbl("  Trade Cadence"), Span::styled("Per Week / Per Month \u{2014} frequency of trade entries.", Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::raw(sp), sub("tastytrade: mechanical entries; size + frequency manage risk.")]),
        Line::from(""),
        Line::from(vec![Span::styled("  \u{2191}/\u{2193} or j/k scroll  \u{b7}  i/Esc close", Style::default().fg(Color::Rgb(51, 65, 85)))]),
    ];

    let content_lines = lines.len() as u16;
    let inner_h = h.saturating_sub(2);
    *max_scroll = content_lines.saturating_sub(inner_h);

    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_CYAN))
                    .title(Span::styled(" \u{2605} Performance KPI Reference  (i to close) ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))),
            ),
        dialog,
    );
}

// ── Admin Settings ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_admin(
    f:               &mut Frame,
    area:            Rect,
    app_mode:        AppMode,
    admin_fields:    &[EditField],
    admin_field_idx: usize,
    admin_scroll:    u16,
    stats:           &PortfolioStats,
    max_heat_pct:    f64,
    stored_heat_ceiling: f64,
    max_pos_bpr_pct: f64,
    default_mgmt_dte: i32,
    export_status:   Option<&str>,
) {
    if app_mode == AppMode::AdminSettings && !admin_fields.is_empty() {
        // Show editable form
        let title = " ✦ Admin — Risk Management Settings  (Ctrl+S:Save  Esc:Cancel) ".to_string();
        let mut lines: Vec<Line> = Vec::new();
        let mut field_line_starts: Vec<usize> = Vec::with_capacity(admin_fields.len());

        for (i, field) in admin_fields.iter().enumerate() {
            if let Some(hdr) = &field.section_header {
                if !hdr.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled(hdr.clone(), Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD)),
                    ]));
                }
            }
            field_line_starts.push(lines.len());
            let is_focused = i == admin_field_idx;
            let (lbl_style, val_style) = if is_focused {
                (Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD), Style::default().bg(Color::Rgb(30,58,138)).fg(C_WHITE))
            } else {
                (Style::default().fg(C_GRAY), Style::default().fg(C_WHITE))
            };
            let cursor = if is_focused { "▌" } else { " " };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:24}", field.label), lbl_style),
                Span::styled(format!("{}{}", field.value, cursor), val_style),
            ]));
        }

        let line_scroll = field_line_starts.get(admin_scroll as usize).copied().unwrap_or(0) as u16;
        f.render_widget(
            Paragraph::new(lines)
                .scroll((line_scroll, 0))
                .block(Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(C_YELLOW))
                    .title(Span::styled(title, Style::default().fg(C_YELLOW)))),
            area,
        );
    } else {
        // Show read-only current settings + instructions
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled("  ✦ Risk Management Settings", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("  Account Size         ", Style::default().fg(C_GRAY)), Span::styled(format!("${:.2}", stats.account_size), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("  Max Heat %           ", Style::default().fg(C_GRAY)), Span::styled(format!("stored: {:.1}%  effective: {:.1}%  (current: {:.1}%)", stored_heat_ceiling, max_heat_pct, stats.alloc_pct), Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  Max Pos BPR %        ", Style::default().fg(C_GRAY)), Span::styled(format!("{:.1}%", max_pos_bpr_pct), Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  Target Undefined %   ", Style::default().fg(C_GRAY)), Span::styled(format!("{:.1}%  (current: {:.1}%)", stats.target_undefined_pct, stats.undefined_risk_pct), Style::default().fg(C_WHITE))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── tastytrade defaults ────────────────────────────────────────", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Max Heat            ", Style::default().fg(C_GRAY)), Span::styled("50% of account in BPR", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Undefined Risk      ", Style::default().fg(C_GRAY)), Span::styled("75% of total BPR", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Max Per Position    ", Style::default().fg(C_GRAY)), Span::styled("5% BPR of account", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Entry DTE Range     ", Style::default().fg(C_GRAY)), Span::styled("21–60 DTE", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Min POP             ", Style::default().fg(C_GRAY)), Span::styled("60% probability of profit", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Profit Target       ", Style::default().fg(C_GRAY)), Span::styled("50% of max profit (GTC order)", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Management          ", Style::default().fg(C_GRAY)), Span::styled(format!("Roll or close at {} DTE", default_mgmt_dte), Style::default().fg(C_GRAY))]),
            Line::from(""),
            Line::from(vec![Span::styled("  Press E to edit settings  Shift+E to export CSV", Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD))]),
        ];
        if let Some(msg) = export_status {
            lines.push(Line::from(""));
            let msg_color = if msg.starts_with('✓') { C_GREEN } else { C_RED };
            lines.push(Line::from(vec![Span::styled(format!("  {}", msg), Style::default().fg(msg_color))]));
        }
        f.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL)
                    .border_style(Style::default().fg(C_BLUE))
                    .title(Span::styled(" ✦ Admin ", Style::default().fg(C_CYAN)))),
            area,
        );
    }
}

// ── Daily Actions ─────────────────────────────────────────────────────────────

fn draw_daily_actions(
    f:          &mut Frame,
    area:       Rect,
    alerts:     &[crate::actions::TradeAlert],
    list_state: &mut ListState,
    collapsed:  &HashSet<crate::actions::AlertKind>,
    pulse_on:   bool,
    stats:      &crate::models::PortfolioStats,
) {
    use crate::actions::{AlertKind, ActionRow, build_action_rows};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // stats bar + plan summary
            Constraint::Length(7),  // morning checklist
            Constraint::Min(0),     // alert list
        ])
        .split(area);

    // ── Stats bar: count by kind ──────────────────────────────────────────────
    let defense = alerts.iter().filter(|a| matches!(a.kind, AlertKind::Defense | AlertKind::MaxLoss | AlertKind::Drawdown)).count();
    let warning = alerts.iter().filter(|a| a.kind == AlertKind::Warning).count();
    let manage  = alerts.iter().filter(|a| a.kind == AlertKind::Manage || a.kind == AlertKind::Roll).count();
    let close   = alerts.iter().filter(|a| a.kind == AlertKind::Close).count();
    let total   = alerts.iter().filter(|a| a.kind != AlertKind::Ok).count();

    // ── "Today's Plan" single-line summary ───────────────────────────────────
    let plan_str = if total == 0 {
        "  ✓ All clear. No positions require action today.".to_string()
    } else {
        let mut parts: Vec<String> = Vec::new();
        if defense > 0 { parts.push(format!("{} defense", defense)); }
        if warning > 0 { parts.push(format!("{} earnings warning", warning)); }
        if manage  > 0 { parts.push(format!("{} to manage/roll", manage)); }
        if close   > 0 { parts.push(format!("{} near profit target", close)); }
        format!("  Today: {}", parts.join(" · "))
    };
    let plan_color = if defense > 0 { C_RED } else if warning > 0 || manage > 0 { C_YELLOW } else { C_GREEN };

    let stats_line = Line::from(vec![
        Span::styled("  ⚡ Daily Actions  ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(C_DARK)),
        Span::styled(format!(" {} DEFENSE ", defense), Style::default().fg(C_RED).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled(format!(" {} WARNING ", warning), Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled(format!(" {} MANAGE ", manage), Style::default().fg(C_YELLOW)),
        Span::styled("  ", Style::default()),
        Span::styled(format!(" {} CLOSE ", close), Style::default().fg(C_GREEN)),
    ]);
    let plan_line = Line::from(vec![
        Span::styled(plan_str, Style::default().fg(plan_color)),
    ]);
    f.render_widget(
        Paragraph::new(vec![stats_line, Line::from(""), plan_line])
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_DARK))),
        chunks[0],
    );

    // ── Morning Checklist ─────────────────────────────────────────────────────
    {
        use crate::actions::AlertKind;
        let bwd     = stats.net_beta_weighted_delta;
        let vix     = stats.vix;
        let bwd_ok  = bwd.abs() < 30.0;
        let vix_ok  = vix.map(|v| v < 30.0).unwrap_or(true);
        let earnings_alerts = alerts.iter().filter(|a| a.kind == AlertKind::Warning).count();

        let bwd_color    = if bwd_ok { C_GREEN } else { C_YELLOW };
        let vix_color    = if vix_ok { C_GREEN } else if vix.map(|v| v < 40.0).unwrap_or(true) { C_YELLOW } else { C_RED };
        let earn_color   = if earnings_alerts == 0 { C_GREEN } else { C_YELLOW };

        let vix_str = vix.map(|v| format!("{:.1}", v)).unwrap_or_else(|| "N/A".to_string());
        let regime_label = vix.map(|v| {
            if v < 15.0 { "Calm" } else if v < 20.0 { "Normal" } else if v < 30.0 { "Elevated" } else if v < 40.0 { "High" } else { "Stress" }
        }).unwrap_or("Unknown");

        let chk_lines = vec![
            Line::from(vec![
                Span::styled(if bwd_ok { "  ✓" } else { "  !" }, Style::default().fg(bwd_color).add_modifier(Modifier::BOLD)),
                Span::styled(" Portfolio Delta:  ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:+.1} BWD", bwd), Style::default().fg(bwd_color)),
                Span::styled(if bwd_ok { "  (neutral)" } else { "  (review)" }, Style::default().fg(C_GRAY)),
            ]),
            Line::from(vec![
                Span::styled(if vix_ok { "  ✓" } else { "  !" }, Style::default().fg(vix_color).add_modifier(Modifier::BOLD)),
                Span::styled(" VIX Regime:       ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{} — {}", vix_str, regime_label), Style::default().fg(vix_color)),
            ]),
            Line::from(vec![
                Span::styled(if earnings_alerts == 0 { "  ✓" } else { "  !" }, Style::default().fg(earn_color).add_modifier(Modifier::BOLD)),
                Span::styled(" Earnings Scan:    ", Style::default().fg(C_GRAY)),
                Span::styled(
                    if earnings_alerts == 0 { "No earnings exposure".to_string() } else { format!("{} positions near earnings", earnings_alerts) },
                    Style::default().fg(earn_color),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ·", Style::default().fg(C_GRAY)),
                Span::styled(" Positions open:   ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{}", stats.open_trades), Style::default().fg(C_WHITE)),
                Span::styled("   Theta/day: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:+.2}", stats.net_theta), Style::default().fg(if stats.net_theta >= 0.0 { C_GREEN } else { C_YELLOW })),
            ]),
        ];
        f.render_widget(
            Paragraph::new(chk_lines)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_DARK))
                    .title(Span::styled(" Morning Checklist ", Style::default().fg(C_CYAN)))),
            chunks[1],
        );
    }

    // ── All-clear panel when no alerts ───────────────────────────────────────
    if total == 0 {
        let avg_dte_str = if let Some(pos) = stats.next_critical_positions.first() {
            format!("Nearest DTE: {}d", pos.1)
        } else {
            "No open positions".to_string()
        };
        let bwd_str = format!("BWD: {:+.1}", stats.net_beta_weighted_delta);
        let bwd_color = if stats.net_beta_weighted_delta.abs() < 30.0 { C_GREEN } else { C_YELLOW };
        let clear_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  ✓ PORTFOLIO CLEAR — No Actions Required",
                    Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("Positions: {}  ", stats.open_trades), Style::default().fg(C_WHITE)),
                Span::styled(avg_dte_str + "  ", Style::default().fg(C_CYAN)),
                Span::styled(bwd_str, Style::default().fg(bwd_color)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Theta/day: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:+.2}  ", stats.net_theta), Style::default().fg(if stats.net_theta >= 0.0 { C_GREEN } else { C_YELLOW })),
                Span::styled("Alloc: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("{:.1}%", stats.alloc_pct), Style::default().fg(C_YELLOW)),
            ]),
        ];
        f.render_widget(
            Paragraph::new(clear_lines)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_GREEN))),
            chunks[2],
        );
        return;
    }

    // ── Alert list — collapsible groups ───────────────────────────────────────
    let rows = build_action_rows(alerts, collapsed);
    let items: Vec<ListItem> = rows.iter().map(|row| {
        match row {
            ActionRow::GroupHeader { kind, count } => {
                let is_collapsed = collapsed.contains(kind);
                let toggle = if is_collapsed { "▶" } else { "▼" };
                let (color, label) = match kind {
                    AlertKind::Defense        => (C_RED,           "DEFENSE"),
                    AlertKind::MaxLoss        => (C_RED,           "MAXLOSS"),
                    AlertKind::GammaRisk      => (C_RED,           "GAMMA"),
                    AlertKind::Warning        => (C_YELLOW,        "WARNING"),
                    AlertKind::DeltaExtreme   => (C_YELLOW,        "Δ EXTREME"),
                    AlertKind::UndefinedDrift => (C_YELLOW,        "DRIFT"),
                    AlertKind::Drawdown       => (C_RED,           "DRAWDOWN"),
                    AlertKind::RollChain      => (C_YELLOW,        "ROLLS"),
                    AlertKind::Manage         => (C_YELLOW,        "MANAGE"),
                    AlertKind::Close          => (C_GREEN,         "CLOSE"),
                    AlertKind::Roll           => (C_BLUE,          "ROLL"),
                    AlertKind::Sizing         => (Color::Magenta,  "SIZING"),
                    AlertKind::Ok             => (C_GRAY,          "OK"),
                };
                let suffix = if *count == 1 { "alert" } else { "alerts" };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", toggle), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} ", label), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("({} {})", count, suffix), Style::default().fg(C_GRAY)),
                ]))
            }
            ActionRow::Alert(alert) => {
                let badge_color = match alert.kind {
                    AlertKind::Defense        => C_RED,
                    AlertKind::MaxLoss        => C_RED,
                    AlertKind::GammaRisk      => C_RED,
                    AlertKind::Warning        => C_YELLOW,
                    AlertKind::DeltaExtreme   => C_YELLOW,
                    AlertKind::UndefinedDrift => C_YELLOW,
                    AlertKind::Drawdown       => C_RED,
                    AlertKind::RollChain      => C_YELLOW,
                    AlertKind::Manage         => C_YELLOW,
                    AlertKind::Close          => C_GREEN,
                    AlertKind::Roll           => C_BLUE,
                    AlertKind::Sizing         => Color::Magenta,
                    AlertKind::Ok             => C_GRAY,
                };
                let badge_style = if matches!(alert.kind, AlertKind::Defense | AlertKind::MaxLoss | AlertKind::GammaRisk | AlertKind::Drawdown) {
                    if pulse_on {
                        Style::default().fg(Color::White).bg(C_RED).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(C_RED).add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default().fg(badge_color).add_modifier(Modifier::BOLD)
                };
                let kind_label = alert.kind.badge();
                let mut lines = vec![
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(format!(" [{}] ", kind_label), badge_style),
                        Span::styled(alert.headline.clone(), Style::default().fg(C_WHITE)),
                    ]),
                ];
                if let Some(ref d) = alert.detail {
                    lines.push(Line::from(vec![
                        Span::raw("           "),
                        Span::styled(format!("↳ {}", d), Style::default().fg(C_GRAY)),
                    ]));
                }
                ListItem::new(lines)
            }
        }
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Daily Alerts ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))))
        .highlight_style(Style::default().bg(C_DARK).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[2], list_state);
}

// ── Journal Help Popup ────────────────────────────────────────────────────────

fn draw_journal_help_popup(f: &mut Frame, area: Rect, scroll: u16, max_scroll: &mut u16, page: u8) {
    let w: u16 = 76.min(area.width.saturating_sub(4));
    let h: u16 = 40.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let dialog = Rect::new(x, y, w, h);
    f.render_widget(Clear, dialog);

    let def = |col: &'static str, desc: &'static str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<12}", col), Style::default().fg(C_YELLOW)),
            Span::styled(desc, Style::default().fg(C_WHITE)),
        ])
    };

    let lines: Vec<Line> = if page == 0 {
        vec![
            Line::from(vec![Span::styled("  Journal Keyboard Shortcuts  ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)), Span::styled("Tab → Field Glossary", Style::default().fg(C_GRAY))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Navigation ──────────────────────────────────────────", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  ↑/↓        ", Style::default().fg(C_YELLOW)), Span::styled("Move selection up/down", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  PgUp/PgDn  ", Style::default().fg(C_YELLOW)), Span::styled("Page up/down (10 rows)", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  Home/End   ", Style::default().fg(C_YELLOW)), Span::styled("Jump to first/last trade", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  Enter      ", Style::default().fg(C_YELLOW)), Span::styled("Toggle trade detail panel", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  Tab        ", Style::default().fg(C_YELLOW)), Span::styled("Switch help page (Shortcuts ↔ Field Glossary)", Style::default().fg(C_WHITE))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Filtering & Sorting ─────────────────────────────────", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  f          ", Style::default().fg(C_YELLOW)), Span::styled("Cycle filter: All/Open/Closed/Rolled/Expired", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  /          ", Style::default().fg(C_YELLOW)), Span::styled("Search by ticker", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  s          ", Style::default().fg(C_YELLOW)), Span::styled("Cycle sort key (Date/Ticker/P&L/DTE/Status)", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  S          ", Style::default().fg(C_YELLOW)), Span::styled("Toggle sort direction (asc/desc)", Style::default().fg(C_WHITE))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Trade Actions ───────────────────────────────────────", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  e          ", Style::default().fg(C_YELLOW)), Span::styled("Edit selected trade", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  n          ", Style::default().fg(C_YELLOW)), Span::styled("New trade", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  c/C        ", Style::default().fg(C_YELLOW)), Span::styled("Close selected trade", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  x          ", Style::default().fg(C_YELLOW)), Span::styled("Delete selected trade (with confirm)", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  a          ", Style::default().fg(C_YELLOW)), Span::styled("Analyze (payoff chart)", Style::default().fg(C_WHITE))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Display ─────────────────────────────────────────────", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  v          ", Style::default().fg(C_YELLOW)), Span::styled("Column visibility picker", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  G          ", Style::default().fg(C_YELLOW)), Span::styled("Toggle Chain View (group by ticker)", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  R          ", Style::default().fg(C_YELLOW)), Span::styled("Refresh data from database", Style::default().fg(C_WHITE))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Trade Statuses ──────────────────────────────────────", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  OPEN       ", Style::default().fg(C_GREEN)), Span::styled("Active position, not yet closed", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  CLOSED     ", Style::default().fg(C_GRAY)), Span::styled("Position closed, P&L realized", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  ROLLED     ", Style::default().fg(C_CYAN)), Span::styled("Position rolled to a new expiration", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  SCRATCH    ", Style::default().fg(C_YELLOW)), Span::styled("Closed near breakeven (< $10)", Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  EXPIRED    ", Style::default().fg(C_GRAY)), Span::styled("Expired worthless (full profit)", Style::default().fg(C_WHITE))]),
            Line::from(""),
            Line::from(vec![Span::styled("  i/Esc:Close  Tab:Field Glossary →", Style::default().fg(C_GRAY))]),
        ]
    } else {
        vec![
            Line::from(vec![Span::styled("  Field Glossary  ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)), Span::styled("Tab → Keyboard Shortcuts", Style::default().fg(C_GRAY))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Entry Fields ────────────────────────────────────────", Style::default().fg(C_GRAY))]),
            def("Date",         "Trade entry date"),
            def("Ticker",       "Underlying symbol (e.g. SPY, AAPL)"),
            def("Spot",         "Underlying price at entry"),
            def("ER",           "Earnings Release date — [!] means ER within expiration"),
            def("Str",          "Strategy type: IC, SPV, SCV, CSP, CC, CAL, etc."),
            def("Qty",          "Number of contracts"),
            def("Credit",       "Net premium collected (or debit paid, shown negative)"),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Risk & Targets ──────────────────────────────────────", Style::default().fg(C_GRAY))]),
            def("GTC",          "Good Till Cancel order price — profit target on the books"),
            def("  $2.45DB",    "Buy-to-close debit at target (short premium position)"),
            def("  $2.45CR",    "Sell-to-close credit at target (long/debit position)"),
            def("BE",           "Breakeven price(s) at expiration"),
            def("BPR",          "Buying Power Reduction — margin held by broker"),
            def("BPR%",         "BPR as % of account size (position sizing check)"),
            def("MaxPft",       "Maximum profit if all legs expire worthless"),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Performance ─────────────────────────────────────────", Style::default().fg(C_GRAY))]),
            def("P&L",          "Realized P&L (closed) or unrealized estimate (open)"),
            def("ROC%",         "Return on Capital = P&L ÷ BPR × 100"),
            def("$V/d",         "Dollar Theta — premium decay per calendar day"),
            def("DTE",          "Days To Expiration remaining from today"),
            def("Exit",         "Date position was closed or rolled"),
            def("Held",         "Calendar days the position was held"),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Market Context ──────────────────────────────────────", Style::default().fg(C_GRAY))]),
            def("OTM%",         "How far the short strike is Out-of-The-Money"),
            def("EM",           "Expected Move ±1SD: underlying × (IV/100) × √(DTE/365)"),
            Line::from(""),
            Line::from(vec![Span::styled("  ── Strategy Abbreviations ──────────────────────────────", Style::default().fg(C_GRAY))]),
            def("IC",           "Iron Condor — sell OTM call spread + put spread"),
            def("SPV",          "Short Put Vertical (bull put spread)"),
            def("SCV",          "Short Call Vertical (bear call spread)"),
            def("CSP",          "Cash Secured Put — sell naked put"),
            def("CC",           "Covered Call — sell call against long stock"),
            def("CAL",          "Calendar Spread — sell near, buy far expiration"),
            def("LDS",          "Long Diagonal Spread"),
            Line::from(""),
            Line::from(vec![Span::styled("  i/Esc:Close  Tab:Shortcuts ←", Style::default().fg(C_GRAY))]),
        ]
    };

    let title = if page == 0 {
        " ? Journal Help  [1/2]  (Tab: Field Glossary) "
    } else {
        " ? Field Glossary  [2/2]  (Tab: Shortcuts) "
    };

    let content_lines = lines.len() as u16;
    let inner_h = h.saturating_sub(2);
    *max_scroll = content_lines.saturating_sub(inner_h);

    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_CYAN))
                    .title(Span::styled(title, Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))),
            ),
        dialog,
    );
}
