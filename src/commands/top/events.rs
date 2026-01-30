use super::app::{App, AppState};
use super::components::render_app_components;

use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    Terminal,
};
use std::{error::Error, io, mem::take, time::Duration};

pub fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match app.state {
        AppState::Main => event_main_state(app, key),
        AppState::SortScreen => event_feedback_state(app, key),
    }
}

fn event_main_state(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _)
        | (KeyCode::Char('Q'), _)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            return Ok(Some(()));
        }
        (KeyCode::Char('r'), _) => {
            app.force_refresh = true;
        }
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
            app.scroll_down(1);
        }
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
            app.scroll_up(1);
        }
        _ => {}
    }
    Ok(None)
}

fn event_feedback_state(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _)
        | (KeyCode::Char('Q'), _)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            return Ok(Some(()));
        }
        (KeyCode::Char('r'), _) => {
            app.force_refresh = true;
        }
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
            app.scroll_offset = app.scroll_offset.saturating_add(1);
            app.force_refresh = true;
        }
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
            app.force_refresh = true;
        }
        _ => {}
    }
    Ok(None)
}

pub fn run_event_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| render_app_components(frame, app))?;

        if app.should_refresh() || take(&mut app.force_refresh) {
            let _ = app.refresh_stats();
        }

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if let Ok(Some(())) = handle_key_event(app, key) {
                return Ok(());
            }
        }
    }
}
