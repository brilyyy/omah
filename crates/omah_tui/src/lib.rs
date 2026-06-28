use std::io::{self, Stdout};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub mod app;
pub mod dep_flow;
pub mod ops;
pub mod screens;
pub mod theme;
pub mod ui;
pub mod widgets;

/// Enter the TUI event loop.
/// `config_path` — path to the omah config file.
pub fn run(config_path: &Path) -> Result<()> {
    let mut app = app::App::new(config_path.to_path_buf());
    app.load_config();

    let mut terminal = init_terminal()?;
    let result = run_tui(&mut terminal, &mut app);
    restore_terminal()?;
    result
}

// ── Terminal lifecycle ───────────────────────────────────────────────────

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}

// ── Event loop ───────────────────────────────────────────────────────────

fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                Event::Resize(_, _) => {
                    // ratatui handles resize automatically on next draw
                }
                _ => {}
            }
        }

        app.tick();
    }
    Ok(())
}
