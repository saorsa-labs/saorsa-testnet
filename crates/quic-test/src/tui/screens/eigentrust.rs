//! EigenTrust reputation tab rendering.
//!
//! Displays trust score information:
//! - Local node's global trust score gauge
//! - Trusted peers table (sorted by score)
//! - Suspicious/low-trust peer detection
//! - Trust evolution over time

use crate::tui::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
};

/// Draw the EigenTrust tab showing reputation scores.
pub fn draw_eigentrust_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Local trust gauge
            Constraint::Length(10), // Trusted peers / Suspicious peers
            Constraint::Min(6),     // Trust evolution
        ])
        .split(area);

    draw_local_trust_gauge(frame, app, chunks[0]);

    // Middle row: Trusted and Suspicious peers side by side
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    draw_trusted_peers(frame, app, middle_chunks[0]);
    draw_suspicious_peers(frame, app, middle_chunks[1]);
    draw_trust_evolution(frame, app, chunks[2]);
}

/// Draw local node's trust score as a gauge.
fn draw_local_trust_gauge(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.eigentrust_stats;

    let score = stats.local_trust_score;
    let color = trust_score_color(score);

    let convergence_status = if stats.converged {
        Span::styled(" [Converged]", Style::default().fg(Color::Green))
    } else {
        Span::styled(
            format!(" [Iter: {}]", stats.convergence_iterations),
            Style::default().fg(Color::Cyan),
        )
    };

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" LOCAL TRUST SCORE "),
            convergence_status,
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));

    // Gauge showing 0.0 to 1.0 trust score
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .ratio(score.clamp(0.0, 1.0))
        .label(format!("{:.4} / 1.0", score));

    frame.render_widget(gauge, area);
}

/// Draw table of trusted peers sorted by score.
fn draw_trusted_peers(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.eigentrust_stats;

    let block = Block::default()
        .title(format!(
            " TRUSTED PEERS ({}) ",
            stats.peer_trust_scores.len()
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let header = Row::new(vec![
        Cell::from("Peer").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Score").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Txns").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().fg(Color::White));

    // Sort by score descending, take top entries
    let mut sorted_peers = stats.peer_trust_scores.clone();
    sorted_peers.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let rows: Vec<Row> = sorted_peers
        .iter()
        .take(8)
        .map(|entry| {
            let pre_trust_marker = if entry.pre_trusted { "*" } else { "" };
            Row::new(vec![
                Cell::from(format!("{}{}", entry.short_id, pre_trust_marker)).style(
                    Style::default().fg(if entry.pre_trusted {
                        Color::Yellow
                    } else {
                        Color::Cyan
                    }),
                ),
                Cell::from(format!("{:.4}", entry.score))
                    .style(Style::default().fg(trust_score_color(entry.score))),
                Cell::from(format!("{}", entry.transactions))
                    .style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(6),
        ],
    )
    .header(header)
    .block(block);

    frame.render_widget(table, area);
}

/// Draw suspicious/low-trust peer alerts.
fn draw_suspicious_peers(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.eigentrust_stats;

    let suspicious_count = stats
        .peer_trust_scores
        .iter()
        .filter(|e| e.suspicious || e.score < stats.trust_threshold)
        .count();

    let border_color = if suspicious_count > 0 {
        Color::Red
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(format!(" SUSPICIOUS ({}) ", suspicious_count))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let mut lines: Vec<Line> = Vec::new();

    // Show peers marked suspicious or below threshold
    let suspicious: Vec<_> = stats
        .peer_trust_scores
        .iter()
        .filter(|e| e.suspicious || e.score < stats.trust_threshold)
        .collect();

    if suspicious.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No suspicious peers",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for entry in suspicious.iter().take(6) {
            let severity = if entry.score < 0.1 {
                ("!!", Color::Red)
            } else {
                ("!", Color::Yellow)
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", severity.0), Style::default().fg(severity.1)),
                Span::styled(entry.short_id.clone(), Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(
                    format!("{:.3}", entry.score),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Draw trust score evolution/history.
fn draw_trust_evolution(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.eigentrust_stats;

    let block = Block::default()
        .title(" TRUST EVOLUTION ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let mut lines: Vec<Line> = Vec::new();

    // Show convergence info
    lines.push(Line::from(vec![
        Span::raw("  Iterations: "),
        Span::styled(
            format!("{}", stats.convergence_iterations),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("    Trust threshold: "),
        Span::styled(
            format!("{:.4}", stats.trust_threshold),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Pre-trusted peers: "),
        Span::styled(
            format!("{}", stats.pre_trusted_count),
            Style::default().fg(Color::Green),
        ),
        Span::raw("    Suspicious: "),
        Span::styled(
            format!("{}", stats.suspicious_count),
            Style::default().fg(if stats.suspicious_count > 0 {
                Color::Red
            } else {
                Color::DarkGray
            }),
        ),
    ]));

    // Trust history sparkline (simplified as text for now)
    if !stats.trust_history.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  History: "),
            Span::styled(
                stats
                    .trust_history
                    .iter()
                    .rev()
                    .take(20)
                    .map(|&v| {
                        if v > 0.7 {
                            '█'
                        } else if v > 0.4 {
                            '▄'
                        } else {
                            '▁'
                        }
                    })
                    .collect::<String>(),
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Get color for trust score (green > 0.7, cyan > 0.4, yellow > 0.2, red otherwise).
fn trust_score_color(score: f64) -> Color {
    if score >= 0.7 {
        Color::Green
    } else if score >= 0.4 {
        Color::Cyan
    } else if score >= 0.2 {
        Color::Yellow
    } else {
        Color::Red
    }
}
