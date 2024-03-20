use std::time::Duration;
use time::macros::format_description;

use atuin_client::{
    history::{History, HistoryStats},
    settings::Settings,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    prelude::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Padding, Paragraph, Row, Table},
    Frame,
};

use super::duration::format_duration;

use super::interactive::{InputAction, State};
use super::super::theme::{Theme, Meaning};

#[allow(clippy::cast_sign_loss)]
fn u64_or_zero(num: i64) -> u64 {
    if num < 0 {
        0
    } else {
        num as u64
    }
}

fn command_time_diff(command_a: &History, command_b: &History) -> String {
    format![
        "{:+.0}h{}m{}",
        ((command_a.timestamp - command_b.timestamp).whole_seconds() as f32) / 3600.,
        ((command_a.timestamp - command_b.timestamp).whole_seconds() / 60).abs() % 60,
        ((command_a.timestamp - command_b.timestamp).whole_seconds() % 60).abs(),
    ]
}
fn layout_command_block<'a>(f: &mut Frame<'a>, compact: bool, title: String, parent: Rect, superscript: Paragraph, body: Paragraph) -> Block<'a> {
    let command = if compact {
        Block::new()
            .borders(Borders::NONE)
    } else {
        Block::new()
            .borders(Borders::ALL)
            .title(title)
            .padding(Padding::horizontal(1))
    };
    let command_layout = Layout::default()
        .direction(if compact { Direction::Horizontal } else { Direction::Vertical })
        .constraints(
            [
                Constraint::Length(if compact { 10 } else { 1 }),
                Constraint::Min(0),
            ]
        )
        .split(command.inner(parent));
    f.render_widget(superscript, command_layout[0]);
    f.render_widget(body, command_layout[1]);
    command
}

pub fn draw_commands(f: &mut Frame<'_>, parent: Rect, history: &History, stats: &HistoryStats, focus: &History, compact: bool, theme: &Theme) {
    let help = Paragraph::new(Text::from(Span::styled(
        format!("[Up/Down to step through session by timestamp]"),
        theme.as_style(Meaning::Guidance).add_modifier(Modifier::BOLD),
    )));
    let commands_block = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(0),
            ]
        )
        .split(parent);
    let commands = Layout::default()
        .direction(if compact { Direction::Vertical } else { Direction::Horizontal })
        .constraints(
            if compact {
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
            }
        )
        .split(commands_block[1]);

    [("Previous command", &stats.previous, 0), ("Next command", &stats.next, 2)].iter().for_each(|(title, command, loc)| {
        let time_offset = (*command)
            .clone()
            .map_or(
                Paragraph::new(
                    "~".to_string()
                ).style(Style::default().fg(Color::DarkGray)),
                |command: History| Paragraph::new(
                    if compact {
                        command_time_diff(&command, history)
                    } else {
                        format![
                            "{} ({})",
                            command_time_diff(&command, history),
                            command_time_diff(&command, focus)
                        ]
                    }
                ).style(Style::default().fg(Color::DarkGray)),
            );
        let text = (*command)
            .clone()
            .map_or(
                Paragraph::new(
                    "[No previous command]".to_string()
                ).style(Style::default().fg(Color::DarkGray)),
                |prev| Paragraph::new(prev.command)
            );
        let command_block = layout_command_block(f, compact, title.to_string(), commands[*loc], time_offset, text);
        f.render_widget(command_block, commands[*loc]);
    });

    let focus_command = if focus == history {
        Paragraph::new("-")
    } else {
        if compact {
            Paragraph::new("|")
        } else {
            Paragraph::new(
                format![
                    "({} from inspected command: {})",
                    command_time_diff(history, focus),
                    focus.command,
                ]
            )
        }
    }.style(Style::default().fg(Color::DarkGray));
    let text = if compact {
        Paragraph::new(
            Text::from(Span::styled(
                history.command.clone(),
                Style::default().add_modifier(Modifier::BOLD).fg(Color::White),
            ))
        )
    } else {
        Paragraph::new(
            Text::from(Span::styled(
                history.command.clone(),
                Style::default().add_modifier(Modifier::BOLD).fg(Color::White),
            ))
        )
    };
    let command = layout_command_block(f, compact, "Command".to_string(), commands[1], focus_command, text);

    f.render_widget(help, commands_block[0]);
    f.render_widget(command, commands[1]);
}

pub fn draw_stats_table(f: &mut Frame<'_>, parent: Rect, history: &History, stats: &HistoryStats) {
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

fn draw_stats_charts(f: &mut Frame<'_>, parent: Rect, stats: &HistoryStats) {
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
        .block(Block::default().title("Runs per day").borders(Borders::ALL))
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

pub fn draw(f: &mut Frame<'_>, chunk: Rect, history: &History, stats: &HistoryStats, settings: &Settings, focus: &History, theme: &Theme) {
    let compact = match settings.style {
        atuin_client::settings::Style::Auto => f.size().height < 14,
        atuin_client::settings::Style::Compact => true,
        atuin_client::settings::Style::Full => false,
    };

    if compact {
        draw_compact(f, chunk, history, stats, focus, theme)
    } else {
        draw_full(f, chunk, history, stats, focus, theme)
    }
}

pub fn draw_compact(f: &mut Frame<'_>, chunk: Rect, history: &History, stats: &HistoryStats, focus: &History, theme: &Theme) {
    draw_commands(f, chunk, history, stats, focus, true, theme);
}

pub fn draw_full(f: &mut Frame<'_>, chunk: Rect, history: &History, stats: &HistoryStats, focus: &History, theme: &Theme) {
    let vert_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)])
        .split(chunk);

    let stats_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(vert_layout[1]);

    draw_commands(f, vert_layout[0], history, stats, focus, false, theme);
    draw_stats_table(f, stats_layout[0], history, stats);
    draw_stats_charts(f, stats_layout[1], stats);
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
            state.inspecting_state.to_previous();
            InputAction::Redraw
        },
        KeyCode::Down => {
            state.inspecting_state.to_next();
            InputAction::Redraw
        },
        _ => InputAction::Continue,
    }
}
