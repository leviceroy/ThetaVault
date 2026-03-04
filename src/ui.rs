use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState,
        Paragraph, Row, Table, TableState, Tabs, Wrap,
    },
    Frame,
};
use crate::models::{LegType, PlaybookStrategy, PerformanceStats, PortfolioStats, Trade};
use crate::calculations::{
    calculate_breakevens, calculate_calendar_payoff_at_price, calculate_held_duration,
    calculate_max_loss_from_legs, calculate_max_profit, calculate_payoff_at_price,
    calculate_pct_max_profit, calculate_roc, calculate_remaining_dte, estimate_pop,
    format_trade_description,
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
    perf_scroll:      u16,
    playbooks:        &[PlaybookStrategy],
    selected_tab:     usize,
    table_state:      &mut TableState,
    playbook_state:   &mut ListState,
    thesis_scroll:    u16,
    show_detail:      bool,
    detail_scroll:    u16,
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
    dash_kpi_popup:          bool,
    max_heat_pct:            f64,
    admin_fields:            &[EditField],
    admin_field_idx:         usize,
    admin_scroll:            u16,
    cal_year:                i32,
    cal_month:               u32,
    cal_day:                 u32,
    thesis_edit_buf:         &str,
    spy_monthly:             &std::collections::HashMap<(i32, u32), f64>,
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
        0 => draw_dashboard(f, chunks[1], stats, all_trades, dash_open_scroll, max_heat_pct),
        1 => draw_journal(
            f, chunks[1],
            display_count, visual_rows, collapsed_months, all_trades,
            table_state,
            show_detail, detail_scroll,
            filter_status, filter_ticker,
            sort_key, sort_desc,
            app_mode,
            edit_fields, edit_field_idx, edit_scroll,
            close_fields, close_field_idx,
        ),
        2 => draw_playbook(
            f, chunks[1], playbooks, playbook_state, thesis_scroll,
            app_mode, playbook_edit_fields, playbook_edit_field_idx, playbook_edit_scroll,
            thesis_edit_buf, perf_stats,
        ),
        3 => draw_daily_actions(f, chunks[1], alerts, actions_list_state, collapsed_action_kinds, pulse_on),
        4 => draw_admin(f, chunks[1], app_mode, admin_fields, admin_field_idx, admin_scroll, stats, max_heat_pct),
        5 => draw_performance(f, chunks[1], stats, perf_stats, perf_scroll, spy_monthly),
        _ => {}
    }

    // ── Footer — changes per mode
    let footer_text = match (selected_tab, app_mode) {
        (1, AppMode::FilterInput)   => " Esc:Done  Backspace:Del  (type ticker to filter) ",
        (1, AppMode::EditTrade)     => " ↑↓/Tab:Field  +/-:Cycle  Ctrl+S:Save  Esc:Cancel  Ctrl+A:AddLeg  Ctrl+D:DelLeg  Enter:Button ",
        (1, AppMode::CloseTrade)    => " ↑↓/Tab:Field  Ctrl+S:Save  Esc:Cancel ",
        (1, AppMode::ConfirmDelete) => " Y/Enter:Confirm Delete  Any other key:Cancel ",
        (1, AppMode::AnalyzeTrade)  => " Esc:Close  (Payoff at Expiration — ASCII chart) ",
        (_, AppMode::DatePicker)    => " ←→:Day  ↑↓:Week  [/]:Month  Enter:Confirm  Esc:Cancel ",
        (1, _) => " Q:Quit  ↑↓:Nav  Enter:Detail  f:Filter  /:Search  s:Sort  e:Edit  c:Close  a:Analyze  x:Del  R:Refresh ",
        (0, _) => " Q:Quit  Tab:Switch  ↑↓:Scroll  i:KPI Info  R:Refresh ",
        (2, AppMode::EditThesis)   => " Type to edit  Enter:Newline  Backspace:Del  Ctrl+S:Save  Esc:Cancel ",
        (2, AppMode::EditPlaybook) => " ↑↓/Tab:Field  +/-:Cycle  Ctrl+S:Save  Esc:Cancel ",
        (2, _)                     => " Q:Quit  Tab:Switch  ↑↓:Select  ↕:Scroll  N:New  E:Edit  T:Edit Thesis ",
        (3, _)                     => " Q:Quit  ↑↓:Nav  Enter:Collapse/→Journal  R:Refresh ",
        (4, AppMode::AdminSettings) => " ↑↓/Tab:Field  Ctrl+S:Save  Esc:Cancel ",
        (4, _)                     => " Q:Quit  E:Edit Settings  R:Refresh ",
        (5, _)                     => " Q:Quit  ↑↓/j/k:Scroll  Tab:Switch  R:Refresh ",
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
        draw_kpi_popup(f, chunks[1], stats, max_heat_pct);
    }

    // ── Date picker overlay — always on top
    if app_mode == AppMode::DatePicker {
        draw_date_picker(f, chunks[1], cal_year, cal_month, cal_day);
    }
}

// ── Dashboard ────────────────────────────────────────────────────────────────

fn draw_dashboard(f: &mut Frame, area: Rect, stats: &PortfolioStats, trades: &[Trade], open_scroll: usize, max_heat_pct: f64) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(0)])
        .split(area);

    // ── Row 1: 8 KPI cards ────────────────────────────────────────────────────
    let kpi = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
        ])
        .split(rows[0]);

    // Card 1 — Balance
    let bal_color = if stats.balance >= stats.account_size { C_GREEN } else { C_RED };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" ${:.0}", stats.balance),
                Style::default().fg(bal_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(" acct ${:.0}", stats.account_size),
                Style::default().fg(C_GRAY),
            )]),
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Balance ", Style::default().fg(C_CYAN)))),
        kpi[0],
    );

    // Card 2 — P&L (realized)
    let pnl_color = if stats.realized_pnl >= 0.0 { C_GREEN } else { C_RED };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" ${:+.0}", stats.realized_pnl),
                Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(" {:.1}% WR", stats.win_rate * 100.0),
                Style::default().fg(C_GRAY),
            )]),
        ])
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
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {}Δ", bwd_str),
                Style::default().fg(bwd_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(" β-wtd delta", Style::default().fg(C_GRAY))]),
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" BWD ", Style::default().fg(C_CYAN)))),
        kpi[3],
    );

    // Card 5 — Heat (BPR/Account × 100, colored vs max_heat_pct)
    let heat_color = if stats.alloc_pct >= max_heat_pct { C_RED }
        else if stats.alloc_pct >= max_heat_pct * 0.75 { C_YELLOW }
        else { C_GREEN };
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
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Heat ", Style::default().fg(C_CYAN)))),
        kpi[4],
    );

    // Card 6 — Risk (undefined%/defined% split as "78/22")
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
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" Risk ", Style::default().fg(C_CYAN)))),
        kpi[5],
    );

    // Card 7 — VIX
    let vix_str   = stats.vix.map_or("—".to_string(), |v| format!("{:.2}", v));
    let vix_color = stats.vix.map_or(C_GRAY, |v| {
        if v > 30.0 { C_RED } else if v > 20.0 { C_YELLOW } else { C_GREEN }
    });
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {}", vix_str),
                Style::default().fg(vix_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(" env fear", Style::default().fg(C_GRAY))]),
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" VIX ", Style::default().fg(C_CYAN)))),
        kpi[6],
    );

    // Card 8 — POP
    let pop_color = if stats.avg_pop >= 68.0 { C_GREEN }
        else if stats.avg_pop >= 50.0 { C_YELLOW }
        else { C_RED };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(" {:.1}%", stats.avg_pop),
                Style::default().fg(pop_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(" {} open", stats.open_trades),
                Style::default().fg(C_GRAY),
            )]),
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" POP ", Style::default().fg(C_CYAN)))),
        kpi[7],
    );

    // ── Row 2: Risk panel | Open positions + Equity ───────────────────────────
    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(rows[1]);

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

    let risk_lines = vec![
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
            Span::styled(format!("${:.2}", stats.net_theta), Style::default().fg(C_GREEN)),
        ]),
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

    f.render_widget(
        Paragraph::new(risk_lines)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
                .title(Span::styled(" Risk Distribution ", Style::default().fg(C_CYAN)))),
        bot[0],
    );

    // ── Right: Open positions (top) + Equity sparkline (bottom)
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(bot[1]);

    let open_trades: Vec<&Trade> = trades.iter().filter(|t| t.is_open()).collect();
    let today      = Utc::now().date_naive();
    let c_orange   = Color::Rgb(249, 115, 22);

    let open_header = Row::new(vec![
        Cell::from("Ticker").style(Style::default().fg(C_CYAN)),
        Cell::from("Opened").style(Style::default().fg(C_CYAN)),
        Cell::from("Str").style(Style::default().fg(C_CYAN)),
        Cell::from("Credit").style(Style::default().fg(C_CYAN)),
        Cell::from("DTE").style(Style::default().fg(C_CYAN)),
        Cell::from("ER").style(Style::default().fg(C_CYAN)),
        Cell::from("BPR").style(Style::default().fg(C_CYAN)),
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
        let bpr_str  = t.bpr.map_or("—".to_string(), |b| format!("${:.0}", b));
        let cr_color = if t.credit_received >= 0.0 { C_GREEN } else { C_RED };
        Row::new(vec![
            Cell::from(t.ticker.clone()).style(Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
            Cell::from(t.trade_date.format("%m/%d/%y").to_string()).style(Style::default().fg(C_GRAY)),
            Cell::from(t.strategy.badge()).style(Style::default().fg(badge_color(t.spread_type()))),
            Cell::from(format!("${:.2}", t.credit_received)).style(Style::default().fg(cr_color)),
            Cell::from(format!("{}", dte)).style(Style::default().fg(dte_c)),
            Cell::from(er_str).style(er_style),
            Cell::from(bpr_str).style(Style::default().fg(C_YELLOW)),
        ])
    }).collect();

    let mut open_table_state = TableState::default();
    *open_table_state.offset_mut() = open_scroll.min(open_trades.len().saturating_sub(1));
    f.render_stateful_widget(
        Table::new(open_rows, [
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(8),
        ])
        .header(open_header)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(
                format!(" Open Positions ({}) ↑↓ ", open_trades.len()),
                Style::default().fg(C_CYAN),
            ))),
        right_rows[0],
        &mut open_table_state,
    );

    let closed: Vec<&Trade> = trades.iter().filter(|t| t.pnl.is_some()).collect();
    let spark = build_equity_sparkline(&closed, right_rows[1].width as usize - 4);
    f.render_widget(
        Paragraph::new(spark)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BLUE))
                .title(Span::styled(" Equity Curve ", Style::default().fg(C_CYAN))))
            .wrap(Wrap { trim: false }),
        right_rows[1],
    );
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
) {
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
            VisualRowKind::YearHeader { .. } | VisualRowKind::MonthHeader { .. } => None,
        }
    };

    match app_mode {
        AppMode::AnalyzeTrade => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(main_area);
            draw_trade_table(f, split[0], visual_rows, collapsed_months, all_trades, table_state);
            if let Some(trade) = ctx_trade(table_state) {
                draw_analyze_pane(f, split[1], trade);
            }
        }
        AppMode::EditTrade => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(main_area);
            draw_trade_table(f, split[0], visual_rows, collapsed_months, all_trades, table_state);
            let ctx = ctx_trade(table_state);
            draw_edit_pane(f, split[1], edit_fields, edit_field_idx, edit_scroll, ctx);
        }
        AppMode::CloseTrade => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(main_area);
            draw_trade_table(f, split[0], visual_rows, collapsed_months, all_trades, table_state);
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

            draw_trade_table(f, table_area, visual_rows, collapsed_months, all_trades, table_state);

            if let Some(det) = detail_area {
                if let Some(trade) = ctx_trade(table_state) {
                    draw_trade_detail(f, det, trade, detail_scroll);
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

    for s in [FilterStatus::All, FilterStatus::Open, FilterStatus::Closed, FilterStatus::Expired] {
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

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(C_DARK)),
        area,
    );
}

// ── Trade table ───────────────────────────────────────────────────────────────

fn draw_trade_table(
    f:                &mut Frame,
    area:             Rect,
    visual_rows:      &[VisualRowKind],
    collapsed_months: &HashSet<(i32, u32)>,
    all_trades:       &[Trade],
    state:            &mut TableState,
) {
    let header = Row::new(
        ["Date", "Ticker", "ER", "Str", "Qty", "Credit", "GTC", "BE", "MaxPft", "P&L", "ROC%", "DTE", "Exit", "Held", "Status"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))),
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
            VisualRowKind::Trade(ti) => {
                let t = &all_trades[*ti];
        let pnl_str = match t.pnl {
            Some(p) => format!("${:+.2}", p),
            None    => "open".to_string(),
        };
        let pnl_style = match t.pnl {
            Some(p) if p > 0.0 => Style::default().fg(C_GREEN),
            Some(_)             => Style::default().fg(C_RED),
            None                => Style::default().fg(C_YELLOW),
        };
        let max_profit = calculate_max_profit(t.credit_received, t.quantity);
        let roc = t.pnl.and_then(|p| calculate_roc(p, &t.legs, t.credit_received, t.quantity, t.spread_type(), t.bpr));
        let dte = calculate_remaining_dte(&t.expiration_date);
        let dte_c = if t.is_open() {
            if dte <= 14 { C_RED } else if dte <= 21 { C_YELLOW } else { C_GREEN }
        } else {
            C_GRAY
        };

        let status_str = if t.is_open() {
            "OPEN".to_string()
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
            let target = t.target_profit_pct.unwrap_or_else(|| t.strategy.default_profit_target_pct());
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
            let bes = calculate_breakevens(&t.legs, t.spread_type());
            match bes.len() {
                2 => format!("{:.0}/{:.0}", bes[0], bes[1]),
                1 => format!("{:.0}", bes[0]),
                _ => "—".to_string(),
            }
        } else {
            "—".to_string()
        };

                Row::new(vec![
                    Cell::from(t.trade_date.format("%m/%d/%y").to_string()).style(Style::default().fg(C_GRAY)),
                    Cell::from(t.ticker.clone()).style(Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
                    Cell::from(er_str).style(er_style),
                    Cell::from(t.strategy.badge()).style(Style::default().fg(badge_color(t.spread_type()))),
                    Cell::from(t.quantity.to_string()),
                    Cell::from(format!("${:.2}", t.credit_received))
                        .style(Style::default().fg(if t.credit_received >= 0.0 { C_GREEN } else { C_RED })),
                    Cell::from(gtc_str).style(gtc_style),
                    Cell::from(be_str).style(Style::default().fg(C_GRAY)),
                    Cell::from(format!("${:.0}", max_profit)).style(Style::default().fg(C_GRAY)),
                    Cell::from(pnl_str).style(pnl_style),
                    Cell::from(roc.map_or("—".to_string(), |r| format!("{:.1}%", r)))
                        .style(roc.map_or(Style::default().fg(C_GRAY), |r| {
                            if r > 0.0 { Style::default().fg(C_GREEN) } else { Style::default().fg(C_RED) }
                        })),
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
                ])
            }
        }
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(11),  // Date
            Constraint::Length(7),   // Ticker
            Constraint::Length(7),   // ER
            Constraint::Length(5),   // Strategy
            Constraint::Length(4),   // Qty
            Constraint::Length(7),   // Credit
            Constraint::Length(9),   // GTC
            Constraint::Length(10),  // BE (Lower/Upper)
            Constraint::Length(7),   // MaxPft
            Constraint::Length(9),   // P&L
            Constraint::Length(7),   // ROC%
            Constraint::Length(5),   // DTE
            Constraint::Length(9),   // Exit (MM/DD/YY)
            Constraint::Length(6),   // Held (Xd)
            Constraint::Length(7),   // Status
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(
                format!(
                    " Trade Journal — {} shown  ({} open, {} closed total) ",
                    shown_c, open_c, closed_c
                ),
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

// ── Trade detail pane ─────────────────────────────────────────────────────────

fn draw_trade_detail(f: &mut Frame, area: Rect, trade: &Trade, scroll: u16) {
    let spread_type = trade.spread_type();
    let max_profit  = calculate_max_profit(trade.credit_received, trade.quantity);
    let max_loss    = calculate_max_loss_from_legs(&trade.legs, trade.credit_received, trade.quantity, spread_type);
    let breakevens  = calculate_breakevens(&trade.legs, spread_type);
    let leg_desc    = format_trade_description(&trade.legs, spread_type);
    let dte         = calculate_remaining_dte(&trade.expiration_date);
    let roc         = trade.pnl.and_then(|p| calculate_roc(p, &trade.legs, trade.credit_received, trade.quantity, spread_type, trade.bpr));
    let pct_max     = trade.pnl.map(|p| calculate_pct_max_profit(p, trade.credit_received, trade.quantity));
    let be_str      = if breakevens.is_empty() {
        "—".to_string()
    } else {
        breakevens.iter().map(|b| format!("{:.2}", b)).collect::<Vec<_>>().join(" / ")
    };

    let mut lines: Vec<Line> = Vec::new();

    // ── Header
    lines.push(Line::from(vec![
        Span::styled(format!(" {} ", trade.ticker), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
        Span::styled(format!("[{}]", trade.strategy.badge()), Style::default().fg(badge_color(spread_type))),
        Span::styled(format!("  {}", trade.strategy.label()), Style::default().fg(C_GRAY)),
        Span::styled(
            if trade.is_open() { "  ● OPEN" } else { "  ✓ CLOSED" },
            Style::default().fg(if trade.is_open() { C_YELLOW } else { C_GRAY }),
        ),
    ]));
    lines.push(Line::from(""));

    // ── Description / Qty
    lines.push(Line::from(vec![
        Span::styled("  Description: ", Style::default().fg(C_GRAY)),
        Span::styled(leg_desc, Style::default().fg(C_CYAN)),
        Span::styled("   Qty: ", Style::default().fg(C_GRAY)),
        Span::styled(trade.quantity.to_string(), Style::default().fg(C_WHITE)),
    ]));

    // ── Dates
    lines.push(Line::from(vec![
        Span::styled("  Entry:  ", Style::default().fg(C_GRAY)),
        Span::styled(trade.entry_date.format("%Y-%m-%d").to_string(), Style::default().fg(C_WHITE)),
        Span::styled("   Exp: ", Style::default().fg(C_GRAY)),
        Span::styled(
            trade.expiration_date.format("%Y-%m-%d").to_string(),
            Style::default().fg(if dte <= 14 { C_RED } else if dte <= 21 { C_YELLOW } else { C_WHITE }),
        ),
        Span::styled(
            if trade.is_open() { format!("  ({} DTE)", dte) } else { String::new() },
            Style::default().fg(C_GRAY),
        ),
    ]));

    if let Some(exit) = trade.exit_date {
        let held = (exit.date_naive() - trade.trade_date.date_naive()).num_days();
        lines.push(Line::from(vec![
            Span::styled("  Exit:   ", Style::default().fg(C_GRAY)),
            Span::styled(exit.format("%Y-%m-%d").to_string(), Style::default().fg(C_WHITE)),
            Span::styled(
                trade.exit_reason.as_deref().map(|r| format!("  ({})", r)).unwrap_or_default(),
                Style::default().fg(C_GRAY),
            ),
            Span::styled(format!("   Held: {}d", held), Style::default().fg(C_GRAY)),
            if let Some(dc) = trade.dte_at_close {
                Span::styled(format!("   DTE@close: {}", dc), Style::default().fg(C_GRAY))
            } else {
                Span::raw("")
            },
        ]));
        // IV / Delta at close + roll count
        let has_close_greeks = trade.iv_at_close.is_some() || trade.delta_at_close.is_some() || trade.roll_count > 0;
        if has_close_greeks {
            let mut spans: Vec<Span> = vec![Span::raw("  ")];
            if let Some(iv) = trade.iv_at_close {
                spans.push(Span::styled("IV@close: ", Style::default().fg(C_GRAY)));
                spans.push(Span::styled(format!("{:.1}%  ", iv), Style::default().fg(C_CYAN)));
            }
            if let Some(d) = trade.delta_at_close {
                spans.push(Span::styled("Δ@close: ", Style::default().fg(C_GRAY)));
                spans.push(Span::styled(format!("{:.2}  ", d), Style::default().fg(C_CYAN)));
            }
            if trade.roll_count > 0 {
                spans.push(Span::styled("Rolls: ", Style::default().fg(C_GRAY)));
                spans.push(Span::styled(format!("{}", trade.roll_count), Style::default().fg(C_YELLOW)));
            }
            lines.push(Line::from(spans));
        }
    }
    lines.push(Line::from(""));

    // ── P&L block
    lines.push(Line::from(vec![
        Span::styled("  Credit:     ", Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.2}", trade.credit_received), Style::default().fg(C_CYAN)),
        Span::styled("   Max Profit: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("${:.0}", max_profit), Style::default().fg(C_GREEN)),
        Span::styled("   Max Loss: ", Style::default().fg(C_GRAY)),
        Span::styled(
            if max_loss > 0.0 { format!("${:.0}", max_loss) } else { "Undef".to_string() },
            Style::default().fg(C_RED),
        ),
    ]));

    if let Some(pnl) = trade.pnl {
        let pc = if pnl > 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::styled("  P&L:        ", Style::default().fg(C_GRAY)),
            Span::styled(format!("${:+.2}", pnl), Style::default().fg(pc).add_modifier(Modifier::BOLD)),
            Span::styled(
                pct_max.map_or(String::new(), |p| format!("  ({:.1}% of max)", p)),
                Style::default().fg(C_GRAY),
            ),
            Span::styled(
                roc.map_or(String::new(), |r| format!("   ROC: {:.1}%", r)),
                Style::default().fg(if roc.unwrap_or(0.0) > 0.0 { C_GREEN } else { C_RED }),
            ),
        ]));
        if let Some(dp) = trade.debit_paid {
            lines.push(Line::from(vec![
                Span::styled("  Debit paid: ", Style::default().fg(C_GRAY)),
                Span::styled(format!("${:.4}", dp), Style::default().fg(C_WHITE)),
            ]));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("  Breakeven:  ", Style::default().fg(C_GRAY)),
        Span::styled(be_str, Style::default().fg(C_YELLOW)),
    ]));

    // ── Greeks
    if trade.delta.is_some() || trade.theta.is_some() {
        lines.push(Line::from(""));
        let mut gs = vec![Span::styled("  Greeks:  ", Style::default().fg(C_GRAY))];
        if let Some(d)  = trade.delta { gs.push(Span::styled(format!("Δ{:.3}  ", d),  Style::default().fg(C_BLUE))); }
        if let Some(th) = trade.theta { gs.push(Span::styled(format!("Θ{:.3}  ", th), Style::default().fg(C_GREEN))); }
        if let Some(g)  = trade.gamma { gs.push(Span::styled(format!("Γ{:.4}  ", g),  Style::default().fg(C_GRAY))); }
        if let Some(v)  = trade.vega  { gs.push(Span::styled(format!("V{:.3}  ", v),  Style::default().fg(C_YELLOW))); }
        if let Some(pop) = trade.pop {
            gs.push(Span::styled(
                format!("POP {:.1}%", pop),
                Style::default().fg(if pop >= 70.0 { C_GREEN } else if pop >= 50.0 { C_YELLOW } else { C_RED }),
            ));
        }
        lines.push(Line::from(gs));
    }

    // ── Entry conditions
    if trade.underlying_price.is_some() || trade.iv_rank.is_some() {
        lines.push(Line::from(""));
        let mut cs = vec![Span::styled("  Entry:   ", Style::default().fg(C_GRAY))];
        if let Some(up) = trade.underlying_price {
            cs.push(Span::styled(format!("S ${:.2}  ", up), Style::default().fg(C_WHITE)));
        }
        if let Some(ivr) = trade.iv_rank {
            let ic = if ivr >= 50.0 { C_GREEN } else if ivr >= 30.0 { C_YELLOW } else { C_GRAY };
            cs.push(Span::styled(format!("IVR {:.0}  ", ivr), Style::default().fg(ic)));
        }
        if let Some(de) = trade.entry_dte   { cs.push(Span::styled(format!("DTE {}  ", de), Style::default().fg(C_GRAY))); }
        if let Some(vx) = trade.vix_at_entry { cs.push(Span::styled(format!("VIX {:.1}  ", vx), Style::default().fg(C_YELLOW))); }
        if let Some(iv) = trade.implied_volatility { cs.push(Span::styled(format!("IV {:.1}%", iv * 100.0), Style::default().fg(C_GRAY))); }
        lines.push(Line::from(cs));
        if let Some(uc) = trade.underlying_price_at_close {
            lines.push(Line::from(vec![
                Span::styled("  Close:   ", Style::default().fg(C_GRAY)),
                Span::styled(format!("S ${:.2}", uc), Style::default().fg(C_WHITE)),
            ]));
        }
    }

    // ── Commission / BPR / flags
    let mut misc: Vec<Span> = vec![Span::styled("  ", Style::default())];
    if let Some(c)  = trade.commission  { misc.push(Span::styled(format!("Comm: ${:.2}  ", c),  Style::default().fg(C_GRAY))); }
    if let Some(b)  = trade.bpr         { misc.push(Span::styled(format!("BPR: ${:.0}  ", b),   Style::default().fg(C_YELLOW))); }
    if let Some(sw) = trade.spread_width { misc.push(Span::styled(format!("Width: ${:.0}  ", sw), Style::default().fg(C_GRAY))); }
    if trade.is_earnings_play { misc.push(Span::styled("⚡ Earnings  ", Style::default().fg(C_YELLOW))); }
    if trade.is_tested        { misc.push(Span::styled("⚠ Tested", Style::default().fg(C_RED))); }
    if misc.len() > 1 { lines.push(Line::from("")); lines.push(Line::from(misc)); }

    // ── Management rule / target
    if let Some(m) = &trade.management_rule {
        lines.push(Line::from(vec![
            Span::styled("  Mgmt Rule: ", Style::default().fg(C_GRAY)),
            Span::styled(m.clone(), Style::default().fg(C_CYAN)),
        ]));
    }
    if let Some(tgt) = trade.target_profit_pct {
        lines.push(Line::from(vec![
            Span::styled("  Target:    ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:.0}% of max profit", tgt), Style::default().fg(C_WHITE)),
        ]));
    }

    // ── Grade
    if let Some(grade) = &trade.trade_grade {
        let gc = match grade.as_str() {
            "A" => C_GREEN,
            "B" => Color::Rgb(132, 204, 22),
            "C" => C_YELLOW,
            "D" => Color::Rgb(249, 115, 22),
            _   => C_RED,
        };
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Grade: ", Style::default().fg(C_GRAY)),
            Span::styled(grade.clone(), Style::default().fg(gc).add_modifier(Modifier::BOLD)),
            trade.grade_notes.as_ref()
                .map_or(Span::raw(""), |n| Span::styled(format!("  — {}", n), Style::default().fg(C_GRAY))),
        ]));
    }

    // ── Entry reason
    if let Some(er) = &trade.entry_reason {
        lines.push(Line::from(vec![
            Span::styled("  Entry reason: ", Style::default().fg(C_GRAY)),
            Span::styled(er.clone(), Style::default().fg(C_WHITE)),
        ]));
    }

    // ── Roll chain
    if let Some(rid) = trade.rolled_from_id {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ↩ Rolled from: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("Trade #{}", rid), Style::default().fg(C_BLUE)),
        ]));
    }

    // ── Tags
    if !trade.tags.is_empty() {
        lines.push(Line::from(""));
        let mut ts = vec![Span::styled("  Tags: ", Style::default().fg(C_GRAY))];
        for tag in &trade.tags {
            ts.push(Span::styled(format!("[{}] ", tag), Style::default().bg(C_DARK).fg(C_CYAN)));
        }
        lines.push(Line::from(ts));
    }

    // ── Notes
    if let Some(notes) = &trade.notes {
        if !notes.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Notes:", Style::default().fg(C_GRAY))]));
            for ln in notes.split('\n') {
                lines.push(Line::from(vec![
                    Span::styled(format!("    {}", ln), Style::default().fg(Color::Rgb(203, 213, 225))),
                ]));
            }
        }
    }

    // ── Per-leg breakdown
    if !trade.legs.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(format!("  Legs ({}):", trade.legs.len()), Style::default().fg(C_GRAY).add_modifier(Modifier::BOLD)),
        ]));
        for leg in &trade.legs {
            let lbl = match leg.leg_type {
                LegType::ShortPut  => "Short Put ",
                LegType::LongPut   => "Long Put  ",
                LegType::ShortCall => "Short Call",
                LegType::LongCall  => "Long Call ",
            };
            let close_str = leg.close_premium
                .map(|cp| format!("  BTC: ${:.4}", cp))
                .unwrap_or_default();
            
            let mut leg_line = vec![
                Span::styled(format!("    {} ", lbl), Style::default().fg(C_GRAY)),
                Span::styled(format!("${:.2}", leg.strike), Style::default().fg(C_WHITE)),
                Span::styled(format!("  prem: ${:.4}", leg.premium), Style::default().fg(C_CYAN)),
            ];
            
            if let Some(exp) = &leg.expiration_date {
                let clean_exp = exp.split('T').next().unwrap_or(exp);
                leg_line.push(Span::styled(format!("  exp: {}", clean_exp), Style::default().fg(C_GRAY)));
            }
            
            leg_line.push(Span::styled(close_str, Style::default().fg(C_RED)));
            
            lines.push(Line::from(leg_line));
        }
    }

    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_BLUE))
                    .title(Span::styled(
                        " Trade Detail  (D:hide  ↑↓:scroll) ",
                        Style::default().fg(C_CYAN),
                    )),
            ),
        area,
    );
}

/// Count how many lines `draw_trade_detail` would push for `trade`.
/// Must stay in sync with every `lines.push(...)` in that function.
pub fn count_detail_lines(trade: &crate::models::Trade) -> usize {
    let mut n = 0usize;

    n += 1; // header  (ticker + badge + strategy + status)
    n += 1; // blank after header
    n += 1; // description / qty
    n += 1; // entry date / expiry

    if trade.exit_date.is_some() {
        n += 1; // exit date line
        let has_close_greeks = trade.iv_at_close.is_some() || trade.delta_at_close.is_some() || trade.roll_count > 0;
        if has_close_greeks { n += 1; } // IV/delta@close + roll count line
    }

    n += 1; // blank after exit block (always pushed)
    n += 1; // credit / max profit / max loss

    if trade.pnl.is_some() {
        n += 1; // P&L line
        if trade.debit_paid.is_some() { n += 1; } // debit paid
    }

    n += 1; // breakeven

    if trade.delta.is_some() || trade.theta.is_some() {
        n += 1; // blank
        n += 1; // Greeks line
    }

    if trade.underlying_price.is_some() || trade.iv_rank.is_some() {
        n += 1; // blank
        n += 1; // entry conditions line
        if trade.underlying_price_at_close.is_some() { n += 1; } // close price
    }

    let has_misc = trade.commission.is_some()
        || trade.bpr.is_some()
        || trade.spread_width.is_some()
        || trade.is_earnings_play
        || trade.is_tested;
    if has_misc { n += 2; } // blank + misc line

    if trade.management_rule.is_some() { n += 1; }
    if trade.target_profit_pct.is_some() { n += 1; }

    if trade.trade_grade.is_some() {
        n += 1; // blank
        n += 1; // grade line
    }

    if trade.entry_reason.is_some() { n += 1; }

    if trade.rolled_from_id.is_some() {
        n += 1; // blank
        n += 1; // rolled-from line
    }

    if !trade.tags.is_empty() {
        n += 1; // blank
        n += 1; // tags line
    }

    if let Some(ref notes) = trade.notes {
        if !notes.is_empty() {
            n += 1; // blank
            n += 1; // "Notes:" header
            n += notes.split('\n').count(); // one line per segment
        }
    }

    if !trade.legs.is_empty() {
        n += 1; // blank
        n += 1; // "Legs (N):" header
        n += trade.legs.len(); // one line per leg
    }

    n
}

pub fn count_perf_lines(
    stats: &crate::models::PortfolioStats,
    perf:  &crate::models::PerformanceStats,
    width: usize,
) -> usize {
    perf_health_lines(stats, width).len()
    + perf_returns_lines(stats, perf, width).len()
    + perf_advanced_lines(stats, width).len()
    + perf_chart_lines(stats, perf, width).len()
    + perf_strategy_lines(perf, width).len()
    + perf_monthly_lines(perf, &std::collections::HashMap::new(), width).len()
    + 1  // trailing blank line pushed in draw_performance
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
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(area);

    let items: Vec<ListItem> = playbooks.iter().map(|pb| {
        let badge = pb.spread_type.as_deref()
            .map(|st| format!("[{}]", crate::models::StrategyType::from_str(st).badge()))
            .unwrap_or_else(|| "    ".to_string());
        ListItem::new(vec![Line::from(vec![
            Span::styled(
                format!(" {} ", badge),
                Style::default().fg(pb.spread_type.as_deref().map(badge_color).unwrap_or(C_GRAY)),
            ),
            Span::styled(pb.name.clone(), Style::default().fg(C_WHITE)),
        ])])
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
                .constraints([Constraint::Length(5), Constraint::Length(6), Constraint::Min(0)])
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
                f.render_widget(
                    Paragraph::new(vec![Line::from(""), Line::from(bs)])
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

            // ── Strategy stats panel
            {
                let sb = perf_stats.strategy_breakdown.iter()
                    .find(|sb| sb.strategy.as_str() == pb.spread_type.as_deref().unwrap_or(""));
                let iw = dc[1].width.saturating_sub(2) as usize;
                let stats_lines: Vec<Line> = if let Some(sb) = sb {
                    let pnl_color = if sb.total_pnl >= 0.0 { C_GREEN } else { C_RED };
                    let roc_color = if sb.avg_roc  >= 0.0 { C_GREEN } else { C_RED };
                    vec![
                        stat_row("Win Rate:", &format!("{:.0}%", sb.win_rate),              C_WHITE,   iw),
                        stat_row("Avg R:R:",  &format!("1:{:.1}", sb.avg_roc / 100.0),      roc_color, iw),
                        stat_row("Usage:",    &format!("{} trades", sb.trades),             C_WHITE,   iw),
                        stat_row("Total P&L:",&format!("${:.0}", sb.total_pnl),             pnl_color, iw),
                    ]
                } else {
                    vec![
                        stat_row("Win Rate:", "—",        C_GRAY, iw),
                        stat_row("Avg R:R:",  "—",        C_GRAY, iw),
                        stat_row("Usage:",    "0 trades", C_GRAY, iw),
                        stat_row("Total P&L:","—",        C_GRAY, iw),
                    ]
                };
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
        calculate_max_profit(trade.credit_received, trade.quantity)
    };
    let max_loss   = calculate_max_loss_from_legs(
        &trade.legs, trade.credit_received, trade.quantity, spread_type,
    );
    let breakevens = calculate_breakevens(&trade.legs, spread_type);
    let pop        = trade.pop.unwrap_or_else(|| estimate_pop(trade));

    let profit_str = format!("+${:.0}", max_profit);
    let loss_str   = if max_loss > 0.0 { format!("-${:.0}", max_loss) } else { "—".to_string() };
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

    // ── Chart dimensions ─────────────────────────────────────────────────────
    let x_axis_rows: u16 = 3; // price-label row + marker row + blank
    let chart_height = inner_h.saturating_sub(header_height + x_axis_rows);

    if chart_height < 4 {
        // Not enough vertical space — show header only
        let mut lines = header_lines;
        lines.push(Line::from(Span::styled(
            "  (terminal too small for chart)",
            Style::default().fg(C_GRAY),
        )));
        f.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    // ── Build chart body ──────────────────────────────────────────────────────
    let chart_lines = build_payoff_chart(trade, inner_w, chart_height);

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
            label_map.entry(col).or_insert(("│", C_GRAY));
            price_at_col.entry(col).or_insert(leg.strike);
        }
    }
    for &be in &breakevens {
        let col = price_to_col(be).min(plot_w - 1);
        label_map.insert(col, ("×", C_YELLOW));
        price_at_col.insert(col, be);
    }
    if let Some(up) = trade.underlying_price {
        let col = price_to_col(up).min(plot_w - 1);
        label_map.insert(col, ("●", C_CYAN));
        price_at_col.insert(col, up);
    }

    // Build price-label row (non-overlapping, centered on marker columns)
    let mut price_label_row: Vec<u8> = vec![b' '; plot_w];
    let mut last_end: i64 = -1;
    for (&col, &price) in &price_at_col {
        let price_str = format!("{:.0}", price);
        let half_len  = price_str.len() as i64 / 2;
        let start     = ((col as i64 - half_len).max(last_end + 1)) as usize;
        if start < plot_w {
            for (i, b) in price_str.bytes().enumerate() {
                if start + i < plot_w {
                    price_label_row[start + i] = b;
                }
            }
            last_end = (start + price_str.len()) as i64;
        }
    }

    // Marker row with colored spans
    let y_pad = " ".repeat(Y_AXIS_W);
    let mut x2_spans: Vec<Span<'static>> = vec![Span::raw(y_pad.clone())];
    let mut cursor = 0usize;
    for (&col, &(sym, color)) in &label_map {
        let col = col.min(plot_w - 1);
        if col >= cursor {
            if col > cursor {
                x2_spans.push(Span::raw(" ".repeat(col - cursor)));
            }
            x2_spans.push(Span::styled(sym.to_string(), Style::default().fg(color)));
            cursor = col + 1;
        }
    }
    if cursor < plot_w {
        x2_spans.push(Span::raw(" ".repeat(plot_w - cursor)));
    }

    // ── Assemble and render ───────────────────────────────────────────────────
    let mut all_lines: Vec<Line<'static>> = Vec::new();
    all_lines.extend(header_lines);
    all_lines.extend(chart_lines);
    // Price-label row
    all_lines.push(Line::from(vec![
        Span::raw(y_pad),
        Span::styled(
            String::from_utf8_lossy(&price_label_row).into_owned(),
            Style::default().fg(C_GRAY),
        ),
    ]));
    // Marker symbol row
    all_lines.push(Line::from(x2_spans));
    all_lines.push(Line::from(""));

    f.render_widget(Paragraph::new(all_lines).block(block), area);
}

/// Build the ASCII payoff chart body. Returns `Vec<Line<'static>>` for use in a Paragraph.
fn build_payoff_chart(trade: &Trade, pane_width: u16, chart_height: u16) -> Vec<Line<'static>> {
    let legs   = &trade.legs;
    let credit = trade.credit_received;

    const Y_AXIS_W: usize = 9; // e.g. " +$480 ┤ "
    let plot_w = (pane_width as usize).saturating_sub(Y_AXIS_W).max(10);
    let plot_h = chart_height as usize;

    // Price range from strikes
    let strikes: Vec<f64> = legs.iter().map(|l| l.strike).filter(|&s| s > 0.0).collect();
    if strikes.is_empty() {
        return vec![Line::from(Span::styled(
            "  No strike data to chart.",
            Style::default().fg(C_GRAY),
        ))];
    }

    let min_s = strikes.iter().cloned().fold(f64::INFINITY,     f64::min);
    let max_s = strikes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (lo, hi) = if (max_s - min_s).abs() < 0.01 {
        (min_s * 0.80, max_s * 1.20)
    } else {
        (min_s * 0.88, max_s * 1.12)
    };

    // Sample P&L at each column price
    let prices: Vec<f64> = (0..plot_w)
        .map(|i| lo + (hi - lo) * i as f64 / (plot_w - 1).max(1) as f64)
        .collect();
    let pnl_vals: Vec<f64> = if trade.spread_type() == "calendar_spread" {
        let remaining_dte = trade.back_month_expiration
            .map(|bme| {
                let secs = (bme.timestamp() - trade.expiration_date.timestamp()).max(0) as f64;
                secs / (86_400.0 * 365.25)
            })
            .unwrap_or(30.0 / 365.25);
        let iv = trade.implied_volatility
            .map(|v| if v > 2.0 { v / 100.0 } else { v })
            .unwrap_or(0.25);
        prices.iter().map(|&p| calculate_calendar_payoff_at_price(legs, credit, p, remaining_dte, iv)).collect()
    } else {
        prices.iter().map(|&p| calculate_payoff_at_price(legs, credit, p)).collect()
    };

    let max_pnl = pnl_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max).max(0.01);
    let min_pnl = pnl_vals.iter().cloned().fold(f64::INFINITY,     f64::min).min(-0.01);

    let zero_row = plot_h / 2;

    // Y-axis label formatter
    let fmt_y = |val: f64| -> String {
        if val.abs() < 0.5 {
            "   $0  ┤ ".to_string()
        } else if val > 0.0 {
            format!("{:>6} ┤ ", format!("+${:.0}", val))
        } else {
            format!("{:>6} ┤ ", format!("-${:.0}", val.abs()))
        }
    };

    // Rows that get a Y-axis dollar label (others show blank prefix)
    let labeled: [usize; 5] = [
        0,
        zero_row.saturating_sub(zero_row / 2),
        zero_row,
        zero_row + (plot_h.saturating_sub(zero_row + 1)) / 2,
        plot_h.saturating_sub(1),
    ];

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(plot_h);

    for r in 0..plot_h {
        // P&L level this row represents
        let level: f64 = if r < zero_row {
            max_pnl * (zero_row - r) as f64 / zero_row.max(1) as f64
        } else if r == zero_row {
            0.0
        } else {
            min_pnl * (r - zero_row) as f64 / (plot_h - zero_row - 1).max(1) as f64
        };

        let y_label = if labeled.contains(&r) {
            fmt_y(level)
        } else {
            " ".repeat(Y_AXIS_W)
        };
        let y_span = Span::styled(y_label, Style::default().fg(C_GRAY));

        if r == zero_row {
            lines.push(Line::from(vec![
                y_span,
                Span::styled("─".repeat(plot_w), Style::default().fg(C_WHITE)),
            ]));
        } else {
            let mut spans: Vec<Span<'static>> = vec![y_span];

            // Run-length encode colored cells
            let mut run_buf   = String::new();
            let mut run_color: Option<Color> = None;

            for x in 0..plot_w {
                let pnl = pnl_vals[x];
                let (ch, col): (char, Option<Color>) = if r < zero_row {
                    if pnl > 0.0 && pnl >= level { ('█', Some(C_GREEN)) } else { (' ', None) }
                } else {
                    if pnl < 0.0 && pnl <= level { ('▓', Some(C_RED))   } else { (' ', None) }
                };

                if run_color == col {
                    run_buf.push(ch);
                } else {
                    // Flush previous run
                    if !run_buf.is_empty() {
                        let content = std::mem::take(&mut run_buf);
                        match run_color {
                            Some(c) => spans.push(Span::styled(content, Style::default().fg(c))),
                            None    => spans.push(Span::raw(content)),
                        }
                    }
                    run_color = col;
                    run_buf.push(ch);
                }
            }
            // Flush final run
            if !run_buf.is_empty() {
                match run_color {
                    Some(c) => spans.push(Span::styled(run_buf, Style::default().fg(c))),
                    None    => spans.push(Span::raw(run_buf)),
                }
            }

            lines.push(Line::from(spans));
        }
    }

    lines
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
        _                                              => C_GRAY,
    }
}

fn build_equity_sparkline(trades: &[&Trade], width: usize) -> Vec<Line<'static>> {
    if trades.is_empty() || width < 4 {
        return vec![Line::from(" No closed trades yet.")];
    }
    let mut running = 0.0_f64;
    let series: Vec<f64> = trades.iter().map(|t| {
        running += t.pnl.unwrap_or(0.0);
        running
    }).collect();

    let min_val = series.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = series.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range   = (max_val - min_val).max(1.0);
    let cols    = width.min(series.len());
    let step    = (series.len() as f64 / cols as f64).ceil().max(1.0) as usize;
    let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let bar_line: String = (0..cols)
        .map(|i| series[(i * step).min(series.len() - 1)])
        .map(|v| bars[((((v - min_val) / range) * 7.0) as usize).min(7)])
        .collect();

    let final_pnl = *series.last().unwrap_or(&0.0);
    let color = if final_pnl >= 0.0 { C_GREEN } else { C_RED };
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(bar_line, Style::default().fg(color))]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  Low: ${:.2}   High: ${:.2}   Now: ${:.2}", min_val, max_val, final_pnl),
            Style::default().fg(C_GRAY),
        )]),
    ]
}

// ── KPI Info Popup ────────────────────────────────────────────────────────────

fn draw_kpi_popup(f: &mut Frame, area: Rect, stats: &PortfolioStats, max_heat_pct: f64) {
    let w: u16 = 64.min(area.width.saturating_sub(4));
    let h: u16 = 22.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let dialog = Rect::new(x, y, w, h);
    f.render_widget(Clear, dialog);

    let bwd_source = if stats.spy_price.is_some() { "live SPY" } else { "no SPY price" };
    let lines = vec![
        Line::from(vec![Span::styled("  KPI Reference  (tastytrade philosophy)", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("  Balance  ", Style::default().fg(C_YELLOW)), Span::styled(format!("account_size + realized_pnl = ${:.2}", stats.balance), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("  P&L      ", Style::default().fg(C_YELLOW)), Span::styled(format!("realized P&L from closed trades = ${:+.2}", stats.realized_pnl), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("  Unreal   ", Style::default().fg(C_YELLOW)), Span::styled(format!("est. θ×days×100×qty = ${:+.0} (theta heuristic)", stats.unrealized_pnl), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("  BWD      ", Style::default().fg(C_YELLOW)), Span::styled(format!("Σ(δ×β×price/SPY×qty×100) = {:+.1}Δ  [{}]. Goal: near 0 (delta-neutral). Green ≤5, Yellow ≤15, Red >15.", stats.net_beta_weighted_delta, bwd_source), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("  Heat     ", Style::default().fg(C_YELLOW)), Span::styled(format!("${:.0} BPR / ${:.0} acct = {:.1}% allocated (max {:.0}%). >100% = using margin. TastyTrade target: <50%.", stats.total_open_bpr, stats.account_size, stats.alloc_pct, max_heat_pct), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("  Risk     ", Style::default().fg(C_YELLOW)), Span::styled(format!("undefined/defined BPR split = {:.0}/{:.0}  (tgt {:.0}/{:.0})", stats.undefined_risk_pct, stats.defined_risk_pct, stats.target_undefined_pct, 100.0 - stats.target_undefined_pct), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("  VIX      ", Style::default().fg(C_YELLOW)), Span::styled(stats.vix.map_or("unavailable".to_string(), |v| { let lbl = if v > 30.0 { "HIGH fear — sell premium, reduce size 50%" } else if v > 20.0 { "elevated — normal sizing" } else { "LOW — prefer defined risk, trade smaller" }; format!("{:.2}  {}", v, lbl) }), Style::default().fg(C_WHITE))]),
        Line::from(vec![Span::styled("  POP      ", Style::default().fg(C_YELLOW)), Span::styled(format!("avg probability of profit = {:.1}%  across {} open positions", stats.avg_pop, stats.open_trades), Style::default().fg(C_WHITE))]),
        Line::from(""),
        Line::from(vec![Span::styled("  tastytrade rules: ", Style::default().fg(C_GRAY)), Span::styled("Close at 50% max profit  ·  Manage at 21 DTE  ·  Never hold through earnings", Style::default().fg(C_GRAY))]),
        Line::from(vec![Span::styled("  sizing:           ", Style::default().fg(C_GRAY)), Span::styled("Max 5% BPR per trade  ·  Heat < 50% total  ·  Undefined ≤ 75%", Style::default().fg(C_GRAY))]),
        Line::from(""),
        Line::from(vec![Span::styled("  Press i to close", Style::default().fg(C_DARK))]),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(C_CYAN))
                    .title(Span::styled(" ◆ KPI Definitions  (i to close) ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))),
            ),
        dialog,
    );
}

// ── Admin Settings ────────────────────────────────────────────────────────────

fn draw_admin(
    f:               &mut Frame,
    area:            Rect,
    app_mode:        AppMode,
    admin_fields:    &[EditField],
    admin_field_idx: usize,
    admin_scroll:    u16,
    stats:           &PortfolioStats,
    max_heat_pct:    f64,
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
        let lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled("  ✦ Risk Management Settings", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("  Account Size         ", Style::default().fg(C_GRAY)), Span::styled(format!("${:.2}", stats.account_size), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("  Max Heat %           ", Style::default().fg(C_GRAY)), Span::styled(format!("{:.1}%  (current: {:.1}%)", max_heat_pct, stats.alloc_pct), Style::default().fg(C_WHITE))]),
            Line::from(vec![Span::styled("  Target Undefined %   ", Style::default().fg(C_GRAY)), Span::styled(format!("{:.1}%  (current: {:.1}%)", stats.target_undefined_pct, stats.undefined_risk_pct), Style::default().fg(C_WHITE))]),
            Line::from(""),
            Line::from(vec![Span::styled("  ── tastytrade defaults ────────────────────────────────────────", Style::default().fg(C_DARK))]),
            Line::from(vec![Span::styled("  Max Heat            ", Style::default().fg(C_GRAY)), Span::styled("50% of account in BPR", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Undefined Risk      ", Style::default().fg(C_GRAY)), Span::styled("75% of total BPR", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Max Per Position    ", Style::default().fg(C_GRAY)), Span::styled("5% BPR of account", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Entry DTE Range     ", Style::default().fg(C_GRAY)), Span::styled("21–60 DTE", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Min POP             ", Style::default().fg(C_GRAY)), Span::styled("60% probability of profit", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Profit Target       ", Style::default().fg(C_GRAY)), Span::styled("50% of max profit (GTC order)", Style::default().fg(C_GRAY))]),
            Line::from(vec![Span::styled("  Management          ", Style::default().fg(C_GRAY)), Span::styled("Roll or close at 21 DTE", Style::default().fg(C_GRAY))]),
            Line::from(""),
            Line::from(vec![Span::styled("  Press E to edit settings", Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD))]),
        ];
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
) {
    use crate::actions::{AlertKind, ActionRow, build_action_rows};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // stats bar
            Constraint::Min(0),     // alert list
        ])
        .split(area);

    // ── Stats bar: count by kind ──────────────────────────────────────────────
    let defense = alerts.iter().filter(|a| a.kind == AlertKind::Defense || a.kind == AlertKind::MaxLoss).count();
    let warning = alerts.iter().filter(|a| a.kind == AlertKind::Warning).count();
    let manage  = alerts.iter().filter(|a| a.kind == AlertKind::Manage || a.kind == AlertKind::Roll).count();
    let close   = alerts.iter().filter(|a| a.kind == AlertKind::Close).count();

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
    f.render_widget(
        Paragraph::new(stats_line)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_DARK))),
        chunks[0],
    );

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
                    AlertKind::Manage         => C_YELLOW,
                    AlertKind::Close          => C_GREEN,
                    AlertKind::Roll           => C_BLUE,
                    AlertKind::Sizing         => Color::Magenta,
                    AlertKind::Ok             => C_GRAY,
                };
                let badge_style = if matches!(alert.kind, AlertKind::Defense | AlertKind::MaxLoss | AlertKind::GammaRisk) {
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
            .title(Span::styled(" Morning Checklist ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))))
        .highlight_style(Style::default().bg(C_DARK).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], list_state);
}

// ── Performance Tab ───────────────────────────────────────────────────────────

fn draw_performance(
    f: &mut Frame,
    area: Rect,
    stats: &PortfolioStats,
    perf: &PerformanceStats,
    scroll: u16,
    spy_monthly: &std::collections::HashMap<(i32, u32), f64>,
) {
    let width = area.width as usize;
    let mut lines: Vec<Line> = Vec::new();
    lines.extend(perf_health_lines(stats, width));
    lines.extend(perf_returns_lines(stats, perf, width));
    lines.extend(perf_advanced_lines(stats, width));
    lines.extend(perf_chart_lines(stats, perf, width));
    lines.extend(perf_strategy_lines(perf, width));
    lines.extend(perf_monthly_lines(perf, spy_monthly, width));
    lines.push(Line::from(""));

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(C_BLUE))
            .title(Span::styled(" ★ Performance ", Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD))))
        .scroll((scroll, 0));
    f.render_widget(para, area);
}

fn perf_section_header(title: &str, width: usize) -> Line<'static> {
    let bar_len = width.saturating_sub(title.len() + 4).max(2);
    let bar = "━".repeat(bar_len);
    Line::from(vec![
        Span::styled(format!("━━━ {} ", title), Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(bar, Style::default().fg(C_DARK)),
    ])
}

fn perf_health_lines(stats: &PortfolioStats, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("🛡 PORTFOLIO HEALTH", width),
        Line::from(""),
    ];
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
        Span::styled("Unrealized: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.0}", stats.unrealized_pnl), Style::default().fg(if stats.unrealized_pnl >= 0.0 { C_GREEN } else { C_RED })),
        Span::raw("   "),
        Span::styled("Avg POP: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}%", stats.avg_pop), Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Drift: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.1}%", stats.drift), Style::default().fg(drift_color)),
    ]));

    // Risk split bar
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
    lines
}

fn perf_returns_lines(stats: &PortfolioStats, perf: &PerformanceStats, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📈 RETURNS", width),
        Line::from(""),
    ];
    if stats.closed_trades == 0 {
        lines.push(Line::from(vec![Span::styled("  No closed trades yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let pnl_color = if stats.realized_pnl >= 0.0 { C_GREEN } else { C_RED };
    let wr_color = if stats.win_rate >= 0.65 { C_GREEN } else if stats.win_rate >= 0.50 { C_YELLOW } else { C_RED };
    let pf_color = if perf.profit_factor >= 1.5 { C_GREEN } else if perf.profit_factor >= 1.0 { C_YELLOW } else { C_RED };
    let ev_color = if perf.expected_value >= 0.0 { C_GREEN } else { C_RED };

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
            if perf.profit_factor.is_infinite() { "∞".to_string() } else { format!("{:.2}", perf.profit_factor) },
            Style::default().fg(pf_color),
        ),
        Span::raw("   "),
        Span::styled("EV: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.0}/trade", perf.expected_value), Style::default().fg(ev_color)),
    ]));

    let rr = if perf.avg_loss > 0.0 { perf.avg_win / perf.avg_loss } else { 0.0 };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Avg Winner: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:+.0}", perf.avg_win), Style::default().fg(C_GREEN)),
        Span::raw("   "),
        Span::styled("Avg Loser: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("-{:.0}", perf.avg_loss), Style::default().fg(C_RED)),
        Span::raw("   "),
        Span::styled("Avg R:R: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("1:{:.2}", rr), Style::default().fg(C_WHITE)),
    ]));

    let sharpe_color  = if perf.sharpe_ratio  >= 1.0 { C_GREEN } else if perf.sharpe_ratio  >= 0.0 { C_YELLOW } else { C_RED };
    let sortino_color = if perf.sortino_ratio >= 1.0 { C_GREEN } else if perf.sortino_ratio >= 0.0 { C_YELLOW } else { C_RED };
    let calmar_color  = if perf.calmar_ratio  >= 0.5 { C_GREEN } else if perf.calmar_ratio  >= 0.0 { C_YELLOW } else { C_RED };
    let dd_color = if stats.max_drawdown > 0.0 { C_RED } else { C_GREEN };
    let streak_color = if stats.current_streak >= 0 { C_GREEN } else { C_RED };
    let streak_str = if stats.current_streak >= 0 {
        format!("+{}W", stats.current_streak)
    } else {
        format!("{}L", stats.current_streak.abs())
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Sharpe: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.2}", perf.sharpe_ratio), Style::default().fg(sharpe_color)),
        Span::raw("   "),
        Span::styled("Sortino: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.2}", perf.sortino_ratio), Style::default().fg(sortino_color)),
        Span::raw("   "),
        Span::styled("Max Drawdown: ", Style::default().fg(C_GRAY)),
        Span::styled(
            format!("-{:.0} ({:.1}%)", stats.max_drawdown, stats.max_drawdown_pct),
            Style::default().fg(dd_color),
        ),
        Span::raw("   "),
        Span::styled("Calmar: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.2}", perf.calmar_ratio), Style::default().fg(calmar_color)),
    ]));
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
    ]));

    if perf.avg_annualized_roc != 0.0 {
        let ann_color = if perf.avg_annualized_roc >= 0.0 { C_GREEN } else { C_RED };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Avg Annualized ROC: ", Style::default().fg(C_GRAY)),
            Span::styled(format!("{:+.1}%/yr", perf.avg_annualized_roc), Style::default().fg(ann_color)),
        ]));
    }

    let dte_str = perf.avg_dte_at_close.map_or("—".to_string(), |d| format!("{:.1}d", d));
    let pmc_str = perf.avg_pct_max_captured.map_or("—".to_string(), |p| format!("{:.1}%", p));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Avg DTE at Close: ", Style::default().fg(C_GRAY)),
        Span::styled(dte_str, Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Avg % Max Captured: ", Style::default().fg(C_GRAY)),
        Span::styled(pmc_str, Style::default().fg(C_WHITE)),
        Span::raw("   "),
        Span::styled("Avg Held: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.0}d", perf.avg_held_days), Style::default().fg(C_WHITE)),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  "),
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
        Span::raw("   "),
        Span::styled("Avg ROC: ", Style::default().fg(C_GRAY)),
        Span::styled(format!("{:.1}%", stats.avg_roc), Style::default().fg(C_WHITE)),
    ]));
    lines
}

fn perf_advanced_lines(stats: &PortfolioStats, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("⚙ ADVANCED METRICS", width),
        Line::from(""),
    ];

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

fn perf_chart_lines(stats: &PortfolioStats, perf: &PerformanceStats, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📊 ACCOUNT GROWTH", width),
        Line::from(""),
    ];

    let history = &perf.balance_history;
    if history.len() < 2 {
        lines.push(Line::from(vec![Span::styled("  No closed trades yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    let last_bal = *history.last().unwrap();
    let growth = last_bal - stats.account_size;
    let growth_pct = if stats.account_size > 0.0 { (growth / stats.account_size) * 100.0 } else { 0.0 };
    let bal_color = if growth >= 0.0 { C_GREEN } else { C_RED };

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("${:.0}", last_bal), Style::default().fg(bal_color)),
        Span::raw("  "),
        Span::styled(format!("({:+.2}%)   ▲ Account Growth", growth_pct), Style::default().fg(C_GRAY)),
    ]));

    // Build sparkline from balance_history
    let min_val = history.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = history.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max_val - min_val).max(1.0);
    let cols = (width.saturating_sub(4)).min(history.len());
    let step = ((history.len() as f64 / cols as f64).ceil() as usize).max(1);
    let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let chart_str: String = (0..cols)
        .map(|i| history[(i * step).min(history.len() - 1)])
        .map(|v| bars[((((v - min_val) / range) * 7.0) as usize).min(7)])
        .collect();

    let chart_color = if last_bal >= stats.account_size { C_GREEN } else { C_RED };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(chart_str, Style::default().fg(chart_color)),
    ]));

    // Baseline marker (only if range spans the baseline)
    if min_val <= stats.account_size && max_val >= stats.account_size {
        let baseline_bar = "─".repeat(cols.min(width.saturating_sub(30)));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(baseline_bar, Style::default().fg(C_DARK)),
            Span::styled(format!(" ${:.0} (baseline)", stats.account_size), Style::default().fg(C_GRAY)),
        ]));
    }

    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(format!("Low: ${:.0}", min_val), Style::default().fg(C_GRAY)),
        Span::raw("    "),
        Span::styled(format!("High: ${:.0}", max_val), Style::default().fg(C_GRAY)),
        Span::raw("    "),
        Span::styled(format!("Now: ${:.0}", last_bal), Style::default().fg(bal_color)),
    ]));
    lines
}

fn perf_strategy_lines(perf: &PerformanceStats, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("⚔ STRATEGY BREAKDOWN", width),
        Line::from(""),
    ];
    if perf.strategy_breakdown.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No closed trades yet.", Style::default().fg(C_GRAY))]));
        return lines;
    }

    lines.push(Line::from(vec![Span::styled(
        "  Strategy                Trades  Win%    P&L        Avg P&L  Avg ROC",
        Style::default().fg(C_GRAY),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  ─────────────────────────────────────────────────────────────────────",
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
        ]));
    }

    lines.push(Line::from(vec![Span::styled(
        "  ─────────────────────────────────────────────────────────────────────",
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
    lines
}

fn perf_monthly_lines(
    perf: &PerformanceStats,
    spy_monthly: &std::collections::HashMap<(i32, u32), f64>,
    width: usize,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        perf_section_header("📅 MONTHLY P&L", width),
        Line::from(""),
    ];
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
        let spy_span = match spy_monthly.get(&(mp.year, mp.month)) {
            Some(&r) => Span::styled(
                format!("  SPY{:+.1}%", r),
                Style::default().fg(if r >= 0.0 { C_GREEN } else { C_RED }),
            ),
            None => Span::styled("  SPY  —  ".to_string(), Style::default().fg(C_GRAY)),
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} {:4}", m_name, mp.year), Style::default().fg(C_GRAY)),
            Span::styled(format!(" ({:2}T) ", mp.trade_count), Style::default().fg(C_GRAY)),
            Span::styled(format!("{:>+8.0}   ", mp.pnl), Style::default().fg(pnl_color)),
            Span::styled(bar_str, Style::default().fg(pnl_color)),
            spy_span,
        ]));
    }
    lines
}
