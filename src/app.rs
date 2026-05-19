use std::io;
use std::time::Instant;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row as TableRow, Table};
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::explorer::ExplorerState;
use crate::results::ResultsState;
use crate::sql::{tokenize, TokenKind};

use crate::autocomplete::AutocompleteState;
use crate::config::Config;
use crate::db::types::{QueryResult, SchemaInfo};
use crate::db::Database;
use crate::editor::EditorBuffer;
use crate::mode::editor::normal::NormalState;
use crate::mode::Mode;
use crate::picker::PickerState;

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
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let (async_tx, async_rx) = mpsc::unbounded_channel();
        Ok(Self {
            mode: Mode::Picker,
            should_quit: false,
            config,
            picker: PickerState::new(),
            db: None,
            focused_pane: FocusedPane::Query,
            editor: EditorBuffer::new(),
            normal_state: NormalState::new(),
            status_message: String::new(),
            results: None,
            query_status: QueryStatus::Idle,
            pending_query: None,
            results_state: ResultsState::new(),
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
                    self.query_status = status;
                    self.results_state.has_next_page = has_next_page;
                    if let Some(r) = result {
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
        use crossterm::event::KeyCode;

        // Global space prefix: space+f toggles maximize (Explorer, QueryNormal, Results)
        if self.pending_space {
            self.pending_space = false;
            if key.code == KeyCode::Char('f') {
                self.toggle_maximize();
                return;
            }
            // Unknown space combo — pass through to mode handler
        }

        let mode = self.mode;
        mode.handle_key(key, self);
    }

    pub fn toggle_maximize(&mut self) {
        if self.maximized.is_some() {
            self.maximized = None;
        } else {
            self.maximized = Some(self.focused_pane);
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

    pub async fn run(&mut self) -> anyhow::Result<()> {
        crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        let mode = self.mode;
                        mode.handle_key(key, self);
                    }
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

        disable_raw_mode()?;
        crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        match self.mode {
            Mode::Picker => self.render_picker(frame, area),
            _ => self.render_main(frame, area),
        }
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
                    Style::default().bg(Color::DarkGray)
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
                                Style::default().bg(Color::Cyan).fg(Color::Black)
                            } else {
                                Style::default()
                            };
                            Line::styled(s.to_string(), style)
                        })
                        .collect();
                    let popup_block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan));
                    let popup = Paragraph::new(items).block(popup_block);
                    frame.render_widget(popup, popup_area);
                }
            }
        }
    }

    fn render_results(&self, frame: &mut ratatui::Frame, area: Rect) {
        let border = self.border_style(FocusedPane::Results);
        let block = Block::default()
            .title(" Results ")
            .borders(Borders::ALL)
            .border_style(border);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(ref result) = self.results {
            if !result.columns.is_empty() {
                let header_cells: Vec<Cell> = result
                    .columns
                    .iter()
                    .map(|c| Cell::from(c.as_str()).style(Style::default().fg(Color::Cyan)))
                    .collect();
                let header = TableRow::new(header_cells)
                    .style(Style::default().add_modifier(Modifier::BOLD));

                let rows: Vec<TableRow> = result
                    .rows
                    .iter()
                    .skip(self.results_state.scroll_row)
                    .take(self.results_state.visible_rows)
                    .enumerate()
                    .map(|(i, row)| {
                        let cells: Vec<Cell> = result
                            .columns
                            .iter()
                            .map(|col| {
                                let val = row.get(col).map(|v| v.to_string()).unwrap_or_default();
                                Cell::from(val)
                            })
                            .collect();
                        let is_selected_row =
                            i + self.results_state.scroll_row == self.results_state.selected_row;
                        let style = if is_selected_row {
                            Style::default().bg(Color::DarkGray)
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
                frame.render_widget(table, inner);
            }
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
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            ratatui::style::Style::default()
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

    fn token_style(kind: &TokenKind) -> Style {
        match kind {
            TokenKind::Keyword => Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            TokenKind::Type => Style::default().fg(Color::Magenta),
            TokenKind::String => Style::default().fg(Color::Green),
            TokenKind::Comment => Style::default().fg(Color::DarkGray),
            TokenKind::Number => Style::default().fg(Color::Yellow),
            TokenKind::Operator => Style::default().fg(Color::Cyan),
            TokenKind::Punctuation => Style::default().fg(Color::White),
            TokenKind::Identifier => Style::default().fg(Color::White),
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
                        Self::token_style(&token.kind),
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
                    .bg(ratatui::style::Color::DarkGray)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );

        let mut state = ratatui::widgets::ListState::default();
        if !filtered_indices.is_empty() {
            state.select(Some(self.picker.selected));
        }

        frame.render_stateful_widget(list, area, &mut state);
    }
}
