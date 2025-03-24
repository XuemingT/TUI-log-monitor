use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::time::{Duration, Instant};
use std::env;
use std::collections::HashMap;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Gauge},
    Frame, Terminal,
};

// Enum for application views
#[derive(PartialEq)]
enum ViewMode {
    LogView,
    StatsView,
    HelpView,
    FilterView,
}

// App state
struct App {
    log_path: String,
    log_lines: Vec<LogLine>,
    filtered_logs: Vec<usize>, // Indices of logs that match current filter
    scroll: usize,
    selected_tab: usize,
    follow_mode: bool,
    last_update: Instant,
    view_mode: ViewMode,
    stats: LogStats,
    filter_text: String,
    filter_editing: bool,
    show_timestamps: bool,
    show_line_numbers: bool,
    max_lines: usize,
}

// Statistics about logs
struct LogStats {
    total_entries: usize,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    debug_count: usize,
    unknown_count: usize,
    entries_by_hour: HashMap<String, usize>,
}

// Represents a line in the log with level-based coloring
struct LogLine {
    content: String,
    timestamp: String,
    level: LogLevel,
    highlighted: bool,
}

// Log levels for coloring
#[derive(PartialEq, Eq, Clone, Copy)]
enum LogLevel {
    Info,
    Debug,
    Warning,
    Error,
    Unknown,
}

impl LogLevel {
    fn from_line(line: &str) -> Self {
        // Special case for common macOS log formats
        if line.contains("ASL Sender Statistics") {
            return LogLevel::Info;
        }
        
        let line_lower = line.to_lowercase();
        if line_lower.contains("error") || line_lower.contains("fail") || line_lower.contains("exception") {
            LogLevel::Error
        } else if line_lower.contains("warn") {
            LogLevel::Warning
        } else if line_lower.contains("debug") {
            LogLevel::Debug
        } else if line_lower.contains("notice") || line_lower.contains("info") {
            LogLevel::Info
        } else {
            LogLevel::Unknown
        }
    }

    fn color(&self) -> Color {
        match self {
            LogLevel::Info => Color::Green,
            LogLevel::Debug => Color::Cyan,
            LogLevel::Warning => Color::Yellow,
            LogLevel::Error => Color::Red,
            LogLevel::Unknown => Color::Gray,
        }
    }
    
    fn as_str(&self) -> &str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Unknown => "UNKNOWN",
        }
    }
}

impl App {
    fn new(log_path: String) -> Self {
        App {
            log_path,
            log_lines: Vec::new(),
            filtered_logs: Vec::new(),
            scroll: 0,
            selected_tab: 0,
            follow_mode: true,
            last_update: Instant::now(),
            view_mode: ViewMode::LogView,
            stats: LogStats {
                total_entries: 0,
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                debug_count: 0,
                unknown_count: 0,
                entries_by_hour: HashMap::new(),
            },
            filter_text: String::new(),
            filter_editing: false,
            show_timestamps: true,
            show_line_numbers: true,
            max_lines: 1000, // Store at most 1000 log lines to prevent memory issues
        }
    }

    // Read the last N lines from the log file
    fn initialize_logs(&mut self, num_lines: usize) -> io::Result<()> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        
        // Simple approach: read all lines into memory and take the last num_lines
        let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();
        let start_idx = if lines.len() > num_lines {
            lines.len() - num_lines
        } else {
            0
        };
        
        for line in &lines[start_idx..] {
            self.add_log_line(line);
        }
        
        self.update_filter();
        self.update_stats();
        
        Ok(())
    }

    fn add_log_line(&mut self, line: &str) {
        // Extract timestamp if possible (basic implementation)
        let timestamp = if line.len() > 15 && line.chars().nth(10) == Some(' ') && line.chars().nth(13) == Some(':') {
            line[0..19].to_string()
        } else {
            "".to_string()
        };
        
        let level = LogLevel::from_line(line);
        
        // Add to log lines
        self.log_lines.push(LogLine {
            content: line.to_string(),
            timestamp,
            level,
            highlighted: false,
        });
        
        // Remove oldest lines if we exceed our limit
        if self.log_lines.len() > self.max_lines {
            self.log_lines.remove(0);
        }
    }

    // Check for new lines in the log file
    fn update_logs(&mut self) -> io::Result<()> {
        if self.last_update.elapsed() < Duration::from_millis(500) {
            // Don't update too frequently
            return Ok(());
        }
        
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        
        // Simple approach for now - read all lines and compare with what we have
        let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();
        
        if lines.len() > self.log_lines.len() {
            // There are new lines
            for i in self.log_lines.len()..lines.len() {
                self.add_log_line(&lines[i]);
            }
            
            // Update stats and filter
            self.update_stats();
            self.update_filter();
            
            // Auto-scroll if follow mode is enabled
            if self.follow_mode {
                self.scroll = self.filtered_logs.len();
            }
        }
        
        self.last_update = Instant::now();
        Ok(())
    }

    fn update_filter(&mut self) {
        if self.filter_text.is_empty() {
            // No filter - show all logs
            self.filtered_logs = (0..self.log_lines.len()).collect();
        } else {
            // Apply filter
            let filter_lower = self.filter_text.to_lowercase();
            self.filtered_logs = self.log_lines.iter()
                .enumerate()
                .filter(|(_, log)| log.content.to_lowercase().contains(&filter_lower))
                .map(|(i, _)| i)
                .collect();
        }
    }

    fn update_stats(&mut self) {
        self.stats.total_entries = self.log_lines.len();
        self.stats.error_count = self.log_lines.iter().filter(|l| l.level == LogLevel::Error).count();
        self.stats.warning_count = self.log_lines.iter().filter(|l| l.level == LogLevel::Warning).count();
        self.stats.info_count = self.log_lines.iter().filter(|l| l.level == LogLevel::Info).count();
        self.stats.debug_count = self.log_lines.iter().filter(|l| l.level == LogLevel::Debug).count();
        self.stats.unknown_count = self.log_lines.iter().filter(|l| l.level == LogLevel::Unknown).count();
        
        // Group by hour for chart
        self.stats.entries_by_hour.clear();
        for log in &self.log_lines {
            if log.timestamp.len() >= 13 {
                let hour = &log.timestamp[11..13];
                *self.stats.entries_by_hour.entry(hour.to_string()).or_insert(0) += 1;
            }
        }
    }

    fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    fn scroll_down(&mut self) {
        if self.scroll < self.filtered_logs.len() {
            self.scroll += 1;
        }
    }

    fn page_up(&mut self) {
        if self.scroll > 10 {
            self.scroll -= 10;
        } else {
            self.scroll = 0;
        }
    }

    fn page_down(&mut self) {
        if self.scroll + 10 < self.filtered_logs.len() {
            self.scroll += 10;
        } else {
            self.scroll = self.filtered_logs.len();
        }
    }

    fn toggle_follow_mode(&mut self) {
        self.follow_mode = !self.follow_mode;
        if self.follow_mode {
            // Jump to the bottom when enabling follow mode
            self.scroll = self.filtered_logs.len();
        }
    }

    fn toggle_timestamps(&mut self) {
        self.show_timestamps = !self.show_timestamps;
    }

    fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    fn add_filter_char(&mut self, c: char) {
        self.filter_text.push(c);
        self.update_filter();
    }

    fn remove_filter_char(&mut self) {
        self.filter_text.pop();
        self.update_filter();
    }

    fn clear_filter(&mut self) {
        self.filter_text.clear();
        self.update_filter();
    }

    fn toggle_filter_mode(&mut self) {
        self.filter_editing = !self.filter_editing;
        if !self.filter_editing {
            // Apply filter when exiting filter mode
            self.update_filter();
        }
    }

    fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % 3;
        match self.selected_tab {
            0 => self.view_mode = ViewMode::LogView,
            1 => self.view_mode = ViewMode::StatsView,
            2 => self.view_mode = ViewMode::HelpView,
            _ => {}
        }
    }

    fn prev_tab(&mut self) {
        if self.selected_tab > 0 {
            self.selected_tab -= 1;
        } else {
            self.selected_tab = 2;
        }
        match self.selected_tab {
            0 => self.view_mode = ViewMode::LogView,
            1 => self.view_mode = ViewMode::StatsView,
            2 => self.view_mode = ViewMode::HelpView,
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Get log path from command line argument or use default
    let args: Vec<String> = env::args().collect();
    let log_path = if args.len() > 1 {
        args[1].clone()
    } else {
        "/var/log/system.log".to_string() // Default log file
    };

    // Create app state
    let mut app = App::new(log_path);
    app.initialize_logs(100)?; // Read the last 100 lines

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| {
            let size = f.size();
            
            // Top-level layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3), // Tab row + filter
                    Constraint::Min(1),    // Content area
                    Constraint::Length(1), // Help text
                ])
                .split(size);
            
            // Render tabs
            let titles = vec!["Logs", "Statistics", "Help"];
            let tabs = Tabs::new(titles.iter().map(|t| Line::from(*t)).collect())
                .block(Block::default().borders(Borders::BOTTOM))
                .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .select(app.selected_tab);
            f.render_widget(tabs, chunks[0]);
            
            // Render the appropriate content based on view mode
            match app.view_mode {
                ViewMode::LogView => draw_log_view(&mut app, f, chunks[1]),
                ViewMode::StatsView => draw_stats_view(&app, f, chunks[1]),
                ViewMode::HelpView => draw_help_view(f, chunks[1]),
                ViewMode::FilterView => {
                    // When in filter mode, still show logs but focus on filter input
                    draw_log_view(&mut app, f, chunks[1]);
                }
            }
            
            // Status bar at bottom
            let status_text = match app.view_mode {
                ViewMode::FilterView => format!("Filter: {} (Press Enter to apply, Esc to cancel)", app.filter_text),
                _ => {
                    let filter_status = if !app.filter_text.is_empty() {
                        format!(" | Filter: {}", app.filter_text)
                    } else {
                        String::new()
                    };
                    
                    format!(
                        "{}Follow: {} | Lines: {}/{}{}", 
                        if app.view_mode == ViewMode::LogView { "" } else { "Log File: {} | " },
                        if app.follow_mode { "ON" } else { "OFF" },
                        app.filtered_logs.len(),
                        app.log_lines.len(),
                        filter_status
                    )
                }
            };
            
            let help_text = match app.view_mode {
                ViewMode::FilterView => "Enter: Apply Filter | Esc: Cancel",
                ViewMode::LogView => "↑/↓: Scroll | PgUp/PgDn: Page | F: Follow | /: Filter | T: Timestamps | N: Line# | Tab: Switch View",
                ViewMode::StatsView => "Tab: Switch View | R: Refresh Stats",
                ViewMode::HelpView => "Tab: Switch View | Q: Quit",
            };
            
            let status_bar = Paragraph::new(status_text)
                .style(Style::default().fg(Color::White));
            f.render_widget(status_bar, chunks[2]);
            
            // Show help text at bottom right
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(ratatui::layout::Alignment::Right);
            f.render_widget(help, chunks[2]);
            
            // Special case for filter input mode
            if app.view_mode == ViewMode::FilterView {
                // Create a popup for filter input
                let area = centered_rect(60, 3, size);
                let filter_input = Paragraph::new(format!("Filter: {}", app.filter_text))
                    .style(Style::default().fg(Color::White))
                    .block(Block::default().borders(Borders::ALL).title("Enter Filter Pattern"));
                f.render_widget(filter_input, area);
            }
        })?;

        // Check for new log entries (except in help view)
        if app.view_mode != ViewMode::HelpView {
            app.update_logs()?;
        }

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.view_mode {
                    ViewMode::FilterView => {
                        match key.code {
                            KeyCode::Enter => {
                                app.view_mode = ViewMode::LogView;
                                app.filter_editing = false;
                                app.update_filter();
                            },
                            KeyCode::Esc => {
                                app.view_mode = ViewMode::LogView;
                                app.filter_editing = false;
                                // Restore previous filter if canceled
                            },
                            KeyCode::Char(c) => {
                                app.add_filter_char(c);
                            },
                            KeyCode::Backspace => {
                                app.remove_filter_char();
                            },
                            _ => {}
                        }
                    },
                    _ => {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('f') => app.toggle_follow_mode(),
                            KeyCode::Char('t') => app.toggle_timestamps(),
                            KeyCode::Char('n') => app.toggle_line_numbers(),
                            KeyCode::Char('/') => {
                                app.view_mode = ViewMode::FilterView;
                                app.filter_editing = true;
                            },
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.clear_filter();
                            },
                            KeyCode::Tab => app.next_tab(),
                            KeyCode::BackTab => app.prev_tab(),
                            KeyCode::Up => {
                                app.follow_mode = false; // Disable follow mode when scrolling
                                app.scroll_up();
                            },
                            KeyCode::Down => app.scroll_down(),
                            KeyCode::PageUp => {
                                app.follow_mode = false;
                                app.page_up();
                            },
                            KeyCode::PageDown => app.page_down(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_log_view<B: ratatui::backend::Backend>(app: &mut App, f: &mut Frame<B>, area: Rect) {
    // Split into filter area and logs area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Filter display
            Constraint::Min(1),    // Logs
        ])
        .split(area);
    
    // Show current filter if any
    let filter_text = if app.filter_text.is_empty() {
        "No filter applied"
    } else {
        &app.filter_text
    };
    
    let filter_display = Paragraph::new(format!("Filter: {}", filter_text))
        .style(Style::default().fg(
            if app.filter_text.is_empty() { Color::DarkGray } else { Color::Yellow }
        ));
    f.render_widget(filter_display, chunks[0]);
    
    // Prepare the log items for display
    let visible_logs: Vec<ListItem> = app.filtered_logs
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let log = &app.log_lines[idx];
            
            // Format the log line
            let mut parts = Vec::new();
            
            // Add line number if enabled
            if app.show_line_numbers {
                parts.push(Span::styled(
                    format!("{:<4} ", i + 1),
                    Style::default().fg(Color::DarkGray)
                ));
            }
            
            // Add timestamp if enabled
            if app.show_timestamps && !log.timestamp.is_empty() {
                parts.push(Span::styled(
                    format!("{} ", log.timestamp),
                    Style::default().fg(Color::DarkGray)
                ));
            }
            
            // Add log level indicator
            parts.push(Span::styled(
                format!("[{}] ", log.level.as_str()),
                Style::default().fg(log.level.color()).add_modifier(Modifier::BOLD)
            ));
            
            // Add the main content
            parts.push(Span::styled(
                log.content.clone(),
                Style::default().fg(log.level.color())
            ));
            
            ListItem::new(Line::from(parts))
        })
        .collect();
    
    // Determine visible range for scrolling
    let start_idx = if app.scroll > 10 { app.scroll - 10 } else { 0 };
    let logs_height = chunks[1].height as usize;
    let end_idx = std::cmp::min(start_idx + logs_height, visible_logs.len());
    
    let visible_items: Vec<ListItem> = visible_logs
        .into_iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .collect();
    
    // Render the logs list
    let logs_list = List::new(visible_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!("Logs ({}/{})", app.filtered_logs.len(), app.log_lines.len())))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    
    f.render_widget(logs_list, chunks[1]);
}

fn draw_stats_view<B: ratatui::backend::Backend>(app: &App, f: &mut Frame<B>, area: Rect) {
    // Split the area into different sections for statistics
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Summary
            Constraint::Length(6), // Log level distribution
            Constraint::Min(1),    // Hourly chart
        ])
        .split(area);
    
    // Summary statistics
    let summary = Paragraph::new(format!(
        "Total Log Entries: {} | Errors: {} | Warnings: {} | Info: {} | Debug: {}",
        app.stats.total_entries,
        app.stats.error_count,
        app.stats.warning_count,
        app.stats.info_count,
        app.stats.debug_count
    ))
    .block(Block::default().borders(Borders::ALL).title("Summary"));
    f.render_widget(summary, chunks[0]);
    
    // Log level distribution
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);
    
    let total = app.stats.total_entries as f64;
    
    if total > 0.0 {
        // Error gauge
        let error_pct = (app.stats.error_count as f64 / total) * 100.0;
        let error_gauge = Gauge::default()
            .block(Block::default().title("Errors").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Red))
            .percent(error_pct as u16)
            .label(format!("{:.1}%", error_pct));
        f.render_widget(error_gauge, horizontal_chunks[0]);
        
        // Warning gauge
        let warning_pct = (app.stats.warning_count as f64 / total) * 100.0;
        let warning_gauge = Gauge::default()
            .block(Block::default().title("Warnings").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Yellow))
            .percent(warning_pct as u16)
            .label(format!("{:.1}%", warning_pct));
        f.render_widget(warning_gauge, horizontal_chunks[1]);
        
        // Info gauge
        let info_pct = (app.stats.info_count as f64 / total) * 100.0;
        let info_gauge = Gauge::default()
            .block(Block::default().title("Info").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(info_pct as u16)
            .label(format!("{:.1}%", info_pct));
        f.render_widget(info_gauge, horizontal_chunks[2]);
        
        // Debug gauge
        let debug_pct = (app.stats.debug_count as f64 / total) * 100.0;
        let debug_gauge = Gauge::default()
            .block(Block::default().title("Debug").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(debug_pct as u16)
            .label(format!("{:.1}%", debug_pct));
        f.render_widget(debug_gauge, horizontal_chunks[3]);
        
        // Unknown gauge
        let unknown_pct = (app.stats.unknown_count as f64 / total) * 100.0;
        let unknown_gauge = Gauge::default()
            .block(Block::default().title("Unknown").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Gray))
            .percent(unknown_pct as u16)
            .label(format!("{:.1}%", unknown_pct));
        f.render_widget(unknown_gauge, horizontal_chunks[4]);
    }
    
    // Message distribution by hour
    let hour_distribution = Block::default()
        .title("Messages by Hour")
        .borders(Borders::ALL);
    f.render_widget(hour_distribution, chunks[2]);
    
    // Sort entries by hour and show a simple text representation
    let mut entries: Vec<(String, usize)> = app.stats.entries_by_hour
        .iter()
        .map(|(hour, count)| (hour.clone(), *count))
        .collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    
    if !entries.is_empty() {
        let hour_text = entries
            .iter()
            .map(|(hour, count)| format!("Hour {}: {} messages", hour, count))
            .collect::<Vec<String>>()
            .join(" | ");
        
        let hour_display = Paragraph::new(hour_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::NONE));
        
            let inner_area = chunks[2].inner(&ratatui::layout::Margin { 
                vertical: 1, 
                horizontal: 2,
            });
        
        f.render_widget(hour_display, inner_area);
    }
}

fn draw_help_view<B: ratatui::backend::Backend>(f: &mut Frame<B>, area: Rect) {
    let text = vec![
        Line::from(vec![Span::styled("Log Monitor - Keyboard Shortcuts", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![
            Span::styled("General", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
        ]),
        Line::from("Tab: Switch between views (Logs, Statistics, Help)"),
        Line::from("Q: Quit the application"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Log View", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
        ]),
        Line::from("↑/↓: Scroll up/down"),
        Line::from("PgUp/PgDn: Page up/down"),
        Line::from("F: Toggle follow mode (auto-scroll to new logs)"),
        Line::from("T: Toggle timestamps display"),
        Line::from("N: Toggle line numbers"),
        Line::from("/: Enter filter mode"),
        Line::from("Ctrl+C: Clear current filter"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Filter Mode", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
        ]),
        Line::from("Enter: Apply filter"),
        Line::from("Esc: Cancel and exit filter mode"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Statistics View", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
        ]),
        Line::from("R: Refresh statistics"),
    ];

    let help_text = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .alignment(ratatui::layout::Alignment::Left);
    
    f.render_widget(help_text, area);
}

/// Helper function to create a centered rect using a percentage of the available rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

