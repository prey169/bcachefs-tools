use super::events;
use super::json::{self, FsJsonOutput};
use super::utils;

use anyhow::{Context, Result};
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppState {
    Main,
    SortScreen,
}

#[derive(Debug, PartialEq)]
pub enum SortedBy {
    AlphabeticalAscending,
    AlphabeticalDecending,
    TotalAscending,
    TotalDescending,
    SinceStartTotalDiffAscending,
    SinceStartTotalDiffDecending,
}

#[derive(Debug)]
pub struct App {
    pub refresh_rate: Duration,
    pub mountpoint: PathBuf,
    pub state: AppState,
    pub sort_algo: SortedBy,
    pub last_refresh: Instant,
    pub force_refresh: bool,
    pub current_stats: FsJsonOutput,
    pub previous_stats: FsJsonOutput,
    pub starting_stats: FsJsonOutput,
    pub scroll_offset: usize,
    pub alphabetical_sort: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            refresh_rate: Duration::default(),
            mountpoint: PathBuf::default(),
            sort_algo: SortedBy::AlphabeticalAscending,
            state: AppState::Main,
            last_refresh: Instant::now(),
            force_refresh: false,
            current_stats: FsJsonOutput::default(),
            previous_stats: FsJsonOutput::default(),
            starting_stats: FsJsonOutput::default(),
            scroll_offset: 0,
            alphabetical_sort: Vec::new(),
        }
    }

    pub fn with_cli_builder(mountpoint: Option<PathBuf>, time_secs: u64) -> Result<Self> {
        let mut app = Self::new();
        let mountpoint =
            utils::handle_mountpoint(&mountpoint).context("Invalid or missing mountpoint")?;
        let refresh_rate = if time_secs > 0 {
            Duration::from_secs(time_secs)
        } else {
            Duration::from_secs(1)
        };

        app.mountpoint = mountpoint;
        app.refresh_rate = refresh_rate;
        Ok(app)
    }

    pub fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_rate
    }

    pub fn refresh_stats(&mut self) -> Result<()> {
        self.previous_stats = self.current_stats.clone();
        self.current_stats =
            json::json(&Some(self.mountpoint.clone())).context("Initial stats query failed")?;
        self.last_refresh = Instant::now();
        Ok(())
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn max_scroll_offset(&self, visible_rows: usize) -> usize {
        self.alphabetical_sort
            .len()
            .saturating_sub(visible_rows)
            .max(0)
    }
}

pub fn run_app(mountpoint: Option<PathBuf>, time: u64) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::with_cli_builder(mountpoint, time)?;

    let _ = app.refresh_stats();

    app.starting_stats = app.current_stats.clone();
    app.previous_stats = app.current_stats.clone();

    app.alphabetical_sort = app.current_stats.counters.clone().into_keys().collect();
    app.alphabetical_sort.sort_unstable();

    events::run_event_loop(&mut app, &mut terminal)?;

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
