//! Adaptive network tab rendering.
//!
//! Displays machine learning optimization metrics:
//! - Thompson Sampling arm values (multi-armed bandit)
//! - Q-Learning cache hit/miss statistics
//! - Churn prediction accuracy
//! - Strategy performance comparison

use crate::tui::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Cell, Paragraph, Row, Table},
};

/// Draw the Adaptive tab showing ML optimization metrics.
pub fn draw_adaptive_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Thompson Sampling + Q-Learning
            Constraint::Length(8),  // Churn prediction
            Constraint::Min(6),     // Strategy performance
        ])
        .split(area);

    // Top row: Thompson Sampling and Q-Learning side by side
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[0]);

    draw_thompson_sampling(frame, app, top_chunks[0]);
    draw_q_learning(frame, app, top_chunks[1]);
    draw_churn_prediction(frame, app, chunks[1]);
    draw_strategy_performance(frame, app, chunks[2]);
}

/// Draw Thompson Sampling arm values as bar chart.
fn draw_thompson_sampling(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.adaptive_stats.thompson_sampling;

    let bars: Vec<Bar> = stats
        .arms
        .iter()
        .map(|arm| {
            let color = if arm.estimated_prob >= 0.8 {
                Color::Green
            } else if arm.estimated_prob >= 0.5 {
                Color::Cyan
            } else if arm.estimated_prob >= 0.3 {
                Color::Yellow
            } else {
                Color::Red
            };
            Bar::default()
                .value((arm.estimated_prob * 100.0) as u64)
                .label(Line::from(arm.name.clone()))
                .style(Style::default().fg(color))
        })
        .collect();

    let best_arm_name = stats
        .best_arm
        .and_then(|idx| stats.arms.get(idx))
        .map(|a| a.name.as_str())
        .unwrap_or("none");

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(format!(" THOMPSON SAMPLING (best: {}) ", best_arm_name))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(8)
        .bar_gap(2)
        .max(100);

    frame.render_widget(bar_chart, area);
}

/// Draw Q-Learning cache statistics.
fn draw_q_learning(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.adaptive_stats.q_learning;

    let hit_rate = stats.cache_hit_rate;

    let block = Block::default()
        .title(format!(" Q-LEARNING (hit rate: {:.1}%) ", hit_rate))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let lines = vec![
        Line::from(vec![
            Span::raw("  Hit Rate:     "),
            Span::styled(
                format!("{:.1}%", stats.cache_hit_rate),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Miss Rate:    "),
            Span::styled(
                format!("{:.1}%", stats.cache_miss_rate),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::raw("  States:       "),
            Span::styled(
                format!("{}", stats.state_count),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("  Actions: "),
            Span::styled(
                format!("{}", stats.action_count),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Epsilon:      "),
            Span::styled(
                format!("{:.3}", stats.epsilon),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw("  (exploration)"),
        ]),
        Line::from(vec![
            Span::raw("  Learn Rate:   "),
            Span::styled(
                format!("{:.3}", stats.learning_rate),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Discount:     "),
            Span::styled(
                format!("{:.3}", stats.discount_factor),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("  Episodes: "),
            Span::styled(
                format!("{}", stats.episodes),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Draw churn prediction statistics.
fn draw_churn_prediction(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.adaptive_stats.churn_prediction;

    let accuracy = stats.accuracy;

    let accuracy_color = if accuracy >= 80.0 {
        Color::Green
    } else if accuracy >= 60.0 {
        Color::Cyan
    } else {
        Color::Yellow
    };

    let block = Block::default()
        .title(format!(" CHURN PREDICTION (accuracy: {:.1}%) ", accuracy))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accuracy_color));

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(block.inner(area));

    // Left side: prediction stats
    let stats_lines = vec![
        Line::from(vec![
            Span::raw("  Predictions: "),
            Span::styled(
                format!("{}", stats.predictions),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Correct:     "),
            Span::styled(
                format!("{}", stats.correct),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("  False Pos:   "),
            Span::styled(
                format!("{}", stats.false_positives),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("  False Neg:   "),
            Span::styled(
                format!("{}", stats.false_negatives),
                Style::default().fg(Color::Red),
            ),
        ]),
    ];

    // Right side: at-risk peers
    let mut risk_lines = vec![Line::from(Span::styled(
        "  At-Risk Peers:",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))];

    for peer in stats.at_risk_peers.iter().take(4) {
        risk_lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(peer.short_id.clone(), Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(
                format!("{:.0}%", peer.risk * 100.0),
                Style::default().fg(if peer.risk >= 0.8 {
                    Color::Red
                } else {
                    Color::Yellow
                }),
            ),
        ]));
    }

    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(stats_lines), chunks[0]);
    frame.render_widget(Paragraph::new(risk_lines), chunks[1]);
}

/// Draw strategy performance comparison table.
fn draw_strategy_performance(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.adaptive_stats.strategy_performance;

    let block = Block::default()
        .title(format!(
            " STRATEGY PERFORMANCE (active: {}) ",
            stats.active_strategy
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let header = Row::new(vec![
        Cell::from("Strategy").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Score").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().fg(Color::White));

    // Build rows from the individual strategy scores
    let strategies = [
        (
            "Thompson",
            stats.thompson_score,
            stats.active_strategy == "thompson",
        ),
        (
            "Q-Learning",
            stats.qlearning_score,
            stats.active_strategy == "qlearning",
        ),
        (
            "Random",
            stats.random_score,
            stats.active_strategy == "random",
        ),
    ];

    let rows: Vec<Row> = strategies
        .iter()
        .map(|(name, score, is_active)| {
            let name_style = if *is_active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Row::new(vec![
                Cell::from(if *is_active {
                    format!("* {}", name)
                } else {
                    format!("  {}", name)
                })
                .style(name_style),
                Cell::from(format!("{:.2}", score)).style(Style::default().fg(if *score >= 0.8 {
                    Color::Green
                } else if *score >= 0.5 {
                    Color::Cyan
                } else {
                    Color::Yellow
                })),
                Cell::from(if *is_active { "ACTIVE" } else { "-" }).style(Style::default().fg(
                    if *is_active {
                        Color::Green
                    } else {
                        Color::DarkGray
                    },
                )),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Min(8),
        ],
    )
    .header(header)
    .block(block);

    frame.render_widget(table, area);
}
