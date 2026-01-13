//! DHT (Distributed Hash Table) tab rendering.
//!
//! Displays Kademlia routing table visualization:
//! - K-bucket fill levels by distance
//! - GET/PUT operation statistics
//! - Latency histogram (P50/P95/P99)
//! - Stored records summary

use crate::tui::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph},
};

/// Draw the DHT tab showing Kademlia routing table and operations.
pub fn draw_dht_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // K-buckets + Operations
            Constraint::Length(8),  // Latency stats
            Constraint::Min(6),     // Records summary
        ])
        .split(area);

    // Top row: K-buckets and Operations side by side
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[0]);

    draw_k_buckets(frame, app, top_chunks[0]);
    draw_dht_operations(frame, app, top_chunks[1]);
    draw_latency_stats(frame, app, chunks[1]);
    draw_records_summary(frame, app, chunks[2]);
}

/// Draw K-bucket visualization as a bar chart by XOR distance.
fn draw_k_buckets(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.dht_stats;

    // Create bars for each k-bucket (up to 256 for 256-bit IDs, show first 20)
    let max_buckets = stats.k_buckets.len().min(20);
    let bars: Vec<Bar> = stats
        .k_buckets
        .iter()
        .take(max_buckets)
        .enumerate()
        .map(|(i, &count)| {
            let color = if count >= 8 {
                Color::Green
            } else if count >= 4 {
                Color::Cyan
            } else if count > 0 {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            Bar::default()
                .value(count as u64)
                .label(Line::from(format!("{}", i)))
                .style(Style::default().fg(color))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(format!(
                    " K-BUCKETS ({} peers in {} buckets) ",
                    stats.total_routing_peers,
                    stats.k_buckets.iter().filter(|&&c| c > 0).count()
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(3)
        .bar_gap(1)
        .max(20); // K=20 typical max

    frame.render_widget(bar_chart, area);
}

/// Draw DHT operation statistics.
fn draw_dht_operations(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.dht_stats;
    let ops = &stats.operations;

    let block = Block::default()
        .title(" DHT OPERATIONS ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let get_success_rate = if ops.gets > 0 {
        (ops.get_successes as f64 / ops.gets as f64) * 100.0
    } else {
        0.0
    };

    let put_success_rate = if ops.puts > 0 {
        (ops.put_successes as f64 / ops.puts as f64) * 100.0
    } else {
        0.0
    };

    let lines = vec![
        Line::from(vec![
            Span::raw("  GET:  "),
            Span::styled(
                format!("{}", ops.get_successes),
                Style::default().fg(Color::Green),
            ),
            Span::raw("/"),
            Span::styled(format!("{}", ops.gets), Style::default().fg(Color::White)),
            Span::raw(" ("),
            Span::styled(
                format!("{:.0}%", get_success_rate),
                Style::default().fg(if get_success_rate >= 90.0 {
                    Color::Green
                } else if get_success_rate >= 70.0 {
                    Color::Cyan
                } else {
                    Color::Red
                }),
            ),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::raw("  PUT:  "),
            Span::styled(
                format!("{}", ops.put_successes),
                Style::default().fg(Color::Green),
            ),
            Span::raw("/"),
            Span::styled(format!("{}", ops.puts), Style::default().fg(Color::White)),
            Span::raw(" ("),
            Span::styled(
                format!("{:.0}%", put_success_rate),
                Style::default().fg(if put_success_rate >= 90.0 {
                    Color::Green
                } else if put_success_rate >= 70.0 {
                    Color::Cyan
                } else {
                    Color::Red
                }),
            ),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::raw("  DEL:  "),
            Span::styled(
                format!("{}", ops.delete_successes),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("/"),
            Span::styled(
                format!("{}", ops.deletes),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Routing: "),
            Span::styled(
                format!("{}", ops.routing_successes),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("/"),
            Span::styled(
                format!("{}", ops.routing_queries),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Records: "),
            Span::styled(
                format!("{}", stats.stored_records),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Draw latency statistics.
fn draw_latency_stats(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.dht_stats;
    let latency = &stats.latency;

    let block = Block::default()
        .title(format!(
            " LATENCY (avg: {:.1}ms, {} samples) ",
            latency.avg_ms, latency.samples
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let lines = vec![
        Line::from(vec![
            Span::raw("  Min: "),
            Span::styled(
                format!("{}ms", latency.min_ms),
                Style::default().fg(Color::Green),
            ),
            Span::raw("    Max: "),
            Span::styled(
                format!("{}ms", latency.max_ms),
                Style::default().fg(if latency.max_ms > 1000 {
                    Color::Red
                } else {
                    Color::Yellow
                }),
            ),
            Span::raw("    Avg: "),
            Span::styled(
                format!("{:.1}ms", latency.avg_ms),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("  P50: "),
            Span::styled(
                format!("{}ms", latency.p50_ms),
                Style::default().fg(Color::Green),
            ),
            Span::raw("    P95: "),
            Span::styled(
                format!("{}ms", latency.p95_ms),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("    P99: "),
            Span::styled(
                format!("{}ms", latency.p99_ms),
                Style::default().fg(if latency.p99_ms > 500 {
                    Color::Red
                } else {
                    Color::Yellow
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  History: "),
            Span::styled(
                latency
                    .history
                    .iter()
                    .rev()
                    .take(30)
                    .map(|&v| {
                        if v < 50 {
                            '▁'
                        } else if v < 100 {
                            '▂'
                        } else if v < 200 {
                            '▄'
                        } else if v < 500 {
                            '▆'
                        } else {
                            '█'
                        }
                    })
                    .collect::<String>(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Draw records summary by type.
fn draw_records_summary(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.dht_stats;

    let block = Block::default()
        .title(format!(
            " STORED RECORDS ({}) - Replication: {} ",
            stats.stored_records, stats.replication_factor
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let mut lines: Vec<Line> = Vec::new();

    if stats.records_by_type.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No records stored yet",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // Sort record types by count descending
        let mut sorted: Vec<_> = stats.records_by_type.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));

        for (record_type, count) in sorted.iter().take(5) {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<15}", record_type),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(format!("{:>6}", count), Style::default().fg(Color::White)),
            ]));
        }
    }

    // Add distance samples visualization if available
    if !stats.distance_samples.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  Distance distribution: "),
            Span::styled(
                stats
                    .distance_samples
                    .iter()
                    .take(20)
                    .map(|&d| match d / 26 {
                        // 0-9 range from 0-255
                        0..=2 => '▁',
                        3..=4 => '▂',
                        5..=6 => '▄',
                        7..=8 => '▆',
                        _ => '█',
                    })
                    .collect::<String>(),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
