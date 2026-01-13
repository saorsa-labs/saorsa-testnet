//! Placement tab rendering.
//!
//! Displays data placement and diversity metrics:
//! - Geographic diversity gauges
//! - Rack and network diversity scores
//! - Regional distribution table
//! - Placement success statistics

use crate::tui::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
};

/// Draw the Placement tab showing diversity metrics.
pub fn draw_placement_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Diversity gauges
            Constraint::Length(10), // Regional distribution
            Constraint::Min(6),     // Placement stats
        ])
        .split(area);

    draw_diversity_gauges(frame, app, chunks[0]);
    draw_regional_distribution(frame, app, chunks[1]);
    draw_placement_stats(frame, app, chunks[2]);
}

/// Draw diversity gauges (geographic, rack, network).
fn draw_diversity_gauges(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.placement_stats;

    let block = Block::default()
        .title(" DIVERSITY METRICS ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(block.inner(area));

    frame.render_widget(block, area);

    // Geographic diversity gauge
    let geo_color = diversity_color(stats.geographic_diversity);
    let geo_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Geographic ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(geo_color)),
        )
        .gauge_style(Style::default().fg(geo_color))
        .ratio(stats.geographic_diversity.clamp(0.0, 1.0))
        .label(format!("{:.1}%", stats.geographic_diversity * 100.0));
    frame.render_widget(geo_gauge, chunks[0]);

    // Rack diversity gauge
    let rack_color = diversity_color(stats.rack_diversity);
    let rack_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Rack ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(rack_color)),
        )
        .gauge_style(Style::default().fg(rack_color))
        .ratio(stats.rack_diversity.clamp(0.0, 1.0))
        .label(format!("{:.1}%", stats.rack_diversity * 100.0));
    frame.render_widget(rack_gauge, chunks[1]);

    // Network diversity gauge
    let net_color = diversity_color(stats.network_diversity);
    let net_gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Network ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(net_color)),
        )
        .gauge_style(Style::default().fg(net_color))
        .ratio(stats.network_diversity.clamp(0.0, 1.0))
        .label(format!("{:.1}%", stats.network_diversity * 100.0));
    frame.render_widget(net_gauge, chunks[2]);
}

/// Draw regional distribution table.
fn draw_regional_distribution(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.placement_stats;

    let block = Block::default()
        .title(format!(
            " REGIONAL DISTRIBUTION ({} regions) ",
            stats.regions.len()
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let header = Row::new(vec![
        Cell::from("Region").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Nodes").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Data %").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Latency").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().fg(Color::White));

    // Sort regions by node count descending
    let mut sorted_regions = stats.regions.clone();
    sorted_regions.sort_by(|a, b| b.node_count.cmp(&a.node_count));

    let rows: Vec<Row> = sorted_regions
        .iter()
        .take(8)
        .map(|region| {
            let status_color = if region.healthy {
                Color::Green
            } else {
                Color::Red
            };
            Row::new(vec![
                Cell::from(format!("{} {}", region.flag, region.name))
                    .style(Style::default().fg(Color::Cyan)),
                Cell::from(format!("{}", region.node_count)),
                Cell::from(format!("{:.1}%", region.data_percentage)),
                Cell::from(format!("{}ms", region.avg_latency_ms as u64)),
                Cell::from(if region.healthy { "OK" } else { "!!" })
                    .style(Style::default().fg(status_color)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Min(6),
        ],
    )
    .header(header)
    .block(block);

    frame.render_widget(table, area);
}

/// Draw placement statistics and targets.
fn draw_placement_stats(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.placement_stats;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Placement operations
    let ops_block = Block::default()
        .title(" PLACEMENT OPERATIONS ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let success_rate = if stats.total_placements > 0 {
        (stats.successful_placements as f64 / stats.total_placements as f64) * 100.0
    } else {
        0.0
    };

    let ops_lines = vec![
        Line::from(vec![
            Span::raw("  Total:      "),
            Span::styled(
                format!("{}", stats.total_placements),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Successful: "),
            Span::styled(
                format!("{}", stats.successful_placements),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" ("),
            Span::styled(
                format!("{:.1}%", success_rate),
                Style::default().fg(if success_rate >= 95.0 {
                    Color::Green
                } else if success_rate >= 80.0 {
                    Color::Cyan
                } else {
                    Color::Yellow
                }),
            ),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::raw("  Retries:    "),
            Span::styled(
                format!("{}", stats.placement_retries),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Failures:   "),
            Span::styled(
                format!("{}", stats.failed_placements),
                Style::default().fg(if stats.failed_placements > 0 {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(ops_lines).block(ops_block), chunks[0]);

    // Right: Replication targets
    let targets_block = Block::default()
        .title(" REPLICATION TARGETS ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let targets_lines = vec![
        Line::from(vec![
            Span::raw("  Target Replicas: "),
            Span::styled(
                format!("{}", stats.target_replicas),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Min Regions:     "),
            Span::styled(
                format!("{}", stats.min_regions),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Avg Replicas:    "),
            Span::styled(
                format!("{:.1}", stats.avg_replica_count),
                Style::default().fg(if stats.avg_replica_count >= stats.target_replicas as f64 {
                    Color::Green
                } else {
                    Color::Yellow
                }),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Under-replicated:"),
            Span::styled(
                format!(" {}", stats.under_replicated_count),
                Style::default().fg(if stats.under_replicated_count > 0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(targets_lines).block(targets_block),
        chunks[1],
    );
}

/// Get color based on diversity score.
fn diversity_color(score: f64) -> Color {
    if score >= 0.8 {
        Color::Green
    } else if score >= 0.6 {
        Color::Cyan
    } else if score >= 0.4 {
        Color::Yellow
    } else {
        Color::Red
    }
}
