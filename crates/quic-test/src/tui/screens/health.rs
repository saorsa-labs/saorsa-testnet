//! Health monitoring tab rendering.
//!
//! Displays system health information:
//! - Overall health indicator and score
//! - Component status grid
//! - Alerts and anomalies lists
//! - Resource usage bars (CPU/Mem/Disk/Net)

use crate::tui::app::App;
use crate::tui::types::{AlertSeverity, HealthStatus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
};

/// Draw the Health tab showing system monitoring.
pub fn draw_health_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Overall health header
            Constraint::Length(10), // Components + Alerts side by side
            Constraint::Min(8),     // Resources + Anomalies
        ])
        .split(area);

    draw_health_header(frame, app, chunks[0]);

    // Middle row: Components and Alerts
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    draw_component_status(frame, app, middle_chunks[0]);
    draw_alerts(frame, app, middle_chunks[1]);

    // Bottom row: Resources and Anomalies
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[2]);

    draw_resource_usage(frame, app, bottom_chunks[0]);
    draw_anomalies(frame, app, bottom_chunks[1]);
}

/// Draw overall health header with score gauge.
fn draw_health_header(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.health_stats;

    let (status_text, status_color) = match stats.status {
        HealthStatus::Healthy => ("HEALTHY", Color::Green),
        HealthStatus::Degraded => ("DEGRADED", Color::Yellow),
        HealthStatus::Unhealthy => ("UNHEALTHY", Color::Red),
        HealthStatus::Critical => ("CRITICAL", Color::Magenta),
        HealthStatus::Unknown => ("UNKNOWN", Color::DarkGray),
    };

    let block = Block::default()
        .title(format!(
            " SYSTEM HEALTH: {} ({:.0}%) ",
            status_text,
            stats.overall_score * 100.0
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(status_color));

    let gauge = Gauge::default()
        .block(block)
        .gauge_style(
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )
        .ratio(stats.overall_score.clamp(0.0, 1.0))
        .label(format!(
            "Score: {:.1}% | Uptime: {} | Last check: {}",
            stats.overall_score * 100.0,
            format_duration(stats.uptime_secs),
            format_ago(stats.last_check_secs_ago),
        ));

    frame.render_widget(gauge, area);
}

/// Draw component status grid.
fn draw_component_status(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.health_stats;

    let healthy_count = stats.components.iter().filter(|c| c.healthy).count();
    let total_count = stats.components.len();

    let block = Block::default()
        .title(format!(" COMPONENTS ({}/{}) ", healthy_count, total_count))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if healthy_count == total_count {
            Color::Green
        } else {
            Color::Yellow
        }));

    let header = Row::new(vec![
        Cell::from("Component").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("St").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Uptime").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Last Error").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().fg(Color::White));

    let rows: Vec<Row> = stats
        .components
        .iter()
        .take(8)
        .map(|comp| {
            let (status_icon, status_color) = if comp.healthy {
                ("●", Color::Green)
            } else {
                ("✗", Color::Red)
            };
            Row::new(vec![
                Cell::from(comp.name.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(status_icon).style(Style::default().fg(status_color)),
                Cell::from(format_duration(comp.uptime_secs)),
                Cell::from(comp.last_error.clone().unwrap_or_else(|| "-".to_string()))
                    .style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(block);

    frame.render_widget(table, area);
}

/// Draw alerts list.
fn draw_alerts(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.health_stats;

    let critical_count = stats
        .alerts
        .iter()
        .filter(|a| matches!(a.severity, AlertSeverity::Critical))
        .count();

    let border_color = if critical_count > 0 {
        Color::Red
    } else if !stats.alerts.is_empty() {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(format!(" ALERTS ({}) ", stats.alerts.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let mut lines: Vec<Line> = Vec::new();

    if stats.alerts.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No active alerts",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for alert in stats.alerts.iter().take(6) {
            let (severity_icon, severity_color) = match alert.severity {
                AlertSeverity::Critical => ("!!", Color::Red),
                AlertSeverity::Error => ("X", Color::LightRed),
                AlertSeverity::Warning => ("!", Color::Yellow),
                AlertSeverity::Info => ("i", Color::Cyan),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", severity_icon),
                    Style::default()
                        .fg(severity_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(alert.message.clone(), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format_ago(alert.timestamp_secs_ago),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Draw resource usage bars.
fn draw_resource_usage(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.health_stats;
    let resources = &stats.resources;

    let block = Block::default()
        .title(" RESOURCE USAGE ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(inner);

    // CPU gauge
    let cpu_color = resource_color(resources.cpu_percent);
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU "))
        .gauge_style(Style::default().fg(cpu_color))
        .ratio((resources.cpu_percent / 100.0).clamp(0.0, 1.0))
        .label(format!("{:.1}%", resources.cpu_percent));
    frame.render_widget(cpu_gauge, chunks[0]);

    // Memory gauge
    let mem_color = resource_color(resources.memory_percent);
    let mem_gauge = Gauge::default()
        .block(Block::default().title(" Memory "))
        .gauge_style(Style::default().fg(mem_color))
        .ratio((resources.memory_percent / 100.0).clamp(0.0, 1.0))
        .label(format!(
            "{:.1}% ({}/{})",
            resources.memory_percent,
            format_bytes(resources.memory_used_bytes),
            format_bytes(resources.memory_total_bytes),
        ));
    frame.render_widget(mem_gauge, chunks[1]);

    // Disk gauge
    let disk_color = resource_color(resources.disk_percent);
    let disk_gauge = Gauge::default()
        .block(Block::default().title(" Disk "))
        .gauge_style(Style::default().fg(disk_color))
        .ratio((resources.disk_percent / 100.0).clamp(0.0, 1.0))
        .label(format!(
            "{:.1}% ({}/{})",
            resources.disk_percent,
            format_bytes(resources.disk_used_bytes),
            format_bytes(resources.disk_total_bytes),
        ));
    frame.render_widget(disk_gauge, chunks[2]);

    // Network gauge (based on bandwidth utilization estimate)
    let net_percent = resources.network_utilization_percent;
    let net_color = resource_color(net_percent);
    let net_gauge = Gauge::default()
        .block(Block::default().title(" Network "))
        .gauge_style(Style::default().fg(net_color))
        .ratio((net_percent / 100.0).clamp(0.0, 1.0))
        .label(format!(
            "↑{}/s ↓{}/s",
            format_bytes(resources.network_tx_bytes_sec),
            format_bytes(resources.network_rx_bytes_sec),
        ));
    frame.render_widget(net_gauge, chunks[3]);
}

/// Draw anomalies list.
fn draw_anomalies(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.health_stats;

    let block = Block::default()
        .title(format!(" ANOMALIES ({}) ", stats.anomalies.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if stats.anomalies.is_empty() {
            Color::DarkGray
        } else {
            Color::Yellow
        }));

    let mut lines: Vec<Line> = Vec::new();

    if stats.anomalies.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No anomalies detected",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for anomaly in stats.anomalies.iter().take(6) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", anomaly.anomaly_type),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("({:.2}σ)", anomaly.deviation),
                    Style::default().fg(if anomaly.deviation >= 3.0 {
                        Color::Red
                    } else {
                        Color::Cyan
                    }),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    anomaly.description.clone(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Get color based on resource usage percentage.
fn resource_color(percent: f64) -> Color {
    if percent >= 90.0 {
        Color::Red
    } else if percent >= 70.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Format duration in seconds to human-readable string.
fn format_duration(secs: u64) -> String {
    if secs >= 86400 {
        format!("{}d", secs / 86400)
    } else if secs >= 3600 {
        format!("{}h", secs / 3600)
    } else if secs >= 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{}s", secs)
    }
}

/// Format seconds ago to human-readable string.
fn format_ago(secs: u64) -> String {
    if secs == 0 {
        "just now".to_string()
    } else if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

/// Format bytes to human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
