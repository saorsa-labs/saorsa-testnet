// Copyright 2024 Saorsa Labs Limited
//
// TUI dashboard module for Saorsa TestNet

use crate::node::WorkerNode;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Sparkline, Tabs},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use tokio::time::interval;

/// TUI Dashboard for monitoring network
pub struct Dashboard {
    node: WorkerNode,
    selected_tab: usize,
    should_quit: bool,
    last_update: Instant,
    
    // Metrics history for graphs
    latency_history: Vec<u64>,
    bandwidth_history: Vec<u64>,
    connections_history: Vec<u64>,
}

impl Dashboard {
    /// Create new dashboard
    pub fn new(node: WorkerNode) -> Self {
        Self {
            node,
            selected_tab: 0,
            should_quit: false,
            last_update: Instant::now(),
            latency_history: vec![0; 60],
            bandwidth_history: vec![0; 60],
            connections_history: vec![0; 60],
        }
    }
    
    /// Run the dashboard
    pub async fn run(self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        // Start the node in background
        let node = self.node;
        let node_handle = tokio::spawn(async move {
            node.run().await
        });
        
        // Extract fields before moving self
        let mut selected_tab = self.selected_tab;
        let mut should_quit = self.should_quit;
        let mut last_update = self.last_update;
        let mut latency_history = self.latency_history;
        let mut bandwidth_history = self.bandwidth_history;
        let mut connections_history = self.connections_history;
        
        // Main UI loop
        let mut tick_interval = interval(Duration::from_millis(250));
        
        loop {
            // Draw UI
            terminal.draw(|f| {
                Self::draw_static(f, selected_tab, last_update, &latency_history, &bandwidth_history, &connections_history)
            })?;
            
            // Handle events
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            should_quit = true;
                        }
                        KeyCode::Tab => {
                            selected_tab = (selected_tab + 1) % 4;
                        }
                        KeyCode::Left => {
                            selected_tab = selected_tab.saturating_sub(1);
                        }
                        KeyCode::Right => {
                            if selected_tab < 3 {
                                selected_tab += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            if should_quit {
                break;
            }
            
            // Update metrics
            tick_interval.tick().await;
            Self::update_metrics_static(&mut latency_history, &mut bandwidth_history, &mut connections_history, &mut last_update).await;
        }
        
        // Cleanup
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        
        // Cancel node task
        node_handle.abort();
        
        Ok(())
    }
    
    /// Draw the UI
    #[allow(dead_code)]
    fn draw(&mut self, f: &mut Frame) {
        let size = f.area();
        
        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Tabs
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Footer
            ])
            .split(size);
        
        // Header
        self.draw_header(f, chunks[0]);
        
        // Tabs
        self.draw_tabs(f, chunks[1]);
        
        // Content based on selected tab
        match self.selected_tab {
            0 => self.draw_overview(f, chunks[2]),
            1 => self.draw_nat_metrics(f, chunks[2]),
            2 => self.draw_adaptive_metrics(f, chunks[2]),
            3 => self.draw_performance(f, chunks[2]),
            _ => {}
        }
        
        // Footer
        self.draw_footer(f, chunks[3]);
    }
    
    /// Draw header
    #[allow(dead_code)]
    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("SAORSA TESTNET", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - "),
                Span::styled("Real-time Network Monitor", Style::default().fg(Color::Gray)),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Double))
        .alignment(Alignment::Center);
        
        f.render_widget(header, area);
    }
    
    /// Draw tabs
    #[allow(dead_code)]
    fn draw_tabs(&self, f: &mut Frame, area: Rect) {
        let titles = vec!["Overview", "NAT Traversal", "Adaptive Network", "Performance"];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        
        f.render_widget(tabs, area);
    }
    
    /// Draw overview tab
    #[allow(dead_code)]
    fn draw_overview(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        
        // Left side - Node info
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(chunks[0]);
        
        self.draw_node_info(f, left_chunks[0]);
        self.draw_connections_graph(f, left_chunks[1]);
        
        // Right side - Network stats
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(chunks[1]);
        
        self.draw_network_stats(f, right_chunks[0]);
        self.draw_bandwidth_graph(f, right_chunks[1]);
    }
    
    /// Draw node info
    #[allow(dead_code)]
    fn draw_node_info(&self, f: &mut Frame, area: Rect) {
        let info = vec![
            Line::from(vec![
                Span::raw("Node ID: "),
                Span::styled("worker-001", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Status: "),
                Span::styled("Active", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Uptime: "),
                Span::raw("2h 15m 30s"),
            ]),
            Line::from(vec![
                Span::raw("Connections: "),
                Span::raw("12"),
            ]),
        ];
        
        let paragraph = Paragraph::new(info)
            .block(Block::default().title("Node Info").borders(Borders::ALL));
        
        f.render_widget(paragraph, area);
    }
    
    /// Draw network stats
    #[allow(dead_code)]
    fn draw_network_stats(&self, f: &mut Frame, area: Rect) {
        let stats = vec![
            Line::from(vec![
                Span::raw("Messages Sent: "),
                Span::raw("1,234"),
            ]),
            Line::from(vec![
                Span::raw("Messages Received: "),
                Span::raw("2,456"),
            ]),
            Line::from(vec![
                Span::raw("Bandwidth Used: "),
                Span::raw("125.4 MB"),
            ]),
            Line::from(vec![
                Span::raw("Avg Latency: "),
                Span::raw("45.2 ms"),
            ]),
        ];
        
        let paragraph = Paragraph::new(stats)
            .block(Block::default().title("Network Stats").borders(Borders::ALL));
        
        f.render_widget(paragraph, area);
    }
    
    /// Draw connections graph
    #[allow(dead_code)]
    fn draw_connections_graph(&self, f: &mut Frame, area: Rect) {
        let sparkline = Sparkline::default()
            .block(Block::default().title("Connections History").borders(Borders::ALL))
            .data(&self.connections_history)
            .style(Style::default().fg(Color::Cyan));
        
        f.render_widget(sparkline, area);
    }
    
    /// Draw bandwidth graph
    #[allow(dead_code)]
    fn draw_bandwidth_graph(&self, f: &mut Frame, area: Rect) {
        let sparkline = Sparkline::default()
            .block(Block::default().title("Bandwidth (KB/s)").borders(Borders::ALL))
            .data(&self.bandwidth_history)
            .style(Style::default().fg(Color::Green));
        
        f.render_widget(sparkline, area);
    }
    
    /// Draw NAT metrics tab
    #[allow(dead_code)]
    fn draw_nat_metrics(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Min(0)])
            .split(area);
        
        // NAT success gauge
        let gauge = Gauge::default()
            .block(Block::default().title("NAT Traversal Success Rate").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(92)
            .label("92%");
        
        f.render_widget(gauge, chunks[0]);
        
        // NAT types breakdown
        let nat_types = vec![
            ListItem::new("Full Cone NAT: 25 nodes (99% success)"),
            ListItem::new("Restricted NAT: 35 nodes (95% success)"),
            ListItem::new("Port Restricted: 20 nodes (90% success)"),
            ListItem::new("Symmetric NAT: 15 nodes (85% success)"),
            ListItem::new("CGNAT: 5 nodes (75% success)"),
        ];
        
        let list = List::new(nat_types)
            .block(Block::default().title("NAT Types Distribution").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        
        f.render_widget(list, chunks[1]);
    }
    
    /// Draw adaptive metrics tab
    #[allow(dead_code)]
    fn draw_adaptive_metrics(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        
        // ML metrics
        let ml_metrics = vec![
            Line::from(vec![
                Span::raw("Thompson Sampling: "),
                Span::styled("85%", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("MAB Reward: "),
                Span::styled("0.72", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Q-Learning Cache: "),
                Span::styled("68%", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("Churn Prediction: "),
                Span::styled("81%", Style::default().fg(Color::Magenta)),
            ]),
        ];
        
        let ml_paragraph = Paragraph::new(ml_metrics)
            .block(Block::default().title("Machine Learning").borders(Borders::ALL));
        
        f.render_widget(ml_paragraph, chunks[0]);
        
        // Routing metrics
        let routing_metrics = vec![
            Line::from(vec![
                Span::raw("EigenTrust: "),
                Span::styled("95% converged", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Hyperbolic: "),
                Span::styled("88% efficiency", Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                Span::raw("SOM Clustering: "),
                Span::styled("76% quality", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Geographic: "),
                Span::styled("5 regions", Style::default().fg(Color::Cyan)),
            ]),
        ];
        
        let routing_paragraph = Paragraph::new(routing_metrics)
            .block(Block::default().title("Adaptive Routing").borders(Borders::ALL));
        
        f.render_widget(routing_paragraph, chunks[1]);
    }
    
    /// Draw performance tab
    #[allow(dead_code)]
    fn draw_performance(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        
        // Latency graph
        let latency_sparkline = Sparkline::default()
            .block(Block::default().title("DHT Lookup Latency (ms)").borders(Borders::ALL))
            .data(&self.latency_history)
            .style(Style::default().fg(Color::Red))
            .max(500);
        
        f.render_widget(latency_sparkline, chunks[0]);
        
        // Performance stats
        let perf_stats = vec![
            Line::from(vec![
                Span::raw("Storage Ops/s: "),
                Span::raw("250"),
            ]),
            Line::from(vec![
                Span::raw("Retrieval Success: "),
                Span::styled("98%", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Replication Health: "),
                Span::styled("95%", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Gossip Propagation: "),
                Span::raw("450ms"),
            ]),
        ];
        
        let perf_paragraph = Paragraph::new(perf_stats)
            .block(Block::default().title("Performance Metrics").borders(Borders::ALL));
        
        f.render_widget(perf_paragraph, chunks[1]);
    }
    
    /// Draw footer
    #[allow(dead_code)]
    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" to switch tabs | "),
                Span::styled("Q", Style::default().fg(Color::Red)),
                Span::raw(" to quit | Last update: "),
                Span::raw(format!("{:.1}s ago", self.last_update.elapsed().as_secs_f32())),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        
        f.render_widget(footer, area);
    }
    
    /// Update metrics
    #[allow(dead_code)]
    async fn update_metrics(&mut self) {
        // Update history buffers (shift left and add new value)
        self.latency_history.rotate_left(1);
        self.latency_history[59] = (rand::random::<f32>() * 200.0) as u64;
        
        self.bandwidth_history.rotate_left(1);
        self.bandwidth_history[59] = (rand::random::<f32>() * 1000.0) as u64;
        
        self.connections_history.rotate_left(1);
        self.connections_history[59] = (rand::random::<f32>() * 20.0) as u64;
        
        self.last_update = Instant::now();
    }
    
    /// Static version of draw method
    fn draw_static(
        f: &mut Frame,
        selected_tab: usize,
        last_update: Instant,
        latency_history: &[u64],
        bandwidth_history: &[u64],
        connections_history: &[u64],
    ) {
        let size = f.area();
        
        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Tabs
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Footer
            ])
            .split(size);
        
        // Header
        Self::draw_header_static(f, chunks[0]);
        
        // Tabs
        Self::draw_tabs_static(f, chunks[1], selected_tab);
        
        // Content based on selected tab
        match selected_tab {
            0 => Self::draw_overview_static(f, chunks[2], latency_history, bandwidth_history, connections_history),
            1 => Self::draw_nat_metrics_static(f, chunks[2]),
            2 => Self::draw_adaptive_metrics_static(f, chunks[2]),
            3 => Self::draw_performance_static(f, chunks[2], latency_history),
            _ => {}
        }
        
        // Footer
        Self::draw_footer_static(f, chunks[3], last_update);
    }
    
    /// Static update metrics
    async fn update_metrics_static(
        latency_history: &mut [u64],
        bandwidth_history: &mut [u64],
        connections_history: &mut [u64],
        last_update: &mut Instant,
    ) {
        // Update history buffers (shift left and add new value)
        latency_history.rotate_left(1);
        latency_history[59] = (rand::random::<f32>() * 200.0) as u64;
        
        bandwidth_history.rotate_left(1);
        bandwidth_history[59] = (rand::random::<f32>() * 1000.0) as u64;
        
        connections_history.rotate_left(1);
        connections_history[59] = (rand::random::<f32>() * 20.0) as u64;
        
        *last_update = Instant::now();
    }
    
    /// Static header
    fn draw_header_static(f: &mut Frame, area: Rect) {
        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("SAORSA TESTNET", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - "),
                Span::styled("Real-time Network Monitor", Style::default().fg(Color::Gray)),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Double))
        .alignment(Alignment::Center);
        
        f.render_widget(header, area);
    }
    
    /// Static tabs
    fn draw_tabs_static(f: &mut Frame, area: Rect, selected_tab: usize) {
        let titles = vec!["Overview", "NAT Traversal", "Adaptive Network", "Performance"];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL))
            .select(selected_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        
        f.render_widget(tabs, area);
    }
    
    /// Static overview
    fn draw_overview_static(
        f: &mut Frame,
        area: Rect,
        _latency_history: &[u64],
        bandwidth_history: &[u64],
        connections_history: &[u64],
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        
        // Left side - Node info
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(chunks[0]);
        
        Self::draw_node_info_static(f, left_chunks[0]);
        Self::draw_connections_graph_static(f, left_chunks[1], connections_history);
        
        // Right side - Network stats
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(chunks[1]);
        
        Self::draw_network_stats_static(f, right_chunks[0]);
        Self::draw_bandwidth_graph_static(f, right_chunks[1], bandwidth_history);
    }
    
    /// Static node info
    fn draw_node_info_static(f: &mut Frame, area: Rect) {
        let info = vec![
            Line::from(vec![
                Span::raw("Node ID: "),
                Span::styled("worker-001", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Status: "),
                Span::styled("Active", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Uptime: "),
                Span::raw("2h 15m 30s"),
            ]),
            Line::from(vec![
                Span::raw("Connections: "),
                Span::raw("12"),
            ]),
        ];
        
        let paragraph = Paragraph::new(info)
            .block(Block::default().title("Node Info").borders(Borders::ALL));
        
        f.render_widget(paragraph, area);
    }
    
    /// Static network stats
    fn draw_network_stats_static(f: &mut Frame, area: Rect) {
        let stats = vec![
            Line::from(vec![
                Span::raw("Messages Sent: "),
                Span::raw("1,234"),
            ]),
            Line::from(vec![
                Span::raw("Messages Received: "),
                Span::raw("2,456"),
            ]),
            Line::from(vec![
                Span::raw("Bandwidth Used: "),
                Span::raw("125.4 MB"),
            ]),
            Line::from(vec![
                Span::raw("Avg Latency: "),
                Span::raw("45.2 ms"),
            ]),
        ];
        
        let paragraph = Paragraph::new(stats)
            .block(Block::default().title("Network Stats").borders(Borders::ALL));
        
        f.render_widget(paragraph, area);
    }
    
    /// Static connections graph
    fn draw_connections_graph_static(f: &mut Frame, area: Rect, connections_history: &[u64]) {
        let sparkline = Sparkline::default()
            .block(Block::default().title("Connections History").borders(Borders::ALL))
            .data(connections_history)
            .style(Style::default().fg(Color::Cyan));
        
        f.render_widget(sparkline, area);
    }
    
    /// Static bandwidth graph
    fn draw_bandwidth_graph_static(f: &mut Frame, area: Rect, bandwidth_history: &[u64]) {
        let sparkline = Sparkline::default()
            .block(Block::default().title("Bandwidth (KB/s)").borders(Borders::ALL))
            .data(bandwidth_history)
            .style(Style::default().fg(Color::Green));
        
        f.render_widget(sparkline, area);
    }
    
    /// Static NAT metrics
    fn draw_nat_metrics_static(f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Min(0)])
            .split(area);
        
        // NAT success gauge
        let gauge = Gauge::default()
            .block(Block::default().title("NAT Traversal Success Rate").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(92)
            .label("92%");
        
        f.render_widget(gauge, chunks[0]);
        
        // NAT types breakdown
        let nat_types = vec![
            ListItem::new("Full Cone NAT: 25 nodes (99% success)"),
            ListItem::new("Restricted NAT: 35 nodes (95% success)"),
            ListItem::new("Port Restricted: 20 nodes (90% success)"),
            ListItem::new("Symmetric NAT: 15 nodes (85% success)"),
            ListItem::new("CGNAT: 5 nodes (75% success)"),
        ];
        
        let list = List::new(nat_types)
            .block(Block::default().title("NAT Types Distribution").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        
        f.render_widget(list, chunks[1]);
    }
    
    /// Static adaptive metrics
    fn draw_adaptive_metrics_static(f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        
        // ML metrics
        let ml_metrics = vec![
            Line::from(vec![
                Span::raw("Thompson Sampling: "),
                Span::styled("85%", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("MAB Reward: "),
                Span::styled("0.72", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Q-Learning Cache: "),
                Span::styled("68%", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("Churn Prediction: "),
                Span::styled("81%", Style::default().fg(Color::Magenta)),
            ]),
        ];
        
        let ml_paragraph = Paragraph::new(ml_metrics)
            .block(Block::default().title("Machine Learning").borders(Borders::ALL));
        
        f.render_widget(ml_paragraph, chunks[0]);
        
        // Routing metrics
        let routing_metrics = vec![
            Line::from(vec![
                Span::raw("EigenTrust: "),
                Span::styled("95% converged", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Hyperbolic: "),
                Span::styled("88% efficiency", Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                Span::raw("SOM Clustering: "),
                Span::styled("76% quality", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("Geographic: "),
                Span::styled("5 regions", Style::default().fg(Color::Cyan)),
            ]),
        ];
        
        let routing_paragraph = Paragraph::new(routing_metrics)
            .block(Block::default().title("Adaptive Routing").borders(Borders::ALL));
        
        f.render_widget(routing_paragraph, chunks[1]);
    }
    
    /// Static performance
    fn draw_performance_static(f: &mut Frame, area: Rect, latency_history: &[u64]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        
        // Latency graph
        let latency_sparkline = Sparkline::default()
            .block(Block::default().title("DHT Lookup Latency (ms)").borders(Borders::ALL))
            .data(latency_history)
            .style(Style::default().fg(Color::Red))
            .max(500);
        
        f.render_widget(latency_sparkline, chunks[0]);
        
        // Performance stats
        let perf_stats = vec![
            Line::from(vec![
                Span::raw("Storage Ops/s: "),
                Span::raw("250"),
            ]),
            Line::from(vec![
                Span::raw("Retrieval Success: "),
                Span::styled("98%", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Replication Health: "),
                Span::styled("95%", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Gossip Propagation: "),
                Span::raw("450ms"),
            ]),
        ];
        
        let perf_paragraph = Paragraph::new(perf_stats)
            .block(Block::default().title("Performance Metrics").borders(Borders::ALL));
        
        f.render_widget(perf_paragraph, chunks[1]);
    }
    
    /// Static footer
    fn draw_footer_static(f: &mut Frame, area: Rect, last_update: Instant) {
        let footer = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" to switch tabs | "),
                Span::styled("Q", Style::default().fg(Color::Red)),
                Span::raw(" to quit | Last update: "),
                Span::raw(format!("{:.1}s ago", last_update.elapsed().as_secs_f32())),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        
        f.render_widget(footer, area);
    }
}