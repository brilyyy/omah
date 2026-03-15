use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod ui;

use app::{App, ConfirmKind, EditState, Screen, StepFormState};

pub fn run(config_path: &Path) -> Result<()> {
    let mut app = App::new(config_path.to_path_buf())?;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop<B>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B: ratatui::backend::Backend,
    B::Error: Send + Sync + 'static,
{
    loop {
        // Auto-advance splash after 1.5 s
        if let Screen::Splash(start) = &app.screen {
            if start.elapsed().as_millis() > 1500 {
                app.screen = Screen::List;
            }
        }

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let screen_kind: u8 = match &app.screen {
                    Screen::Splash(_) => 5,
                    Screen::List => 0,
                    Screen::AddForm => 1,
                    Screen::Confirm(ConfirmKind::RestoreOne) => 2,
                    Screen::Confirm(ConfirmKind::RestoreAll) => 3,
                    Screen::Edit => 4,
                    Screen::Settings => 6,
                };

                let should_quit = match screen_kind {
                    5 => { app.screen = Screen::List; false }
                    0 => handle_list(app, key.code),
                    1 => { handle_form(app, key.code); false }
                    2 => { handle_confirm(app, ConfirmKind::RestoreOne, key.code); false }
                    3 => { handle_confirm(app, ConfirmKind::RestoreAll, key.code); false }
                    4 => { handle_edit(app, key.code); false }
                    6 => { handle_settings(app, key.code); false }
                    _ => false,
                };

                if should_quit { break; }
            }
        }
    }
    Ok(())
}

// ── List screen ────────────────────────────────────────────────────────────

fn handle_list(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.statuses.is_empty() {
                app.selected = (app.selected + 1).min(app.statuses.len() - 1);
            }
            app.message = None;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.selected = app.selected.saturating_sub(1);
            app.message = None;
        }
        KeyCode::Char('n') => {
            app.form.reset();
            app.screen = Screen::AddForm;
            app.message = None;
        }
        KeyCode::Char('e') => {
            if !app.statuses.is_empty() {
                let idx = app.selected;
                app.open_edit(idx);
            }
        }
        KeyCode::Char('S') => app.open_settings(),
        KeyCode::Char('b') => app.backup_selected(),
        KeyCode::Char('B') => app.backup_all(),
        KeyCode::Char('r') => {
            if !app.statuses.is_empty() {
                app.screen = Screen::Confirm(ConfirmKind::RestoreOne);
                app.message = None;
            }
        }
        KeyCode::Char('R') => {
            if !app.statuses.is_empty() {
                app.screen = Screen::Confirm(ConfirmKind::RestoreAll);
                app.message = None;
            }
        }
        _ => {}
    }
    false
}

// ── Add form ───────────────────────────────────────────────────────────────

fn handle_form(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => { app.screen = Screen::List; app.message = None; }
        KeyCode::Tab => app.form.field = (app.form.field + 1) % 3,
        KeyCode::BackTab => app.form.field = app.form.field.checked_sub(1).unwrap_or(2),
        KeyCode::Enter => {
            if app.form.field < 2 { app.form.field += 1; } else { app.save_new_dot(); }
        }
        KeyCode::Char(' ') if app.form.field == 2 => app.form.symlink = !app.form.symlink,
        KeyCode::Char(c) => match app.form.field {
            0 => app.form.name.push(c),
            1 => app.form.source.push(c),
            _ => {}
        },
        KeyCode::Backspace => match app.form.field {
            0 => { app.form.name.pop(); }
            1 => { app.form.source.pop(); }
            _ => {}
        },
        _ => {}
    }
}

// ── Confirm dialog ─────────────────────────────────────────────────────────

fn handle_confirm(app: &mut App, kind: ConfirmKind, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Enter => {
            match kind {
                ConfirmKind::RestoreOne => app.restore_selected(),
                ConfirmKind::RestoreAll => app.restore_all(),
            }
            app.screen = Screen::List;
        }
        KeyCode::Char('n') | KeyCode::Esc => { app.screen = Screen::List; app.message = None; }
        _ => {}
    }
}

// ── Edit screen ────────────────────────────────────────────────────────────

fn handle_edit(app: &mut App, key: KeyCode) {
    if app.edit.step_form.is_some() {
        handle_edit_step_form(&mut app.edit, key);
        return;
    }
    if app.edit.exclude_input.is_some() {
        handle_edit_exclude_input(&mut app.edit, key);
        return;
    }
    match key {
        KeyCode::Esc => { app.screen = Screen::List; app.message = None; }
        KeyCode::Char('s') => app.save_edit(),
        KeyCode::Tab => app.edit.focus = (app.edit.focus + 1) % 6,
        KeyCode::BackTab => app.edit.focus = app.edit.focus.checked_sub(1).unwrap_or(5),
        other => match app.edit.focus {
            0..=3 => handle_edit_fields(&mut app.edit, other),
            4 => handle_edit_steps(&mut app.edit, other),
            5 => handle_edit_excludes(&mut app.edit, other),
            _ => {}
        },
    }
}

fn handle_edit_fields(edit: &mut EditState, key: KeyCode) {
    match key {
        KeyCode::Char(' ') if edit.focus == 2 => edit.symlink = !edit.symlink,
        KeyCode::Enter => { if edit.focus < 5 { edit.focus += 1; } }
        KeyCode::Char(c) => match edit.focus {
            0 => edit.name.push(c),
            1 => edit.source.push(c),
            3 => edit.deps.push(c),
            _ => {}
        },
        KeyCode::Backspace => match edit.focus {
            0 => { edit.name.pop(); }
            1 => { edit.source.pop(); }
            3 => { edit.deps.pop(); }
            _ => {}
        },
        _ => {}
    }
}

fn handle_edit_steps(edit: &mut EditState, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if !edit.steps.is_empty() {
                edit.step_sel = (edit.step_sel + 1).min(edit.steps.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => edit.step_sel = edit.step_sel.saturating_sub(1),
        KeyCode::Char('a') => edit.step_form = Some(StepFormState::new()),
        KeyCode::Char('d') => {
            if !edit.steps.is_empty() {
                edit.steps.remove(edit.step_sel);
                if edit.step_sel > 0 && edit.step_sel >= edit.steps.len() {
                    edit.step_sel -= 1;
                }
            }
        }
        _ => {}
    }
}

fn handle_edit_excludes(edit: &mut EditState, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if !edit.excludes.is_empty() {
                edit.exclude_sel = (edit.exclude_sel + 1).min(edit.excludes.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            edit.exclude_sel = edit.exclude_sel.saturating_sub(1);
        }
        KeyCode::Char('a') => edit.exclude_input = Some(String::new()),
        KeyCode::Char('d') => {
            if !edit.excludes.is_empty() {
                edit.excludes.remove(edit.exclude_sel);
                if edit.exclude_sel > 0 && edit.exclude_sel >= edit.excludes.len() {
                    edit.exclude_sel -= 1;
                }
            }
        }
        _ => {}
    }
}

fn handle_edit_step_form(edit: &mut EditState, key: KeyCode) {
    let form = match edit.step_form.as_mut() {
        Some(f) => f,
        None => return,
    };
    match key {
        KeyCode::Esc => edit.step_form = None,
        KeyCode::Tab | KeyCode::Enter if form.field == 0 => form.field = 1,
        KeyCode::BackTab => { if form.field > 0 { form.field -= 1; } }
        KeyCode::Enter if form.field == 1 => {
            let install = form.install.trim().to_string();
            if !install.is_empty() {
                let check = form.check.trim().to_string();
                let sel = edit.steps.len();
                edit.steps.push(app::StepEdit { check, install });
                edit.step_sel = sel;
            }
            edit.step_form = None;
        }
        KeyCode::Char(c) => match form.field {
            0 => form.check.push(c),
            1 => form.install.push(c),
            _ => {}
        },
        KeyCode::Backspace => match form.field {
            0 => { form.check.pop(); }
            1 => { form.install.pop(); }
            _ => {}
        },
        _ => {}
    }
}

fn handle_edit_exclude_input(edit: &mut EditState, key: KeyCode) {
    let buf = match edit.exclude_input.as_mut() {
        Some(b) => b,
        None => return,
    };
    match key {
        KeyCode::Esc => edit.exclude_input = None,
        KeyCode::Enter => {
            let pat = buf.trim().to_string();
            if !pat.is_empty() {
                let sel = edit.excludes.len();
                edit.excludes.push(pat);
                edit.exclude_sel = sel;
            }
            edit.exclude_input = None;
        }
        KeyCode::Char(c) => buf.push(c),
        KeyCode::Backspace => { buf.pop(); }
        _ => {}
    }
}

// ── Settings screen ────────────────────────────────────────────────────────

fn handle_settings(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => { app.screen = Screen::List; app.message = None; }
        KeyCode::Char('s') => app.save_settings(),
        KeyCode::Tab => app.settings.focus = (app.settings.focus + 1) % 2,
        KeyCode::BackTab => app.settings.focus = app.settings.focus.checked_sub(1).unwrap_or(1),
        KeyCode::Enter => app.settings.focus = (app.settings.focus + 1) % 2,
        KeyCode::Char(c) => match app.settings.focus {
            0 => app.settings.os.push(c),
            1 => app.settings.pkg_manager.push(c),
            _ => {}
        },
        KeyCode::Backspace => match app.settings.focus {
            0 => { app.settings.os.pop(); }
            1 => { app.settings.pkg_manager.pop(); }
            _ => {}
        },
        _ => {}
    }
}
