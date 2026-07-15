use std::io;
use std::time::Instant;

use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row as TableRow, Table};
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::explorer::ExplorerState;
use crate::results::ResultsState;
use crate::results_render::{matched_ranges_for, render_cell};
use crate::sql::{tokenize, TokenKind};

use crate::autocomplete::AutocompleteState;
use crate::config::Config;
use crate::db::types::{QueryResult, SchemaInfo};
use crate::db::Database;
use crate::editor::EditorBuffer;
use crate::mode::editor::normal::NormalState;
use crate::mode::Mode;
use crate::picker::PickerState;

/// Install a panic hook that restores the terminal — disables bracketed
/// paste, leaves the alternate screen, and turns raw mode back off — so a
/// panic doesn't leave the user's shell unusable. Idempotent: only
/// installs the hook once even if `run()` is called multiple times.
fn install_panic_hook_for_terminal_restore() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = crossterm::execute!(io::stdout(), DisableBracketedPaste, LeaveAlternateScreen);
            prev(info);
        }));
    });
}

/// RAII guard that restores the terminal on drop. Pairs with the
/// `EnableBracketedPaste` + `enable_raw_mode` setup in `App::run` so a
/// `?`-propagated error mid-loop (e.g. an `event::read()` or `draw()`
/// failure) still leaves the shell usable. The panic hook covers
/// unwinds; this guard covers normal early returns.
struct TerminalRestoreGuard;

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), DisableBracketedPaste, LeaveAlternateScreen);
    }
}

pub enum AsyncResult {
    QueryDone {
        query_id: u64,
        status: QueryStatus,
        result: Option<QueryResult>,
        has_next_page: bool,
    },
    Connected {
        db: Box<dyn Database>,
        schema: Option<SchemaInfo>,
    },
    ConnectFailed(String),
    /// `<space>z` cancel completed. `in_tx` is the connection's transaction
    /// state observed after the cancel landed; status bar uses it to decide
    /// whether to surface the "may need ROLLBACK" hint (see ADR 6).
    Cancelled {
        in_tx: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedPane {
    Explorer,
    Query,
    Results,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryStatus {
    Idle,
    Running,
    Success,
    Error(String),
}

pub struct App {
    pub mode: Mode,
    pub config: Config,
    pub should_quit: bool,
    pub picker: PickerState,
    pub db: Option<Box<dyn Database>>,
    pub focused_pane: FocusedPane,
    pub editor: EditorBuffer,
    pub normal_state: NormalState,
    pub status_message: String,
    pub results: Option<crate::db::types::QueryResult>,
    pub query_status: QueryStatus,
    pub pending_query: Option<String>,
    pub results_state: ResultsState,
    pub fuzzy_filter: crate::filter::FuzzyFilter,
    pub last_query: Option<String>,
    pub explorer_state: ExplorerState,
    pub pending_space: bool,
    pub maximized: Option<FocusedPane>,
    pub autocomplete: AutocompleteState,
    pub last_keystroke: Option<Instant>,
    pub pending_schema_load: bool,
    // Set by picker on connect. Not cleared on disconnect — connection persists
    // across query errors. Reset only when returning to picker or switching connections.
    pub active_connection: Option<String>,
    pub async_rx: mpsc::UnboundedReceiver<AsyncResult>,
    pub async_tx: mpsc::UnboundedSender<AsyncResult>,
    pub query_id: u64,
    pub theme: crate::theme::Theme,
    pub themes_dir: std::path::PathBuf,
    pub theme_picker: Option<crate::mode::theme_picker::ThemePickerState>,
    pub help: Option<crate::mode::help::HelpState>,
    pub cell_viewer: Option<crate::mode::cell_viewer::CellViewerState>,
    pub history_picker: Option<crate::mode::history_picker::HistoryPickerState>,
    pub clipboard_writer: crate::clipboard::ClipboardWriter,
    pub app_config: crate::config::AppConfig,
    pub app_config_path: std::path::PathBuf,
    pub sqrit_dir: std::path::PathBuf,
    pub query_started_at: Option<Instant>,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let (async_tx, async_rx) = mpsc::unbounded_channel();

        // Theming bootstrap: ensure ~/.sqrit/themes/ exists with the bundled defaults,
        // then load the active theme named in ~/.sqrit/config.toml (or default if absent).
        // Paths are anchored at `~/.sqrit/` and error out if `$HOME` is unresolvable
        // rather than silently writing under the current working directory.
        let sqrit_dir = crate::config::AppConfig::sqrit_dir()?;
        let themes_dir = crate::config::AppConfig::themes_dir()?;
        let app_config_path = crate::config::AppConfig::config_path()?;
        let _ = crate::theme::ensure_bundled(&themes_dir);
        let app_config = crate::config::AppConfig::load_from(&app_config_path).unwrap_or_default();
        let (theme, theme_warning) =
            crate::theme::load_active(&themes_dir, app_config.theme.as_deref());
        let mut status_message = String::new();
        if let Some(w) = theme_warning {
            status_message = w;
        }

        Ok(Self {
            mode: Mode::Picker,
            should_quit: false,
            config,
            picker: PickerState::new(),
            db: None,
            focused_pane: FocusedPane::Query,
            editor: EditorBuffer::new(),
            normal_state: NormalState::new(),
            status_message,
            results: None,
            query_status: QueryStatus::Idle,
            pending_query: None,
            results_state: ResultsState::new(),
            fuzzy_filter: crate::filter::FuzzyFilter::new(),
            last_query: None,
            explorer_state: ExplorerState::new(),
            pending_space: false,
            maximized: None,
            autocomplete: AutocompleteState::new(),
            last_keystroke: None,
            pending_schema_load: false,
            active_connection: None,
            async_rx,
            async_tx,
            query_id: 0,
            theme,
            themes_dir,
            theme_picker: None,
            help: None,
            cell_viewer: None,
            history_picker: None,
            clipboard_writer: crate::clipboard::ClipboardWriter::new(),
            app_config,
            app_config_path,
            sqrit_dir,
            query_started_at: None,
        })
    }

    pub fn drain_async_results(&mut self) {
        while let Ok(msg) = self.async_rx.try_recv() {
            match msg {
                AsyncResult::QueryDone {
                    query_id,
                    status,
                    result,
                    has_next_page,
                } => {
                    if query_id != self.query_id {
                        continue;
                    }
                    self.record_history(&status, result.as_ref());
                    self.query_status = status;
                    self.results_state.has_next_page = has_next_page;
                    if let Some(r) = result {
                        let query = self.results_state.filter.as_deref().unwrap_or("");
                        self.results_state.filter_hits = self.fuzzy_filter.rank(&r, query);
                        self.results = Some(r);
                    }
                }
                AsyncResult::Connected { db, schema } => {
                    self.db = Some(db);
                    if let Some(s) = schema {
                        self.explorer_state.schema = Some(s);
                    }
                }
                AsyncResult::ConnectFailed(e) => {
                    self.query_status = QueryStatus::Error(e);
                }
                AsyncResult::Cancelled { in_tx } => {
                    self.status_message = if in_tx {
                        "query cancelled — transaction may need ROLLBACK".to_string()
                    } else {
                        "query cancelled".to_string()
                    };
                    self.query_status = QueryStatus::Idle;
                }
            }
        }
    }

    /// Check if autocomplete should trigger after idle timeout (V7).
    /// Call from event loop; also testable directly.
    pub fn tick_autocomplete(&mut self) {
        if self.mode != Mode::QueryInsert {
            return;
        }
        if let Some(last) = self.last_keystroke {
            if last.elapsed() >= std::time::Duration::from_millis(300) {
                if !self.autocomplete.is_visible() {
                    let text = self.editor.text();
                    let (row, col) = self.editor.cursor();
                    let prefix = crate::autocomplete::current_word_prefix(&text, row, col);
                    let candidates =
                        crate::autocomplete::suggest(&prefix, self.explorer_state.schema.as_ref());
                    if !candidates.is_empty() {
                        self.autocomplete.open(candidates);
                    }
                }
                self.last_keystroke = None;
            }
        }
    }

    /// Process a single key event via the current mode handler.
    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Global help overlay. `?` from QueryNormal / Explorer / Results opens
        // the help modal; inactive in QueryInsert (literal `?`), Picker (typed
        // into the filter), and any modal mode (Help / ThemePicker — those
        // own their own dismiss key).
        if key.modifiers == KeyModifiers::NONE
            && matches!(key.code, KeyCode::Char('?'))
            && matches!(
                self.mode,
                Mode::QueryNormal | Mode::Explorer | Mode::Results
            )
        {
            let origin = self.mode;
            crate::mode::help::open(self, origin);
            return;
        }

        // Global space-prefix command palette (see CONTEXT.md "Command Palette").
        // Active in non-Insert, non-Picker modes. QueryInsert keeps `<space>` as
        // a literal char; Picker types `<space>` into its filter.
        //
        // Palette dispatch only fires for unmodified keys. Modified combos
        // (Ctrl/Alt/Shift) clear the pending flag and fall through to the
        // active mode handler, so e.g. `<space>` then `Ctrl+C` is not the
        // same as `<space>c`.
        if self.pending_space {
            self.pending_space = false;
            if key.modifiers == KeyModifiers::NONE {
                match key.code {
                    KeyCode::Char('f') => {
                        self.toggle_maximize();
                        return;
                    }
                    KeyCode::Char('t') => {
                        let origin = self.mode;
                        crate::mode::theme_picker::enter(self, origin);
                        return;
                    }
                    KeyCode::Char('q') => {
                        self.should_quit = true;
                        return;
                    }
                    KeyCode::Char('c') => {
                        self.mode = Mode::Picker;
                        return;
                    }
                    KeyCode::Char('x') => {
                        self.disconnect_and_return_to_picker();
                        return;
                    }
                    KeyCode::Char('z') => {
                        self.trigger_cancel();
                        return;
                    }
                    KeyCode::Char('h') => {
                        let origin = self.mode;
                        crate::mode::history_picker::open(self, origin);
                        return;
                    }
                    _ => {
                        // Unknown space combo — pass through to mode handler
                    }
                }
            }
        }

        let mode = self.mode;
        mode.handle_key(key, self);
    }

    /// Drop the active DB handle, clear cached schema + connection label,
    /// and route the user back to the connection picker. Centralizes the
    /// `<space>x` disconnect path so future entry points (e.g. a `:disconnect`
    /// command) can't diverge from it.
    pub fn disconnect_and_return_to_picker(&mut self) {
        self.db = None;
        self.active_connection = None;
        self.explorer_state.schema = None;
        self.mode = Mode::Picker;
    }

    pub fn toggle_maximize(&mut self) {
        if self.maximized.is_some() {
            self.maximized = None;
        } else {
            self.maximized = Some(self.focused_pane);
        }
    }

    fn record_history(&mut self, status: &QueryStatus, result: Option<&QueryResult>) {
        let Some(conn) = self.active_connection.as_ref() else {
            return;
        };
        let Some(sql) = self.last_query.clone() else {
            return;
        };
        let duration_ms = self
            .query_started_at
            .take()
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let (status_kind, rows) = match status {
            QueryStatus::Success => {
                let rows = result.map(|r| {
                    r.total_count
                        .unwrap_or(r.rows.len() as u64)
                        .max(r.rows_affected.unwrap_or(0))
                });
                (crate::history::HistoryStatus::Ok, rows)
            }
            QueryStatus::Error(_) => (crate::history::HistoryStatus::Error, None),
            _ => return,
        };
        let entry = crate::history::HistoryEntry {
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            sql,
            duration_ms,
            status: status_kind,
            rows,
        };
        let path = crate::history::history_path_for(&self.sqrit_dir, conn);
        let store = crate::history::HistoryStore::new(path);
        if let Err(e) = store.append(&entry) {
            self.status_message = format!("history append failed: {}", e);
        }
    }

    pub fn execute_pending(&mut self) {
        let query = match self.pending_query.take() {
            Some(q) => q,
            None => return,
        };

        self.query_id += 1;
        let query_id = self.query_id;
        self.query_status = QueryStatus::Running;
        self.query_started_at = Some(Instant::now());
        self.last_query = Some(query.clone());

        let is_select = query.trim_start().to_uppercase().starts_with("SELECT");

        if let Some(ref db) = self.db {
            let db: Box<dyn Database> = db.clone_box();
            let offset = self.results_state.page_offset as u64;
            let limit = self.results_state.page_size as u64 + 1;
            let page_size = self.results_state.page_size;
            let tx = self.async_tx.clone();

            tokio::spawn(async move {
                let result = if is_select {
                    db.execute_paginated(&query, offset, limit).await
                } else {
                    db.execute(&query).await
                };

                let msg = match result {
                    Ok(mut r) => {
                        let has_next_page = if is_select {
                            if r.rows.len() > page_size {
                                r.rows.truncate(page_size);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        AsyncResult::QueryDone {
                            query_id,
                            status: QueryStatus::Success,
                            result: Some(r),
                            has_next_page,
                        }
                    }
                    Err(e) => AsyncResult::QueryDone {
                        query_id,
                        status: QueryStatus::Error(e.to_string()),
                        result: None,
                        has_next_page: false,
                    },
                };
                let _ = tx.send(msg);
            });
        } else {
            self.query_status = QueryStatus::Error("No database connection".to_string());
        }
    }

    /// Fire a DB-level cancel on the running query (see ADR 6). Bumps
    /// `query_id` so any QueryDone that still arrives from the cancelled
    /// future is discarded by the existing guard in `drain_async_results`.
    /// The cancel completion message is delivered via `AsyncResult::Cancelled`.
    pub fn trigger_cancel(&mut self) {
        let Some(db) = self.db.as_ref() else {
            return;
        };
        self.query_id += 1;
        let db = db.clone_box();
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            let _ = db.cancel().await;
            let in_tx = db.in_transaction().await.unwrap_or(false);
            let _ = tx.send(AsyncResult::Cancelled { in_tx });
        });
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        install_panic_hook_for_terminal_restore();
        crossterm::execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
        // Restore terminal on every exit path — `?` propagation mid-loop,
        // normal return, or unwind. Drop order guarantees we run after
        // `terminal` is dropped.
        let _restore = TerminalRestoreGuard;
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        // Go through `handle_key_event` so the space-prefix
                        // dispatcher (<space>t, <space>f) runs before mode
                        // handlers. Bypassing it loses the prefix.
                        self.handle_key_event(key);
                    }
                    Event::Paste(text) => {
                        // V9: paste events bypass the space-prefix
                        // dispatcher — a pasted leading space must not
                        // arm the command palette.
                        self.mode.handler().handle_paste(&text, self);
                    }
                    _ => {}
                }
            }

            self.tick_autocomplete();

            if self.pending_query.is_some() {
                self.execute_pending();
            }

            // Deferred connect + schema load after picker selection
            if self.pending_schema_load {
                if let Some(ref db) = self.db {
                    let mut db = db.clone_box();
                    let tx = self.async_tx.clone();
                    self.pending_schema_load = false;
                    tokio::spawn(async move {
                        if let Err(e) = db.connect().await {
                            let _ = tx.send(AsyncResult::ConnectFailed(e.to_string()));
                            return;
                        }
                        let schema = db.schema_info().await.ok();
                        let _ = tx.send(AsyncResult::Connected { db, schema });
                    });
                }
            }

            // Drain async results (non-blocking)
            self.drain_async_results();

            if self.should_quit {
                break;
            }
        }

        // `TerminalRestoreGuard` runs on drop and handles the
        // disable_raw_mode + DisableBracketedPaste + LeaveAlternateScreen
        // sequence for both the happy path and any `?`-propagated error.
        Ok(())
    }

    pub fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        match self.mode {
            Mode::Picker => self.render_picker(frame, area),
            _ => self.render_main(frame, area),
        }
        if self.mode == Mode::ThemePicker && self.theme_picker.is_some() {
            self.render_theme_picker(frame, area);
        }
        if self.mode == Mode::Help && self.help.is_some() {
            self.render_help(frame, area);
        }
        if self.mode == Mode::CellViewer && self.cell_viewer.is_some() {
            self.render_cell_viewer(frame, area);
        }
        if self.mode == Mode::HistoryPicker && self.history_picker.is_some() {
            self.render_history_picker(frame, area);
        }
    }

    fn render_history_picker(&self, frame: &mut ratatui::Frame, area: Rect) {
        let Some(state) = self.history_picker.as_ref() else {
            return;
        };
        let title = format!(" Query History ({}) ", state.entries.len());
        let modal = Self::cell_viewer_modal_rect(area, title.chars().count());
        if modal.width == 0 || modal.height == 0 {
            return;
        }
        frame.render_widget(Clear, modal);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_focused))
            .style(Style::default().bg(self.theme.bg).fg(self.theme.fg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        let filter_line = format!("/ {}", state.filter);
        frame.render_widget(
            Paragraph::new(filter_line).style(Style::default().fg(self.theme.fg)),
            chunks[0],
        );

        let visible = state.visible();
        let list_h = chunks[1].height as usize;
        let start = state.selected.saturating_sub(list_h.saturating_sub(1));
        let lines: Vec<Line> = visible
            .iter()
            .enumerate()
            .skip(start)
            .take(list_h)
            .map(|(i, entry)| {
                let mut style = Style::default().fg(self.theme.fg);
                if i == state.selected {
                    style = style.bg(self.theme.selection_bg);
                }
                let single = entry.sql.replace('\n', " ");
                Line::from(Span::styled(single, style))
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), chunks[1]);
    }

    /// Modal rect for the cell viewer. Width ~60% of the terminal (clamped
    /// to `max(title_w + 4, 20)` minimum so very narrow terminals still
    /// produce something usable); height ~80%. Centered. Pure — exposed for
    /// testability.
    pub fn cell_viewer_modal_rect(area: Rect, title_w: usize) -> Rect {
        let min_w = (title_w.saturating_add(4) as u16).max(20);
        let desired_w = ((area.width as u32 * 60) / 100) as u16;
        let w = desired_w.max(min_w).min(area.width);
        let desired_h = ((area.height as u32 * 80) / 100) as u16;
        let h = desired_h.max(5).min(area.height);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn render_cell_viewer(&self, frame: &mut ratatui::Frame, area: Rect) {
        let Some(state) = self.cell_viewer.as_ref() else {
            return;
        };
        let view_label = match state.view {
            crate::cell_viewer::ViewMode::Raw => "raw",
            crate::cell_viewer::ViewMode::Formatted => "formatted",
        };
        let title = format!(" Cell — {} ({}) ", state.column, view_label);
        let title_w = title.chars().count();
        let modal = Self::cell_viewer_modal_rect(area, title_w);
        if modal.width == 0 || modal.height == 0 {
            return;
        }
        frame.render_widget(Clear, modal);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_focused))
            .style(Style::default().bg(self.theme.bg).fg(self.theme.fg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        let body = state.displayed();
        let paragraph = Paragraph::new(body)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((state.scroll, 0))
            .style(Style::default().fg(self.theme.fg));
        frame.render_widget(paragraph, inner);
    }

    /// Rendered title for the help overlay block. Single source of truth
    /// shared between `render_help` (drawing) and `help_modal_rect`
    /// (sizing) so the box can't shrink below the title width.
    pub fn help_title(origin: Mode) -> String {
        format!(" Help — {} ", origin.label())
    }

    /// Modal rect for the help overlay. Width fits the longer of the title
    /// and the widest "key" + gutter + "action" row; height fits all
    /// bindings plus borders. Pure — exposed for testability.
    pub fn help_modal_rect(
        area: Rect,
        row_count: usize,
        title_w: usize,
        max_key: usize,
        max_action: usize,
    ) -> Rect {
        let gutter = 2usize; // spaces between key column and action column
        let content_w = max_key.saturating_add(gutter).saturating_add(max_action);
        let desired_w = content_w.max(title_w).saturating_add(4) as u16; // borders + 1ch padding each side
        let desired_h = (row_count as u16).saturating_add(2); // borders
        let w = desired_w.min(area.width);
        let h = desired_h.min(area.height);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn render_help(&self, frame: &mut ratatui::Frame, area: Rect) {
        let Some(state) = self.help.as_ref() else {
            return;
        };
        let bindings = state.origin.handler().bindings();
        let max_key = bindings
            .iter()
            .map(|b| b.key.chars().count())
            .max()
            .unwrap_or(0);
        let max_action = bindings
            .iter()
            .map(|b| b.action.chars().count())
            .max()
            .unwrap_or(0);
        let title = Self::help_title(state.origin);
        let title_w = title.chars().count();
        let modal = Self::help_modal_rect(area, bindings.len(), title_w, max_key, max_action);
        if modal.width == 0 || modal.height == 0 {
            return;
        }
        frame.render_widget(Clear, modal);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_focused))
            .style(Style::default().bg(self.theme.bg).fg(self.theme.fg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        let lines: Vec<Line<'_>> = bindings
            .iter()
            .take(inner.height as usize)
            .map(|b| {
                let pad = max_key.saturating_sub(b.key.chars().count());
                let padding = " ".repeat(pad);
                Line::from(vec![
                    Span::styled(
                        format!("{}{}", b.key, padding),
                        Style::default().fg(self.theme.keyword),
                    ),
                    Span::raw("  "),
                    Span::styled(b.action, Style::default().fg(self.theme.fg)),
                ])
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }

    /// Modal rect sized to fit the longest theme name plus borders/padding,
    /// clamped to `area`. Centered. Pure — exposed for testability.
    /// `max_name_len` is in characters (not bytes) so non-ASCII names lay out correctly.
    pub fn theme_picker_modal_rect(area: Rect, item_count: usize, max_name_len: usize) -> Rect {
        let title_len = " Themes ".chars().count();
        let content_width = max_name_len.max(title_len) as u16;
        let desired_w = content_width.saturating_add(4); // borders + 1ch padding each side
        let desired_h = (item_count as u16).saturating_add(2); // borders
        let w = desired_w.min(area.width);
        let h = desired_h.min(area.height);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn render_theme_picker(&self, frame: &mut ratatui::Frame, area: Rect) {
        let Some(picker) = self.theme_picker.as_ref() else {
            return;
        };
        let max_name = picker
            .available
            .iter()
            .map(|s| s.chars().count())
            .max()
            .unwrap_or(0);
        let modal = Self::theme_picker_modal_rect(area, picker.available.len(), max_name);
        if modal.width == 0 || modal.height == 0 {
            return;
        }
        frame.render_widget(Clear, modal);

        let block = Block::default()
            .title(" Themes ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_focused))
            .style(Style::default().bg(self.theme.bg).fg(self.theme.fg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        let lines: Vec<Line<'_>> = picker
            .available
            .iter()
            .enumerate()
            .take(inner.height as usize)
            .map(|(i, name)| {
                let style = if i == picker.selected {
                    Style::default()
                        .bg(self.theme.selection_bg)
                        .fg(self.theme.fg)
                } else {
                    Style::default().fg(self.theme.fg)
                };
                Line::styled(name.as_str(), style)
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_main(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let status_height = 1u16;
        let main_height = area.height.saturating_sub(status_height);

        let main_area = Rect {
            height: main_height,
            ..area
        };
        let status_area = Rect {
            y: area.y + main_height,
            height: status_height,
            ..area
        };

        // Maximized: render only focused pane full-screen
        if let Some(maximized_pane) = self.maximized {
            match maximized_pane {
                FocusedPane::Explorer => {
                    self.prepare_explorer_viewport(main_area);
                    self.render_explorer(frame, main_area);
                }
                FocusedPane::Query => self.render_query(frame, main_area),
                FocusedPane::Results => self.render_results(frame, main_area),
            }
            let status_text = self.status_bar_text();
            frame.render_widget(Paragraph::new(status_text), status_area);
            return;
        }

        // Normal 3-pane layout
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(25), Constraint::Min(0)])
            .split(main_area);

        let explorer_area = main_chunks[0];
        let right_area = main_chunks[1];

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(right_area);

        let query_area = right_chunks[0];
        let results_area = right_chunks[1];

        self.prepare_explorer_viewport(explorer_area);
        self.render_explorer(frame, explorer_area);
        self.render_query(frame, query_area);
        self.render_results(frame, results_area);

        let status_text = self.status_bar_text();
        frame.render_widget(Paragraph::new(status_text), status_area);
    }

    fn prepare_explorer_viewport(&mut self, area: Rect) {
        let inner = Block::default().borders(Borders::ALL).inner(area);
        self.explorer_state.set_viewport(inner.height as usize);
    }

    fn render_explorer(&self, frame: &mut ratatui::Frame, area: Rect) {
        let border = self.border_style(FocusedPane::Explorer);
        let block = Block::default()
            .title(" Explorer ")
            .borders(Borders::ALL)
            .border_style(border);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items = self.explorer_state.items();
        let scroll_offset = self.explorer_state.scroll_offset;
        let visible_rows = self.explorer_state.visible_rows;
        let selected = self.explorer_state.selected;
        let lines: Vec<Line<'_>> = items
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_rows)
            .map(|(i, item)| {
                let display = match item {
                    crate::explorer::TreeItem::Table { name, expanded } => {
                        let arrow = if *expanded { 'v' } else { '>' };
                        format!("{} {}", arrow, name)
                    }
                    crate::explorer::TreeItem::Column {
                        name, data_type, ..
                    } => {
                        format!("  {} ({})", name, data_type)
                    }
                    crate::explorer::TreeItem::View { name, expanded } => {
                        let arrow = if *expanded { 'v' } else { '>' };
                        format!("{} {}", arrow, name)
                    }
                    crate::explorer::TreeItem::ViewColumn {
                        name, data_type, ..
                    } => {
                        format!("  {} ({})", name, data_type)
                    }
                };
                let style = if i == selected {
                    Style::default().bg(self.theme.selection_bg)
                } else {
                    Style::default()
                };
                Line::styled(display.to_string(), style)
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }

    /// Computes scroll offset and absolute terminal position for the INSERT mode cursor.
    /// Returns `(scroll_offset, term_x, term_y)`.
    /// Pure function — no side effects, fully testable.
    pub fn insert_cursor_position(
        cursor_row: usize,
        cursor_col: usize,
        inner: Rect,
    ) -> (u16, u16, u16) {
        let inner_h = inner.height as usize;
        let scroll_usize = if inner_h > 0 && cursor_row + 1 > inner_h {
            cursor_row + 1 - inner_h
        } else {
            0
        };
        // All arithmetic stays in usize; truncate to u16 only at the boundary.
        let term_x_usize =
            (inner.x as usize + cursor_col).min(inner.right().saturating_sub(1) as usize);
        let term_y_usize = inner.y as usize + cursor_row - scroll_usize;
        let scroll_offset = scroll_usize.min(u16::MAX as usize) as u16;
        let term_x = term_x_usize.min(u16::MAX as usize) as u16;
        let term_y = term_y_usize.min(u16::MAX as usize) as u16;
        (scroll_offset, term_x, term_y)
    }

    fn render_query(&self, frame: &mut ratatui::Frame, area: Rect) {
        let border = self.border_style(FocusedPane::Query);
        let mode_label = match self.mode {
            Mode::QueryNormal => " NORMAL ",
            Mode::QueryInsert => " INSERT ",
            _ => "",
        };
        let block = Block::default()
            .title(format!(" Query {} ", mode_label))
            .borders(Borders::ALL)
            .border_style(border);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let cursor_row = self.editor.cursor_row();
        let cursor_col = self.editor.cursor_col();
        let (scroll_offset, term_x, term_y) =
            Self::insert_cursor_position(cursor_row, cursor_col, inner);

        let query_text = self.highlighted_lines();
        let query_paragraph = Paragraph::new(query_text).scroll((scroll_offset, 0));
        frame.render_widget(query_paragraph, inner);

        // Show terminal cursor in INSERT mode (V8).
        if self.mode == Mode::QueryInsert {
            frame.set_cursor_position(ratatui::layout::Position {
                x: term_x,
                y: term_y,
            });
        }

        // Autocomplete popup — y position accounts for scroll offset.
        if self.autocomplete.is_visible() {
            let filtered = self.autocomplete.filtered();
            if !filtered.is_empty() {
                let max_visible = 8usize;
                let popup_height = filtered.len().min(max_visible) as u16 + 2;
                let popup_width = 30u16;
                let visible_cursor_row = cursor_row as u16 - scroll_offset;
                let popup_y = inner.y + visible_cursor_row.saturating_add(1).min(inner.height);
                let popup_x = inner.x + cursor_col as u16;
                let popup_area = Rect {
                    x: popup_x.min(inner.right().saturating_sub(popup_width)),
                    y: popup_y.min(inner.bottom().saturating_sub(popup_height)),
                    width: popup_width.min(inner.width),
                    height: popup_height.min(inner.bottom().saturating_sub(popup_y)),
                };
                if popup_area.width > 0 && popup_area.height > 0 {
                    frame.render_widget(Clear, popup_area);
                    let items: Vec<Line<'_>> = filtered
                        .iter()
                        .take(max_visible)
                        .enumerate()
                        .map(|(i, s)| {
                            let style = if i == self.autocomplete.selected_index() {
                                Style::default()
                                    .bg(self.theme.border_focused)
                                    .fg(self.theme.bg)
                            } else {
                                Style::default()
                            };
                            Line::styled(s.to_string(), style)
                        })
                        .collect();
                    let popup_block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(self.theme.border_focused));
                    let popup = Paragraph::new(items).block(popup_block);
                    frame.render_widget(popup, popup_area);
                }
            }
        }
    }

    fn render_results(&self, frame: &mut ratatui::Frame, area: Rect) {
        let border = self.border_style(FocusedPane::Results);
        let title = match self.results_state.filter.as_deref() {
            Some(term) if !term.is_empty() => format!(" Results (filter: {}) ", term),
            _ => " Results ".to_string(),
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let show_prompt = self.mode == Mode::ResultsFilter;
        let (table_area, prompt_area) = if show_prompt && inner.height >= 1 {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            (chunks[0], Some(chunks[1]))
        } else {
            (inner, None)
        };

        if let Some(ref result) = self.results {
            if !result.columns.is_empty() {
                let selected_col = self.results_state.selected_col;
                let selected_row = self.results_state.selected_row;
                let header_cells: Vec<Cell> = result
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(col_idx, c)| {
                        let mut style = Style::default().fg(self.theme.border_focused);
                        if col_idx == selected_col {
                            style = style.add_modifier(Modifier::REVERSED);
                        }
                        Cell::from(c.name.as_str()).style(style)
                    })
                    .collect();
                let header = TableRow::new(header_cells)
                    .style(Style::default().add_modifier(Modifier::BOLD));

                let visible = self.results_state.visible_row_indices(result);
                let rows: Vec<TableRow> = visible
                    .iter()
                    .skip(self.results_state.scroll_row)
                    .take(self.results_state.visible_rows)
                    .map(|&row_idx| {
                        let cells: Vec<Cell> = result
                            .columns
                            .iter()
                            .enumerate()
                            .map(|(col_idx, col)| {
                                let val = result.rows[row_idx]
                                    .get(&col.name)
                                    .map(|v| v.to_string())
                                    .unwrap_or_default();
                                // Build the cell style additively so any future
                                // per-cell fg/bg (e.g. NULL dim, error red) is
                                // preserved; REVERSED only flips fg/bg at the
                                // terminal layer, leaving row tint visible.
                                let mut style = Style::default();
                                if row_idx == selected_row && col_idx == selected_col {
                                    style = style.add_modifier(Modifier::REVERSED);
                                }
                                let matched_ranges = matched_ranges_for(
                                    &self.results_state.filter_hits,
                                    row_idx,
                                    col_idx,
                                );
                                Cell::from(Line::from(render_cell(
                                    &val,
                                    matched_ranges,
                                    &self.theme,
                                )))
                                .style(style)
                            })
                            .collect();
                        let style = if row_idx == selected_row {
                            Style::default().bg(self.theme.selection_bg)
                        } else {
                            Style::default()
                        };
                        TableRow::new(cells).style(style)
                    })
                    .collect();

                let widths: Vec<ratatui::layout::Constraint> = result
                    .columns
                    .iter()
                    .map(|_| ratatui::layout::Constraint::Ratio(1, result.columns.len() as u32))
                    .collect();

                let table = Table::new(rows, &widths).header(header);
                frame.render_widget(table, table_area);
            }
        }

        if let Some(prompt_area) = prompt_area {
            let filter = self.results_state.filter.as_deref().unwrap_or("");
            let prompt = format!("/ {}", filter);
            frame.render_widget(
                Paragraph::new(prompt).style(Style::default().fg(self.theme.fg)),
                prompt_area,
            );
        }
    }

    /// Switch mode and focused_pane to the given pane. Single source of truth
    /// for `e`/`q`/`r` pane-focus shortcuts.
    pub fn switch_pane(&mut self, mode: Mode, pane: FocusedPane) {
        self.mode = mode;
        self.focused_pane = pane;
    }

    pub fn border_style(&self, pane: FocusedPane) -> ratatui::style::Style {
        let is_focused = match self.maximized {
            Some(maximized) => maximized == pane,
            None => self.focused_pane == pane,
        };
        if is_focused {
            ratatui::style::Style::default().fg(self.theme.border_focused)
        } else {
            ratatui::style::Style::default().fg(self.theme.border_unfocused)
        }
    }

    pub fn status_bar_text(&self) -> String {
        let mode_str = self.mode.label();
        let conn = self.active_connection.as_deref().unwrap_or("no connection");

        let query_status: &str = match &self.query_status {
            QueryStatus::Idle => "",
            QueryStatus::Running => "running...",
            QueryStatus::Success => "ok",
            QueryStatus::Error(_) => "", // handled below
        };

        let status = if let QueryStatus::Error(e) = &self.query_status {
            format!("ERR: {}", e)
        } else if self.status_message.is_empty() {
            query_status.to_string()
        } else if query_status.is_empty() {
            self.status_message.clone()
        } else {
            format!("{} | {}", query_status, self.status_message)
        };

        format!(" {} | {} | {}", mode_str, conn, status)
    }

    fn token_style(&self, kind: &TokenKind) -> Style {
        match kind {
            TokenKind::Keyword => Style::default()
                .fg(self.theme.keyword)
                .add_modifier(Modifier::BOLD),
            TokenKind::Type => Style::default().fg(self.theme.type_),
            TokenKind::String => Style::default().fg(self.theme.string),
            TokenKind::Comment => Style::default().fg(self.theme.comment),
            TokenKind::Number => Style::default().fg(self.theme.number),
            TokenKind::Operator => Style::default().fg(self.theme.border_focused),
            TokenKind::Punctuation => Style::default().fg(self.theme.fg),
            TokenKind::Identifier => Style::default().fg(self.theme.fg),
            TokenKind::Whitespace => Style::default(),
        }
    }

    fn highlighted_lines(&self) -> Vec<Line<'_>> {
        let text = self.editor.text();
        let tokens = tokenize(&text);
        let mut lines: Vec<Line<'_>> = Vec::new();
        let mut current_spans: Vec<Span<'_>> = Vec::new();

        for token in tokens {
            for (i, line_text) in token.text.split('\n').enumerate() {
                if i > 0 {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                if !line_text.is_empty() {
                    current_spans.push(Span::styled(
                        line_text.to_string(),
                        self.token_style(&token.kind),
                    ));
                }
            }
        }

        if current_spans.is_empty() && lines.is_empty() {
            lines.push(Line::from(""));
        } else if !current_spans.is_empty() {
            lines.push(Line::from(current_spans));
        }

        lines
    }

    fn render_picker(&self, frame: &mut ratatui::Frame, area: Rect) {
        let filtered_indices = self.picker.filtered_indices(self);
        let items: Vec<ratatui::widgets::ListItem> = if filtered_indices.is_empty() {
            vec![ratatui::widgets::ListItem::new(
                "No connections found. Press 'n' to add one.",
            )]
        } else {
            filtered_indices
                .iter()
                .map(|&idx| {
                    let conn = &self.config.connections[idx];
                    let db_type = match conn.db_type {
                        crate::config::DbType::Sqlite => "SQLite",
                        crate::config::DbType::Postgres => "PG",
                        crate::config::DbType::Mysql => "MySQL",
                    };
                    ratatui::widgets::ListItem::new(format!("[{}] {}", db_type, conn.name))
                })
                .collect()
        };

        let title = if self.picker.filter.is_empty() {
            " sqrit — Connections".to_string()
        } else {
            format!(" sqrit — Connections (filter: {})", self.picker.filter)
        };

        let list = ratatui::widgets::List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(
                ratatui::style::Style::default()
                    .bg(self.theme.selection_bg)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );

        let mut state = ratatui::widgets::ListState::default();
        if !filtered_indices.is_empty() {
            state.select(Some(self.picker.selected));
        }

        frame.render_stateful_widget(list, area, &mut state);
    }
}
