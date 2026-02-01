//! TUI module using ratatui.
//!
//! Component-based pattern for high responsiveness.

use crate::{agent, scraper, Config, Summary};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

/// Application state
#[derive(Debug, Clone, PartialEq)]
enum AppState {
    /// Main view showing summary or welcome message
    Main,
    /// URL input dialogue
    UrlInput,
    /// Loading content
    Loading,
    /// Error state
    Error(String),
}

/// The main TUI application
pub struct App {
    /// Current application state
    state: AppState,
    /// URL input buffer
    url_input: String,
    /// Current summary being displayed
    summary: Option<Summary>,
    /// Source URL of the current summary
    source_url: Option<String>,
    /// Whether the app should quit
    should_quit: bool,
    /// Status message
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::Main,
            url_input: String::new(),
            summary: None,
            source_url: None,
            should_quit: false,
            status: "Press 'o' to open a URL, 'q' to quit".to_string(),
        }
    }
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyCode) {
        match &self.state {
            AppState::Main => match key {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('o') => {
                    self.state = AppState::UrlInput;
                    self.url_input.clear();
                }
                _ => {}
            },
            AppState::UrlInput => match key {
                KeyCode::Esc => {
                    self.state = AppState::Main;
                    self.url_input.clear();
                }
                KeyCode::Enter => {
                    if !self.url_input.is_empty() {
                        self.state = AppState::Loading;
                    }
                }
                KeyCode::Backspace => {
                    self.url_input.pop();
                }
                KeyCode::Char(c) => {
                    self.url_input.push(c);
                }
                _ => {}
            },
            AppState::Loading => {
                // Can't cancel loading for now
            }
            AppState::Error(_) => match key {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state = AppState::Main;
                }
                KeyCode::Char('q') => self.should_quit = true,
                _ => {}
            },
        }
    }

    /// Fetch and summarise a URL
    async fn fetch_and_summarise(&mut self) {
        let url = self.url_input.clone();
        self.status = format!("Fetching: {}", url);

        // Fetch content
        match scraper::fetch_content(&url).await {
            Ok(content) => {
                self.status = format!("Summarising {} characters...", content.text.len());

                // Load config and summarise
                match Config::load() {
                    Ok(config) => match agent::summarize(&content.text, &config).await {
                        Ok(summary) => {
                            self.summary = Some(summary);
                            self.source_url = Some(url);
                            self.state = AppState::Main;
                            self.status = "Press 'o' to open another URL, 'q' to quit".to_string();
                        }
                        Err(e) => {
                            self.state = AppState::Error(format!("Summarisation failed: {}", e));
                        }
                    },
                    Err(e) => {
                        self.state = AppState::Error(format!("Config error: {}", e));
                    }
                }
            }
            Err(e) => {
                self.state = AppState::Error(format!("Failed to fetch URL: {}", e));
            }
        }
    }
}

/// Draw the UI
fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    // Main content area
    draw_main_content(frame, app, chunks[0]);

    // Status bar
    let status = Paragraph::new(app.status.clone())
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(status, chunks[1]);

    // Draw URL input dialogue if active
    if app.state == AppState::UrlInput {
        draw_url_dialogue(frame, app);
    }

    // Draw loading indicator
    if app.state == AppState::Loading {
        draw_loading(frame);
    }

    // Draw error dialogue
    if let AppState::Error(ref msg) = app.state {
        draw_error(frame, msg);
    }
}

/// Draw the main content area
fn draw_main_content(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Summa - Webpage Summariser ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    if let Some(ref summary) = app.summary {
        // Display summary
        let mut lines: Vec<Line> = vec![];

        // Title
        lines.push(Line::from(vec![Span::styled(
            &summary.title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        // Source URL
        if let Some(ref url) = app.source_url {
            lines.push(Line::from(vec![
                Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
                Span::styled(url, Style::default().fg(Color::Blue)),
            ]));
            lines.push(Line::from(""));
        }

        // Conclusion
        lines.push(Line::from(vec![Span::styled(
            "💡 Conclusion",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(format!("   {}", summary.conclusion)));
        lines.push(Line::from(""));

        // Key Points
        lines.push(Line::from(vec![Span::styled(
            "📌 Key Points",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]));
        for point in &summary.key_points {
            lines.push(Line::from(format!("   • {}", point)));
        }
        lines.push(Line::from(""));

        // Entities
        if !summary.entities.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "🏷️  Entities",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(format!("   {}", summary.entities.join(", "))));
            lines.push(Line::from(""));
        }

        // Action Items
        if !summary.action_items.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "✅ Action Items",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            for item in &summary.action_items {
                lines.push(Line::from(format!("   • {}", item)));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    } else {
        // Welcome message
        let welcome = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Welcome to Summa!",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Intelligent webpage summarisation powered by LLMs."),
            Line::from(""),
            Line::from(vec![
                Span::styled("  o  ", Style::default().fg(Color::Yellow)),
                Span::raw("Open a URL to summarise"),
            ]),
            Line::from(vec![
                Span::styled("  q  ", Style::default().fg(Color::Yellow)),
                Span::raw("Quit"),
            ]),
        ];
        let paragraph = Paragraph::new(welcome).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// Draw the URL input dialogue
fn draw_url_dialogue(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());

    // Clear the area behind the dialogue
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Enter URL ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .margin(1)
        .split(inner);

    let label = Paragraph::new("URL:");
    frame.render_widget(label, chunks[0]);

    let input = Paragraph::new(app.url_input.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(input, chunks[1]);
}

/// Draw loading indicator
fn draw_loading(frame: &mut Frame) {
    let area = centered_rect(40, 10, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Loading ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));

    let text = Paragraph::new("Please wait...")
        .block(block)
        .style(Style::default().fg(Color::White));
    frame.render_widget(text, area);
}

/// Draw error dialogue
fn draw_error(frame: &mut Frame, message: &str) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Error ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Red));

    let text = Paragraph::new(message)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(text, area);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Run the TUI application
pub async fn run() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| draw(f, &app))?;

        // Handle loading state - need to process async
        if app.state == AppState::Loading {
            app.fetch_and_summarise().await;
            continue;
        }

        // Poll for events with a timeout
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }

        if app.should_quit {
            break;
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
