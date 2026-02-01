//! TUI module using ratatui.
//!
//! Component-based pattern for high responsiveness.

use crate::{agent, scraper, Config, Storage, StoredSummary, Summary};
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
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

// Colour scheme (myon/ilseon inspired)
const BG_DEEP: Color = Color::Rgb(54, 52, 58);
const FG_PRIMARY: Color = Color::Rgb(224, 224, 224);
const FG_MUTED: Color = Color::Rgb(176, 176, 176);
const BORDER_ACTIVE: Color = Color::Rgb(90, 155, 128);
const BORDER_QUIET: Color = Color::Rgb(31, 31, 31);
const ACCENT_URGENT: Color = Color::Rgb(179, 95, 95);

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

/// Which pane is currently focused
#[derive(Debug, Clone, PartialEq)]
enum FocusedPane {
    List,
    Detail,
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
    /// List of stored summaries
    stored_summaries: Vec<StoredSummary>,
    /// List selection state
    list_state: ListState,
    /// Which pane is focused
    focused_pane: FocusedPane,
    /// Scroll offset for detail view
    detail_scroll: u16,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::Main,
            url_input: String::new(),
            summary: None,
            source_url: None,
            should_quit: false,
            status: "Press 'o' to open URL, ↑↓ to navigate, Tab to switch panes, 'q' to quit"
                .to_string(),
            stored_summaries: Vec::new(),
            list_state: ListState::default(),
            focused_pane: FocusedPane::List,
            detail_scroll: 0,
        }
    }
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Load stored summaries from storage
    fn load_summaries(&mut self) {
        if let Ok(config) = Config::load() {
            if let Ok(storage) = Storage::open(&config.storage.path) {
                if let Ok(summaries) = storage.list_all() {
                    self.stored_summaries = summaries;
                    // Select first item if available
                    if !self.stored_summaries.is_empty() {
                        self.list_state.select(Some(0));
                        self.update_selected_summary();
                    }
                }
            }
        }
    }

    /// Update the displayed summary based on selection
    fn update_selected_summary(&mut self) {
        if let Some(index) = self.list_state.selected() {
            if let Some(stored) = self.stored_summaries.get(index) {
                self.summary = Some(stored.summary.clone());
                self.source_url = Some(stored.url.clone());
                self.detail_scroll = 0; // Reset scroll when selecting new summary
            }
        }
    }

    /// Select the previous item in the list
    fn select_previous(&mut self) {
        if self.stored_summaries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.stored_summaries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.update_selected_summary();
    }

    /// Select the next item in the list
    fn select_next(&mut self) {
        if self.stored_summaries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.stored_summaries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.update_selected_summary();
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
                KeyCode::Tab => {
                    self.focused_pane = match self.focused_pane {
                        FocusedPane::List => FocusedPane::Detail,
                        FocusedPane::Detail => FocusedPane::List,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.focused_pane == FocusedPane::List {
                        self.select_previous();
                    } else {
                        // Scroll detail view up
                        self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.focused_pane == FocusedPane::List {
                        self.select_next();
                    } else {
                        // Scroll detail view down
                        self.detail_scroll = self.detail_scroll.saturating_add(1);
                    }
                }
                KeyCode::PageUp => {
                    if self.focused_pane == FocusedPane::Detail {
                        self.detail_scroll = self.detail_scroll.saturating_sub(10);
                    }
                }
                KeyCode::PageDown => {
                    if self.focused_pane == FocusedPane::Detail {
                        self.detail_scroll = self.detail_scroll.saturating_add(10);
                    }
                }
                KeyCode::Home => {
                    if self.focused_pane == FocusedPane::Detail {
                        self.detail_scroll = 0;
                    }
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
                            // Persist the summary
                            if let Err(e) = self.save_summary(&url, &summary, &config) {
                                // Log but don't fail - storage is optional
                                eprintln!("Warning: Failed to save summary: {}", e);
                            }

                            self.summary = Some(summary);
                            self.source_url = Some(url);
                            self.state = AppState::Main;
                            self.status = "Press 'o' to open URL, ↑↓ to navigate, Tab to switch panes, 'q' to quit".to_string();

                            // Reload summaries list to include the new one
                            self.load_summaries();
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

    /// Save a summary to persistent storage
    fn save_summary(&self, url: &str, summary: &Summary, config: &Config) -> anyhow::Result<()> {
        let storage = Storage::open(&config.storage.path)?;
        storage.store(url, summary)?;
        Ok(())
    }
}

/// Draw the UI
fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    // Split main area into list (left) and detail (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    // Draw summary list on the left
    draw_summary_list(frame, app, main_chunks[0]);

    // Draw detail view on the right
    draw_detail_view(frame, app, main_chunks[1]);

    // Status bar
    let status =
        Paragraph::new(app.status.clone()).style(Style::default().fg(FG_MUTED).bg(BORDER_QUIET));
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

/// Draw the summary list on the left
fn draw_summary_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::List;
    let border_color = if is_focused {
        BORDER_ACTIVE
    } else {
        BORDER_QUIET
    };

    let block = Block::default()
        .title(" Summaries ")
        .borders(Borders::ALL)
        .style(Style::default().fg(border_color).bg(BG_DEEP));

    if app.stored_summaries.is_empty() {
        let empty_msg = Paragraph::new("No summaries yet.\nPress 'o' to add one.")
            .block(block)
            .style(Style::default().fg(FG_MUTED));
        frame.render_widget(empty_msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .stored_summaries
        .iter()
        .map(|stored| {
            let title = &stored.summary.title;
            let date = stored.created_at.format("%m/%d %H:%M").to_string();
            let content = Line::from(vec![
                Span::styled(truncate_string(title, 20), Style::default().fg(FG_PRIMARY)),
                Span::styled(format!(" ({})", date), Style::default().fg(FG_MUTED)),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(BG_DEEP)
                .bg(BORDER_ACTIVE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

/// Truncate a string to a maximum length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max_len - 1).collect::<String>())
    }
}

/// Draw the detail view on the right
fn draw_detail_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Detail;
    let border_color = if is_focused {
        BORDER_ACTIVE
    } else {
        BORDER_QUIET
    };

    let title = if is_focused {
        " Summary Detail (↑↓ scroll) "
    } else {
        " Summary Detail "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(border_color).bg(BG_DEEP));

    if let Some(ref summary) = app.summary {
        // Display summary
        let mut lines: Vec<Line> = vec![];

        // Title
        lines.push(Line::from(vec![Span::styled(
            &summary.title,
            Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        // Source URL
        if let Some(ref url) = app.source_url {
            lines.push(Line::from(vec![
                Span::styled("Source: ", Style::default().fg(FG_MUTED)),
                Span::styled(url, Style::default().fg(BORDER_ACTIVE)),
            ]));
            lines.push(Line::from(""));
        }

        // Conclusion
        lines.push(Line::from(vec![Span::styled(
            "💡 Conclusion",
            Style::default()
                .fg(BORDER_ACTIVE)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(Span::styled(
            &summary.conclusion,
            Style::default().fg(FG_PRIMARY),
        )));
        lines.push(Line::from(""));

        // Key Points
        lines.push(Line::from(vec![Span::styled(
            "📌 Key Points",
            Style::default()
                .fg(BORDER_ACTIVE)
                .add_modifier(Modifier::BOLD),
        )]));
        for point in &summary.key_points {
            lines.push(Line::from(Span::styled(
                format!("• {}", point),
                Style::default().fg(FG_PRIMARY),
            )));
        }
        lines.push(Line::from(""));

        // Entities
        if !summary.entities.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "🏷️  Entities",
                Style::default()
                    .fg(BORDER_ACTIVE)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(Span::styled(
                summary.entities.join(", "),
                Style::default().fg(FG_MUTED),
            )));
            lines.push(Line::from(""));
        }

        // Action Items
        if !summary.action_items.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "✅ Action Items",
                Style::default()
                    .fg(BORDER_ACTIVE)
                    .add_modifier(Modifier::BOLD),
            )]));
            for item in &summary.action_items {
                lines.push(Line::from(Span::styled(
                    format!("• {}", item),
                    Style::default().fg(FG_PRIMARY),
                )));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0));
        frame.render_widget(paragraph, area);
    } else {
        // Welcome message
        let welcome = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Welcome to Summa!",
                Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(Span::styled(
                "Intelligent webpage summarisation powered by LLMs.",
                Style::default().fg(FG_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  o    ", Style::default().fg(BORDER_ACTIVE)),
                Span::styled("Open a URL to summarise", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled("  ↑↓   ", Style::default().fg(BORDER_ACTIVE)),
                Span::styled("Navigate summaries", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled("  Tab  ", Style::default().fg(BORDER_ACTIVE)),
                Span::styled("Switch panes", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled("  q    ", Style::default().fg(BORDER_ACTIVE)),
                Span::styled("Quit", Style::default().fg(FG_PRIMARY)),
            ]),
        ];
        let paragraph = Paragraph::new(welcome).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// Draw the URL input dialogue
fn draw_url_dialogue(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 30, frame.area());

    // Clear the area behind the dialogue
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Enter URL ")
        .borders(Borders::ALL)
        .style(Style::default().fg(BORDER_ACTIVE).bg(BG_DEEP));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Label
            Constraint::Length(1), // Spacing
            Constraint::Length(3), // Input field
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Help text
        ])
        .split(inner);

    let label = Paragraph::new("URL:").style(Style::default().fg(FG_MUTED));
    frame.render_widget(label, chunks[0]);

    let input = Paragraph::new(format!(" {}", app.url_input))
        .style(Style::default().fg(FG_PRIMARY))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_ACTIVE)),
        );
    frame.render_widget(input, chunks[2]);

    let help =
        Paragraph::new("Press Enter to submit, Esc to cancel").style(Style::default().fg(FG_MUTED));
    frame.render_widget(help, chunks[4]);
}

/// Draw loading indicator
fn draw_loading(frame: &mut Frame) {
    let area = centered_rect(40, 10, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Loading ")
        .borders(Borders::ALL)
        .style(Style::default().fg(BORDER_ACTIVE).bg(BG_DEEP));

    let text = Paragraph::new("Please wait...")
        .block(block)
        .style(Style::default().fg(FG_MUTED));
    frame.render_widget(text, area);
}

/// Draw error dialogue
fn draw_error(frame: &mut Frame, message: &str) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Error ")
        .borders(Borders::ALL)
        .style(Style::default().fg(ACCENT_URGENT).bg(BG_DEEP));

    let text = Paragraph::new(message)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(FG_PRIMARY));
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

    // Load saved summaries
    app.load_summaries();

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| draw(f, &mut app))?;

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
