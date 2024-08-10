use std::time::Duration;
use time::macros::format_description;

use atuin_client::{
    history::{History, HistoryStats},
    settings::Settings,
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    prelude::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Padding, Paragraph, Row, Table},
    Frame,
};

use super::duration::format_duration;

use super::super::theme::{Meaning, Theme};
use super::interactive::{InputAction, State};

#[allow(clippy::cast_sign_loss)]
fn u64_or_zero(num: i64) -> u64 {
    if num < 0 {
        0
    } else {
        num as u64
    }
}

pub fn draw_commands(
    f: &mut Frame<'_>,
    parent: Rect,
    history: &History,
    stats: &HistoryStats,
    compact: bool,
    theme: &Theme,
) {
    let commands = Layout::default()
        .direction(if compact {
            Direction::Vertical
        } else {
            Direction::Horizontal
        })
        .constraints(if compact {
            [
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ]
        } else {
            [
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 2),
                Constraint::Ratio(1, 4),
            ]
        })
        .split(parent);

    let command = Paragraph::new(Text::from(Span::styled(
        history.command.clone(),
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::White),
    )))
    .block(if compact {
        Block::new()
            .borders(Borders::NONE)
            .style(theme.as_style(Meaning::Base))
    } else {
        Block::new()
            .borders(Borders::ALL)
            .style(theme.as_style(Meaning::Base))
            .title("Command")
            .padding(Padding::horizontal(1))
    });

    let previous = Paragraph::new(
        stats
            .previous
            .clone()
            .map_or("[No previous command]".to_string(), |prev| prev.command),
    )
    .block(if compact {
        Block::new()
            .borders(Borders::NONE)
            .style(theme.as_style(Meaning::Annotation))
    } else {
        Block::new()
            .borders(Borders::ALL)
            .style(theme.as_style(Meaning::Annotation))
            .title("Previous command")
            .padding(Padding::horizontal(1))
    });

    let next = Paragraph::new(
        stats
            .next
            .clone()
            .map_or("[No next command]".to_string(), |next| next.command),
    )
    .block(if compact {
        Block::new()
            .borders(Borders::NONE)
            .style(theme.as_style(Meaning::Annotation))
    } else {
        Block::new()
            .borders(Borders::ALL)
            .title("Next command")
            .padding(Padding::horizontal(1))
            .style(theme.as_style(Meaning::Annotation))
    });

    f.render_widget(previous, commands[0]);
    f.render_widget(command, commands[1]);
    f.render_widget(next, commands[2]);
}

pub fn draw_stats_table(
    f: &mut Frame<'_>,
    parent: Rect,
    history: &History,
    stats: &HistoryStats,
    theme: &Theme,
) {
    let duration = Duration::from_nanos(u64_or_zero(history.duration));
    let avg_duration = Duration::from_nanos(stats.average_duration);

    let rows = [
        Row::new(vec!["Time".to_string(), history.timestamp.to_string()]),
        Row::new(vec!["Duration".to_string(), format_duration(duration)]),
        Row::new(vec![
            "Avg duration".to_string(),
            format_duration(avg_duration),
        ]),
        Row::new(vec!["Exit".to_string(), history.exit.to_string()]),
        Row::new(vec!["Directory".to_string(), history.cwd.to_string()]),
        Row::new(vec!["Session".to_string(), history.session.to_string()]),
        Row::new(vec!["Total runs".to_string(), stats.total.to_string()]),
    ];

    let widths = [Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)];

    let table = Table::new(rows, widths).column_spacing(1).block(
        Block::default()
            .title("Command stats")
            .borders(Borders::ALL)
            .style(theme.as_style(Meaning::Base))
            .padding(Padding::vertical(1)),
    );

    f.render_widget(table, parent);
}

fn num_to_day(num: &str) -> String {
    match num {
        "0" => "Sunday".to_string(),
        "1" => "Monday".to_string(),
        "2" => "Tuesday".to_string(),
        "3" => "Wednesday".to_string(),
        "4" => "Thursday".to_string(),
        "5" => "Friday".to_string(),
        "6" => "Saturday".to_string(),
        _ => "Invalid day".to_string(),
    }
}

fn sort_duration_over_time(durations: &[(String, i64)]) -> Vec<(String, i64)> {
    let format = format_description!("[day]-[month]-[year]");
    let output = format_description!("[month]/[year repr:last_two]");

    let mut durations: Vec<(time::Date, i64)> = durations
        .iter()
        .map(|d| {
            (
                time::Date::parse(d.0.as_str(), &format).expect("invalid date string from sqlite"),
                d.1,
            )
        })
        .collect();

    durations.sort_by(|a, b| a.0.cmp(&b.0));

    durations
        .iter()
        .map(|(date, duration)| {
            (
                date.format(output).expect("failed to format sqlite date"),
                *duration,
            )
        })
        .collect()
}

fn draw_stats_charts(f: &mut Frame<'_>, parent: Rect, stats: &HistoryStats, theme: &Theme) {
    let exits: Vec<Bar> = stats
        .exits
        .iter()
        .map(|(exit, count)| {
            Bar::default()
                .label(exit.to_string().into())
                .value(u64_or_zero(*count))
        })
        .collect();

    let exits = BarChart::default()
        .block(
            Block::default()
                .title("Exit code distribution")
                .style(theme.as_style(Meaning::Base))
                .borders(Borders::ALL),
        )
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default())
        .value_style(Style::default())
        .label_style(Style::default())
        .data(BarGroup::default().bars(&exits));

    let day_of_week: Vec<Bar> = stats
        .day_of_week
        .iter()
        .map(|(day, count)| {
            Bar::default()
                .label(num_to_day(day.as_str()).into())
                .value(u64_or_zero(*count))
        })
        .collect();

    let day_of_week = BarChart::default()
        .block(
            Block::default()
                .title("Runs per day")
                .style(theme.as_style(Meaning::Base))
                .borders(Borders::ALL),
        )
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default())
        .value_style(Style::default())
        .label_style(Style::default())
        .data(BarGroup::default().bars(&day_of_week));

    let duration_over_time = sort_duration_over_time(&stats.duration_over_time);
    let duration_over_time: Vec<Bar> = duration_over_time
        .iter()
        .map(|(date, duration)| {
            let d = Duration::from_nanos(u64_or_zero(*duration));
            Bar::default()
                .label(date.clone().into())
                .value(u64_or_zero(*duration))
                .text_value(format_duration(d))
        })
        .collect();

    let duration_over_time = BarChart::default()
        .block(
            Block::default()
                .title("Duration over time")
                .style(theme.as_style(Meaning::Base))
                .borders(Borders::ALL),
        )
        .bar_width(5)
        .bar_gap(1)
        .bar_style(Style::default())
        .value_style(Style::default())
        .label_style(Style::default())
        .data(BarGroup::default().bars(&duration_over_time));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(parent);

    f.render_widget(exits, layout[0]);
    f.render_widget(day_of_week, layout[1]);
    f.render_widget(duration_over_time, layout[2]);
}

pub fn draw(
    f: &mut Frame<'_>,
    chunk: Rect,
    history: &History,
    stats: &HistoryStats,
    settings: &Settings,
    theme: &Theme,
) {
    let compact = match settings.style {
        atuin_client::settings::Style::Auto => f.size().height < 14,
        atuin_client::settings::Style::Compact => true,
        atuin_client::settings::Style::Full => false,
    };

    if compact {
        draw_compact(f, chunk, history, stats, theme);
    } else {
        draw_full(f, chunk, history, stats, theme);
    }
}

pub fn draw_compact(
    f: &mut Frame<'_>,
    chunk: Rect,
    history: &History,
    stats: &HistoryStats,
    theme: &Theme,
) {
    draw_commands(f, chunk, history, stats, true, theme);
}

pub fn draw_full(
    f: &mut Frame<'_>,
    chunk: Rect,
    history: &History,
    stats: &HistoryStats,
    theme: &Theme,
) {
    let vert_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)])
        .split(chunk);

    let stats_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(vert_layout[1]);

    draw_commands(f, vert_layout[0], history, stats, false, theme);
    draw_stats_table(f, stats_layout[0], history, stats, theme);
    draw_stats_charts(f, stats_layout[1], stats, theme);
}

// I'm going to break this out more, but just starting to move things around before changing
// structure and making it nicer.
pub fn input(
    state: &mut State,
    _settings: &Settings,
    selected: usize,
    input: &KeyEvent,
) -> InputAction {
    let ctrl = input.modifiers.contains(KeyModifiers::CONTROL);

    match input.code {
        KeyCode::Char('d') if ctrl => InputAction::Delete(selected),
        KeyCode::Up => {
            state.inspecting_state.move_to_previous();
            InputAction::Redraw
        }
        KeyCode::Down => {
            state.inspecting_state.move_to_next();
            InputAction::Redraw
        }
        _ => InputAction::Continue,
    }
}
