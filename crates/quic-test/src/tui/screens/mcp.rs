//! MCP (Model Context Protocol) tab rendering.
//!
//! Displays MCP client interface:
//! - Connection status and server info
//! - Available tools list
//! - Tool details and parameter inputs
//! - Invocation history

use crate::tui::app::App;
use crate::tui::types::McpConnectionStatus;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
};

/// Draw the MCP tab showing AI tool integration.
pub fn draw_mcp_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Connection status header
            Constraint::Length(14), // Tools list + Tool details
            Constraint::Min(6),     // Invocation history
        ])
        .split(area);

    draw_connection_header(frame, app, chunks[0]);

    // Middle row: Tools list and Tool details
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(chunks[1]);

    draw_tools_list(frame, app, middle_chunks[0]);
    draw_tool_details(frame, app, middle_chunks[1]);
    draw_invocation_history(frame, app, chunks[2]);
}

/// Draw MCP connection status header.
fn draw_connection_header(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.mcp_state;

    let (status_text, status_color) = match state.connection {
        McpConnectionStatus::Connected => ("CONNECTED", Color::Green),
        McpConnectionStatus::Connecting => ("CONNECTING...", Color::Cyan),
        McpConnectionStatus::Disconnected => ("DISCONNECTED", Color::DarkGray),
        McpConnectionStatus::Error => ("ERROR", Color::Red),
    };

    let block = Block::default()
        .title(format!(" MCP CLIENT: {} ", status_text))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(status_color));

    let lines = if let Some(ref info) = state.server_info {
        vec![
            Line::from(vec![
                Span::raw("  Server: "),
                Span::styled(&info.name, Style::default().fg(Color::Cyan)),
                Span::raw("  Version: "),
                Span::styled(&info.version, Style::default().fg(Color::White)),
                Span::raw("  Protocol: "),
                Span::styled(&info.protocol_version, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::raw("  Tools: "),
                Span::styled(
                    format!("{}", state.tools.len()),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  Invocations: "),
                Span::styled(
                    format!("{}", state.history.len()),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw("  Endpoint: "),
                Span::styled(
                    state.endpoint.clone().unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "  Not connected - press [C] to connect",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Draw available tools list.
fn draw_tools_list(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.mcp_state;

    let block = Block::default()
        .title(format!(" TOOLS ({}) ", state.tools.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let items: Vec<ListItem> = state
        .tools
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            let is_selected = state.selected_tool == Some(i);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    if is_selected { "▶ " } else { "  " },
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(&tool.name, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(list, area);
}

/// Draw selected tool details.
fn draw_tool_details(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.mcp_state;

    let block = Block::default()
        .title(" TOOL DETAILS ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let lines = if let Some(idx) = state.selected_tool {
        if let Some(tool) = state.tools.get(idx) {
            let mut lines = vec![
                Line::from(vec![Span::styled(
                    &tool.name,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(Span::styled(
                    &tool.description,
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Parameters:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            for param in &tool.parameters {
                let required_marker = if param.required { "*" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}{}", param.name, required_marker),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(": "),
                    Span::styled(&param.param_type, Style::default().fg(Color::DarkGray)),
                ]));
                if let Some(ref desc) = param.description {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(desc, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press [Enter] to invoke",
                Style::default().fg(Color::DarkGray),
            )));

            lines
        } else {
            vec![Line::from(Span::styled(
                "Tool not found",
                Style::default().fg(Color::Red),
            ))]
        }
    } else {
        vec![
            Line::from(Span::styled(
                "  Select a tool from the list",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  [↑/↓] Navigate tools",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  [Enter] Invoke selected tool",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  [C] Connect/Reconnect",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Draw invocation history table.
fn draw_invocation_history(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::types::McpInvocationResult;

    let state = &app.mcp_state;

    let success_count = state
        .history
        .iter()
        .filter(|h| matches!(h.result, McpInvocationResult::Success(_)))
        .count();
    let total_count = state.history.len();

    let block = Block::default()
        .title(format!(
            " INVOCATION HISTORY ({}/{} success) ",
            success_count, total_count
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let header = Row::new(vec![
        Cell::from("St").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Tool").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Time").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Duration").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Result").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().fg(Color::White));

    let rows: Vec<Row> = state
        .history
        .iter()
        .rev()
        .take(8)
        .map(|invoc| {
            use crate::tui::types::McpInvocationResult;

            let (status_icon, status_color, result_preview) = match &invoc.result {
                McpInvocationResult::Success(output) => {
                    let preview = if output.len() > 30 {
                        format!("{}...", &output[..30])
                    } else {
                        output.clone()
                    };
                    ("✓", Color::Green, preview)
                }
                McpInvocationResult::Error(err) => {
                    let preview = format!("ERR: {}", if err.len() > 25 { &err[..25] } else { err });
                    ("✗", Color::Red, preview)
                }
                McpInvocationResult::Pending => ("⏳", Color::Yellow, "pending...".to_string()),
            };

            let timestamp_secs_ago = invoc.timestamp.elapsed().as_secs();
            let duration_ms = invoc.duration.as_millis();

            Row::new(vec![
                Cell::from(status_icon).style(Style::default().fg(status_color)),
                Cell::from(invoc.tool_name.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(format_timestamp(timestamp_secs_ago)),
                Cell::from(format!("{}ms", duration_ms)),
                Cell::from(result_preview).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(block);

    frame.render_widget(table, area);
}

/// Format timestamp to relative time string.
fn format_timestamp(secs_ago: u64) -> String {
    if secs_ago < 60 {
        format!("{}s ago", secs_ago)
    } else if secs_ago < 3600 {
        format!("{}m ago", secs_ago / 60)
    } else {
        format!("{}h ago", secs_ago / 3600)
    }
}
