//! CVH Fuzzy - Universal fuzzy finder for CVH Linux
//!
//! A fast, all-in-one fuzzy finder for:
//! - Files and directories
//! - Applications (.desktop files)
//! - Command history
//! - Custom input sources

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nucleo::{Config, Nucleo};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::{
    env,
    fs,
    io::{self, BufRead},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use walkdir::WalkDir;

mod apps;
mod config;
mod matcher;

/// CVH Fuzzy - Universal fuzzy finder
#[derive(Parser, Debug)]
#[command(name = "cvh-fuzzy")]
#[command(author = "CVH Linux Team")]
#[command(version = "0.1.0")]
#[command(about = "Universal fuzzy finder for files, apps, and commands")]
struct Args {
    /// Search mode
    #[arg(short, long, value_enum, default_value = "apps")]
    mode: Mode,

    /// Initial query
    #[arg(short, long, default_value = "")]
    query: String,

    /// Maximum height (lines)
    #[arg(long, default_value = "40")]
    height: u16,

    /// Show border
    #[arg(long, default_value = "true")]
    border: bool,

    /// Print selected item with null terminator
    #[arg(long)]
    print0: bool,

    /// Directory to search (for files/dirs mode)
    #[arg(short = 'p', long)]
    path: Option<PathBuf>,

    /// Read items from stdin
    #[arg(long)]
    stdin: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// Search applications (.desktop files)
    Apps,
    /// Search files
    Files,
    /// Search directories
    Dirs,
    /// Search command history
    History,
    /// Read from stdin
    Stdin,
}

/// An item that can be searched
#[derive(Clone, Debug)]
struct Item {
    /// Display text
    display: String,
    /// Value to return on selection
    value: String,
    /// Optional icon or type indicator
    icon: Option<String>,
}

/// Application state
struct App {
    /// Current query string
    query: String,
    /// All items
    items: Vec<Item>,
    /// Filtered/matched items (indices into items, already sorted by score)
    filtered: Vec<usize>,
    /// Currently selected index in filtered list
    selected: usize,
    /// List state for scrolling
    list_state: ListState,
    /// Nucleo matcher
    matcher: Nucleo<String>,
    /// Should quit
    should_quit: bool,
    /// Selected item (if any)
    selected_item: Option<String>,
}

impl App {
    fn new(items: Vec<Item>) -> Self {
        let config = Config::DEFAULT;
        let matcher = Nucleo::new(config, Arc::new(|| {}), None, 1);

        // Inject items into matcher with their index as data
        let injector = matcher.injector();
        for (idx, item) in items.iter().enumerate() {
            let _ = injector.push(idx.to_string(), |_, cols| {
                cols[0] = item.display.clone().into();
            });
        }

        let mut app = App {
            query: String::new(),
            items,
            filtered: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
            matcher,
            should_quit: false,
            selected_item: None,
        };

        app.update_filter();
        app
    }

    fn update_filter(&mut self) {
        // Update pattern in matcher
        self.matcher.pattern.reparse(
            0,
            &self.query,
            nucleo::pattern::CaseMatching::Smart,
            nucleo::pattern::Normalization::Smart,
            false,
        );

        // Tick the matcher
        let _status = self.matcher.tick(10);

        // Get results - nucleo already returns items sorted by score
        self.filtered.clear();
        let snapshot = self.matcher.snapshot();

        for idx in 0..snapshot.matched_item_count() {
            if let Some(item) = snapshot.get_matched_item(idx) {
                // The data contains the original index as a string
                if let Ok(original_idx) = item.data.parse::<usize>() {
                    self.filtered.push(original_idx);
                } else {
                    // Fallback: use the match index
                    self.filtered.push(idx as usize);
                }
            }
        }

        // Reset selection if out of bounds
        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }

        // Update list state
        self.list_state.select(Some(self.selected));
    }

    fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
            self.list_state.select(Some(self.selected));
        }
    }

    fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.filtered.len() - 1);
            self.list_state.select(Some(self.selected));
        }
    }

    fn confirm_selection(&mut self) {
        if let Some(&idx) = self.filtered.get(self.selected) {
            if let Some(item) = self.items.get(idx) {
                self.selected_item = Some(item.value.clone());
            }
        }
        self.should_quit = true;
    }

    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match (key, modifiers) {
            // Quit without selection
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            // Navigation
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.select_next();
            }
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.select_prev();
            }
            // Confirm selection
            (KeyCode::Enter, _) => {
                self.confirm_selection();
            }
            // Backspace
            (KeyCode::Backspace, _) => {
                self.query.pop();
                self.update_filter();
            }
            // Clear query
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.query.clear();
                self.update_filter();
            }
            // Type character
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.query.push(c);
                self.update_filter();
            }
            _ => {}
        }
    }
}

fn load_items(mode: Mode, path: Option<PathBuf>) -> Result<Vec<Item>> {
    match mode {
        Mode::Apps => apps::load_applications(),
        Mode::Files => {
            let base = path.unwrap_or_else(|| env::current_dir().unwrap_or_default());
            let mut items = Vec::new();
            for entry in WalkDir::new(&base)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .take(10000)
            {
                let path = entry.path();
                let display = path.strip_prefix(&base)
                    .unwrap_or(path)
                    .display()
                    .to_string();
                items.push(Item {
                    display: display.clone(),
                    value: path.display().to_string(),
                    icon: Some("".to_string()),
                });
            }
            Ok(items)
        }
        Mode::Dirs => {
            let base = path.unwrap_or_else(|| env::current_dir().unwrap_or_default());
            let mut items = Vec::new();
            for entry in WalkDir::new(&base)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
                .take(5000)
            {
                let path = entry.path();
                let display = path.strip_prefix(&base)
                    .unwrap_or(path)
                    .display()
                    .to_string();
                if !display.is_empty() {
                    items.push(Item {
                        display: display.clone(),
                        value: path.display().to_string(),
                        icon: Some("".to_string()),
                    });
                }
            }
            Ok(items)
        }
        Mode::History => {
            let mut items = Vec::new();
            // Try to read zsh history
            if let Some(home) = dirs::home_dir() {
                let hist_file = home.join(".zsh_history");
                if let Ok(content) = fs::read_to_string(&hist_file) {
                    for line in content.lines().rev().take(1000) {
                        // Zsh history format: : timestamp:0;command
                        let cmd = if line.starts_with(':') {
                            line.split(';').nth(1).unwrap_or(line)
                        } else {
                            line
                        };
                        if !cmd.is_empty() {
                            items.push(Item {
                                display: cmd.to_string(),
                                value: cmd.to_string(),
                                icon: None,
                            });
                        }
                    }
                }
            }
            Ok(items)
        }
        Mode::Stdin => {
            let mut items = Vec::new();
            let stdin = io::stdin();
            for line in stdin.lock().lines().take(10000) {
                if let Ok(line) = line {
                    items.push(Item {
                        display: line.clone(),
                        value: line,
                        icon: None,
                    });
                }
            }
            Ok(items)
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App, show_border: bool) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input
            Constraint::Min(1),     // List
        ])
        .split(frame.area());

    // Input box
    let input_block = if show_border {
        Block::default()
            .borders(Borders::ALL)
            .title(" CVH Fuzzy ")
            .border_style(Style::default().fg(Color::Cyan))
    } else {
        Block::default()
    };

    let input = Paragraph::new(format!("> {}", app.query))
        .style(Style::default().fg(Color::White))
        .block(input_block);
    frame.render_widget(input, chunks[0]);

    // Results list
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&idx| {
            let item = &app.items[idx];
            let content = if let Some(ref icon) = item.icon {
                format!("{} {}", icon, item.display)
            } else {
                item.display.clone()
            };
            ListItem::new(Line::from(content))
        })
        .collect();

    let list_block = if show_border {
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {}/{} ", app.filtered.len(), app.items.len()))
            .border_style(Style::default().fg(Color::DarkGray))
    } else {
        Block::default()
    };

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("  ");

    frame.render_stateful_widget(list, chunks[1], &mut app.list_state);
}

fn run_tui(mut app: App, show_border: bool) -> Result<Option<String>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &mut app, show_border))?;

        // Poll for events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key.code, key.modifiers);
            }
        }

        // Tick matcher for async results
        app.matcher.tick(10);
        app.update_filter();

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

    Ok(app.selected_item)
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load items based on mode
    let mode = if args.stdin { Mode::Stdin } else { args.mode };
    let items = load_items(mode, args.path)?;

    // Create app
    let mut app = App::new(items);
    app.query = args.query;
    app.update_filter();

    // Run TUI
    if let Some(selected) = run_tui(app, args.border)? {
        // Handle selection based on mode
        match mode {
            Mode::Apps => {
                // Launch the application
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&selected)
                    .spawn()?;
            }
            _ => {
                // Print the selection
                if args.print0 {
                    print!("{}\0", selected);
                } else {
                    println!("{}", selected);
                }
            }
        }
    }

    Ok(())
}
