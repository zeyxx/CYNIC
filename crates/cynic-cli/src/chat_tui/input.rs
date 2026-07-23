//! Keyboard handling: translate terminal events into TUI actions.

use std::io;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::actions::TuiAction;
use super::state::{App, Focus};

pub(super) fn handle_key(
    app: &mut App,
    key: KeyEvent,
    _rt: &tokio::runtime::Runtime,
) -> io::Result<Vec<TuiAction>> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return Ok(vec![TuiAction::Quit]);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('n')) {
        return Ok(vec![TuiAction::NewSession]);
    }

    if app.help_open {
        return Ok(match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
                vec![TuiAction::ToggleHelp]
            }
            _ => Vec::new(),
        });
    }

    if app.model_picker_open {
        return Ok(match key.code {
            KeyCode::Esc | KeyCode::Char('q') => vec![TuiAction::ModelPickerClose],
            KeyCode::Up | KeyCode::Char('k') => vec![TuiAction::ModelPickerPrev],
            KeyCode::Down | KeyCode::Char('j') => vec![TuiAction::ModelPickerNext],
            KeyCode::Enter => vec![TuiAction::ModelPickerSubmit],
            _ => Vec::new(),
        });
    }

    let mut actions = Vec::new();
    if app.focus == Focus::Scrollback {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => actions.push(TuiAction::FocusPrompt),
            KeyCode::Char(' ') => actions.push(TuiAction::FocusPrompt),
            KeyCode::F(2) => actions.push(TuiAction::ToggleTaskWindow),
            KeyCode::Esc => actions.push(TuiAction::ClearOrQuit),
            KeyCode::Up | KeyCode::Char('k') => actions.push(TuiAction::ScrollScrollbackUp),
            KeyCode::Down | KeyCode::Char('j') => actions.push(TuiAction::ScrollScrollbackDown),
            KeyCode::PageUp => actions.push(TuiAction::ScrollScrollbackPageUp),
            KeyCode::PageDown => actions.push(TuiAction::ScrollScrollbackPageDown),
            KeyCode::Home | KeyCode::Char('g') => actions.push(TuiAction::ScrollScrollbackTop),
            KeyCode::End | KeyCode::Char('G') => actions.push(TuiAction::ScrollScrollbackBottom),
            KeyCode::Char('i') => actions.push(TuiAction::FocusPrompt),
            KeyCode::Char('?') => actions.push(TuiAction::ToggleHelp),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                actions.push(TuiAction::FocusPrompt);
                actions.push(TuiAction::PromptInsertChar(c));
            }
            _ => {}
        }
        return Ok(actions);
    }

    match key.code {
        KeyCode::Tab | KeyCode::BackTab => actions.push(TuiAction::FocusScrollback),
        KeyCode::F(1) => actions.push(TuiAction::OpenModelPicker),
        KeyCode::F(2) => actions.push(TuiAction::ToggleTaskWindow),
        KeyCode::Esc => actions.push(TuiAction::ClearOrQuit),
        KeyCode::Enter => {
            if !app.input.trim().is_empty() {
                let prompt = app.input.clone();
                actions.push(TuiAction::PromptSubmit { prompt });
            }
        }
        KeyCode::Backspace => actions.push(TuiAction::PromptBackspace),
        KeyCode::Left => actions.push(TuiAction::PromptLeft),
        KeyCode::Right => actions.push(TuiAction::PromptRight),
        KeyCode::Home => actions.push(TuiAction::PromptHome),
        KeyCode::End => actions.push(TuiAction::PromptEnd),
        KeyCode::PageUp => actions.push(TuiAction::ScrollScrollbackPageUp),
        KeyCode::PageDown => actions.push(TuiAction::ScrollScrollbackPageDown),
        KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            actions.push(TuiAction::OpenModelPicker)
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            actions.push(TuiAction::PromptClear);
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            actions.push(TuiAction::PromptInsertChar(c));
        }
        _ => {}
    }

    Ok(actions)
}
