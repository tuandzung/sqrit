use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::db::types::ObjectRef;
use crate::mode::{KeyBinding, Mode, ModeHandler};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionContent {
    Loading,
    Ready(String),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionViewerState {
    pub object: ObjectRef,
    pub content: DefinitionContent,
    pub scroll: u16,
}

pub struct DefinitionViewerHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "j / k",
        action: "Scroll down / up",
    },
    KeyBinding {
        key: "y",
        action: "Copy definition",
    },
    KeyBinding {
        key: "Esc",
        action: "Close definition viewer",
    },
];

impl ModeHandler for DefinitionViewerHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(state) = app.definition_viewer.as_mut() {
                    state.scroll = state.scroll.saturating_add(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(state) = app.definition_viewer.as_mut() {
                    state.scroll = state.scroll.saturating_sub(1);
                }
            }
            KeyCode::Char('y') => copy_ready(app),
            KeyCode::Esc => close(app),
            _ => {}
        }
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }
}

pub fn open(app: &mut App, object: ObjectRef) {
    app.definition_request_id = app.definition_request_id.wrapping_add(1);
    let request_id = app.definition_request_id;
    app.definition_viewer = Some(DefinitionViewerState {
        object: object.clone(),
        content: DefinitionContent::Loading,
        scroll: 0,
    });
    app.mode = Mode::DefinitionViewer;

    let Some(db) = app.db.as_ref().map(|db| db.clone_box()) else {
        app.definition_viewer.as_mut().unwrap().content =
            DefinitionContent::Error("No database connection".into());
        return;
    };
    let tx = app.async_tx.clone();
    tokio::spawn(async move {
        let result = db
            .object_definition(&object)
            .await
            .map_err(|error| error.to_string());
        let _ = tx.send(crate::app::AsyncResult::DefinitionLoaded { request_id, result });
    });
}

fn copy_ready(app: &mut App) {
    let text = match app.definition_viewer.as_ref() {
        Some(DefinitionViewerState {
            content: DefinitionContent::Ready(text),
            ..
        }) => text.clone(),
        _ => return,
    };
    let char_count = text.chars().count();
    app.status_message = copy_status(app.clipboard_writer.copy(&text), char_count);
}

fn copy_status(result: anyhow::Result<()>, char_count: usize) -> String {
    match result {
        Ok(()) => format!("Copied definition ({char_count} chars)"),
        Err(error) => format!("Failed to copy definition: {error}"),
    }
}

fn close(app: &mut App) {
    app.definition_viewer = None;
    app.mode = Mode::Explorer;
}

#[cfg(test)]
mod tests {
    use super::copy_status;

    #[test]
    fn copy_status_reports_success() {
        assert_eq!(copy_status(Ok(()), 12), "Copied definition (12 chars)");
    }

    #[test]
    fn copy_status_surfaces_clipboard_error() {
        assert_eq!(
            copy_status(Err(anyhow::anyhow!("clipboard unavailable")), 12),
            "Failed to copy definition: clipboard unavailable"
        );
    }
}
