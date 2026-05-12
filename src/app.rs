use std::io;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Table, Row as TableRow, Cell};
use ratatui::Terminal;

use crate::sql::{TokenKind, tokenize};
use crate::results::ResultsState;
use crate::explorer::ExplorerState;

use crate::config::Config;
use crate::db::Database;
use crate::editor::EditorBuffer;
use crate::mode::Mode;
use crate::mode::editor::normal::NormalState;
use crate::picker::PickerState;

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
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let config = Config::load()?;
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
        })
    }

    pub async fn execute_pending(&mut self) {
        let query = match self.pending_query.take() {
            Some(q) => q,
            None => return,
        };

        self.query_status = QueryStatus::Running;
        self.last_query = Some(query.clone());

        let is_select = query.trim_start().to_uppercase().starts_with("SELECT");

        if let Some(ref db) = self.db {
            let result = if is_select {
                let offset = self.results_state.page_offset as u64;
                let limit = self.results_state.page_size as u64 + 1;
                db.execute_paginated(&query, offset, limit).await
            } else {
                db.execute(&query).await
            };

            match result {
                Ok(mut r) => {
                    if is_select {
                        if r.rows.len() > self.results_state.page_size {
                            self.results_state.has_next_page = true;
                            r.rows.truncate(self.results_state.page_size);
                        } else {
                            self.results_state.has_next_page = false;
                        }
                    }
                    self.results = Some(r);
                    self.query_status = QueryStatus::Success;
                }
                Err(e) => {
                    self.query_status = QueryStatus::Error(e.to_string());
                }
            }
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

            if self.pending_query.is_some() {
                self.execute_pending().await;
            }

            if self.should_quit {
                break;
            }
        }

        disable_raw_mode()?;
        crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        match self.mode {
            Mode::Picker => self.render_picker(frame, area),
            _ => self.render_main(frame, area),
        }
    }

    fn render_main(&self, frame: &mut ratatui::Frame, area: Rect) {
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

        // Explorer | Right side
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(25), Constraint::Min(0)])
            .split(main_area);

        let explorer_area = main_chunks[0];
        let right_area = main_chunks[1];

        // Query | Results
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(right_area);

        let query_area = right_chunks[0];
        let results_area = right_chunks[1];

        // Explorer pane
        let explorer_border = self.border_style(FocusedPane::Explorer);
        let explorer_block = Block::default()
            .title(" Explorer ")
            .borders(Borders::ALL)
            .border_style(explorer_border);
        let explorer_inner = explorer_block.inner(explorer_area);
        frame.render_widget(explorer_block, explorer_area);

        let items = self.explorer_state.items();
        let lines: Vec<Line<'_>> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let display = match item {
                    crate::explorer::TreeItem::Table { name, expanded } => {
                        let arrow = if *expanded { 'v' } else { '>' };
                        format!("{} {}", arrow, name)
                    }
                    crate::explorer::TreeItem::Column { name, data_type, .. } => {
                        format!("  {} ({})", name, data_type)
                    }
                    crate::explorer::TreeItem::View { name, expanded } => {
                        let arrow = if *expanded { 'v' } else { '>' };
                        format!("{} {}", arrow, name)
                    }
                    crate::explorer::TreeItem::ViewColumn { name, data_type, .. } => {
                        format!("  {} ({})", name, data_type)
                    }
                };
                let style = if i == self.explorer_state.selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                Line::styled(display.to_string(), style)
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), explorer_inner);

        // Query pane
        let query_border = self.border_style(FocusedPane::Query);
        let mode_label = match self.mode {
            Mode::QueryNormal => " NORMAL ",
            Mode::QueryInsert => " INSERT ",
            _ => "",
        };
        let query_block = Block::default()
            .title(format!(" Query {} ", mode_label))
            .borders(Borders::ALL)
            .border_style(query_border);
        let query_inner = query_block.inner(query_area);
        frame.render_widget(query_block, query_area);

        let query_text = self.highlighted_lines();
        let query_paragraph = Paragraph::new(query_text);
        frame.render_widget(query_paragraph, query_inner);

        // Results pane
        let results_border = self.border_style(FocusedPane::Results);
        let results_block = Block::default()
            .title(" Results ")
            .borders(Borders::ALL)
            .border_style(results_border);
        let results_inner = results_block.inner(results_area);
        frame.render_widget(results_block, results_area);

        if let Some(ref result) = self.results {
            if !result.columns.is_empty() {
                let header_cells: Vec<Cell> = result.columns.iter()
                    .map(|c| Cell::from(c.as_str()).style(Style::default().fg(Color::Cyan)))
                    .collect();
                let header = TableRow::new(header_cells)
                    .style(Style::default().add_modifier(Modifier::BOLD));

                let rows: Vec<TableRow> = result.rows.iter()
                    .skip(self.results_state.scroll_row)
                    .take(self.results_state.visible_rows)
                    .enumerate()
                    .map(|(i, row)| {
                        let cells: Vec<Cell> = result.columns.iter()
                            .map(|col| {
                                let val = row.get(col).map(|v| v.to_string()).unwrap_or_default();
                                Cell::from(val)
                            })
                            .collect();
                        let is_selected_row = i + self.results_state.scroll_row == self.results_state.selected_row;
                        let style = if is_selected_row {
                            Style::default().bg(Color::DarkGray)
                        } else {
                            Style::default()
                        };
                        TableRow::new(cells).style(style)
                    })
                    .collect();

                let widths: Vec<ratatui::layout::Constraint> = result.columns.iter()
                    .map(|_| ratatui::layout::Constraint::Ratio(1, result.columns.len() as u32))
                    .collect();

                let table = Table::new(rows, &widths)
                    .header(header);
                frame.render_widget(table, results_inner);
            }
        }

        // Status bar
        let conn_name = self
            .config
            .connections
            .first()
            .map(|c| c.name.as_str())
            .unwrap_or("none");
        let mode_str = match self.mode {
            Mode::Explorer => "EXPLORER",
            Mode::QueryNormal => "NORMAL",
            Mode::QueryInsert => "INSERT",
            Mode::Results => "RESULTS",
            _ => "",
        };
        let status_text = format!(" {} | {} | {}", mode_str, conn_name, self.status_message);
        frame.render_widget(Paragraph::new(status_text), status_area);
    }

    pub fn border_style(&self, pane: FocusedPane) -> ratatui::style::Style {
        if self.focused_pane == pane {
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            ratatui::style::Style::default()
        }
    }

    fn token_style(kind: &TokenKind) -> Style {
        match kind {
            TokenKind::Keyword => Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
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
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL),
            )
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
