use crate::commands::top::utils::calculate_diffs;

use super::app::{App, AppState, SortedBy};

use ratatui::{
    layout::{Alignment, Constraint},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

pub fn render_app_components(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let header = Row::new(vec![
        Cell::from(Line::from("Key").style(Style::default().add_modifier(Modifier::BOLD))),
        Cell::from(Line::from(format!("{:?} delta", app.refresh_rate)).right_aligned()),
        Cell::from(Line::from("Since start").right_aligned()),
        Cell::from(Line::from("Total").right_aligned()),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let mut rows = vec![header];

    let diffs = calculate_diffs(&app.previous_stats, &app.current_stats).counters;
    let total_diffs = calculate_diffs(&app.starting_stats, &app.current_stats).counters;

    let push_stats_line = |key: &str, rows: &mut Vec<Row>| {
        let delta = diffs.get(key).copied().unwrap_or(0);
        let since_start = total_diffs.get(key).copied().unwrap_or(0);
        let total = app.current_stats.counters.get(key).copied().unwrap_or(0);

        let row = Row::new(vec![
            Cell::from(format!("{key}:")),
            Cell::from(Text::from(delta.to_string()).alignment(Alignment::Right)),
            Cell::from(Text::from(since_start.to_string()).alignment(Alignment::Right)),
            Cell::from(Text::from(total.to_string()).alignment(Alignment::Right)),
        ])
        .style(Style::default());

        rows.push(row);
    };

    match app.sort_algo {
        SortedBy::AlphabeticalAscending => {
            for key in &app.alphabetical_sort {
                push_stats_line(key, &mut rows);
            }
        }
        SortedBy::AlphabeticalDecending => {
            for key in app.alphabetical_sort.iter().rev() {
                push_stats_line(key, &mut rows);
            }
        }
        SortedBy::SinceStartTotalDiffAscending => {
            let mut diff_sort: Vec<_> = total_diffs.iter().collect();
            diff_sort.sort_by(|a, b| a.1.cmp(b.1));
            for key in &diff_sort {
                push_stats_line(key.0, &mut rows);
            }
        }
        SortedBy::SinceStartTotalDiffDecending => {
            let mut diff_sort: Vec<_> = total_diffs.iter().collect();
            diff_sort.sort_by(|a, b| b.1.cmp(a.1));
            for key in &diff_sort {
                push_stats_line(key.0, &mut rows);
            }
        }
        SortedBy::TotalAscending => {
            let mut counters: Vec<(&String, u64)> = app
                .current_stats
                .counters
                .iter()
                .map(|(k, v)| (k, *v))
                .collect();

            counters.sort_by_key(|&(_, val)| val);

            for (key, _) in counters {
                push_stats_line(key, &mut rows);
            }
        }
        SortedBy::TotalDescending => {
            let mut counters: Vec<(&String, u64)> = app
                .current_stats
                .counters
                .iter()
                .map(|(k, v)| (k, *v))
                .collect();

            counters.sort_by_key(|&(_, val)| std::cmp::Reverse(val));

            for (key, _) in counters {
                push_stats_line(key, &mut rows);
            }
        }
        _ => todo!(),
    }

    let reserved_lines: usize = 3;
    let visible_data_rows = (area.height as usize).saturating_sub(reserved_lines).max(1);
    let max_offset = app.max_scroll_offset(visible_data_rows);
    let offset = app.scroll_offset.min(max_offset);

    if app.scroll_offset > max_offset {
        app.scroll_offset = max_offset
    }

    let table = Table::new(
        rows,
        [
            Constraint::Min(36),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(25),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("bcachefs-top  –  {:?}", app.sort_algo))
            .title_style(Style::default().fg(Color::Magenta)),
    )
    .column_spacing(1);

    let mut table_state = TableState::default().with_offset(offset);

    frame.render_stateful_widget(table, area, &mut table_state);
}
