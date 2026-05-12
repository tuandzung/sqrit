use std::io;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use crate::config::Config;
use crate::db::Database;
use crate::mode::Mode;
use crate::picker::PickerState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedPane {
    Explorer,
    Query,
    Results,
}

pub struct App {
    pub mode: Mode,
    pub config: Config,
    pub should_quit: bool,
    pub picker: PickerState,
    pub db: Option<Box<dyn Database>>,
    pub focused_pane: FocusedPane,
    pub query_text: String,
    pub query_cursor: usize,
    pub status_message: String,
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
            query_text: String::new(),
            query_cursor: 0,
            status_message: String::new(),
        })
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
        frame.render_widget(
            Block::default()
                .title(" Explorer ")
                .borders(Borders::ALL)
                .border_style(explorer_border),
            explorer_area,
        );

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

        let query_paragraph = Paragraph::new(self.query_text.as_str());
        frame.render_widget(query_paragraph, query_inner);

        // Results pane
        let results_border = self.border_style(FocusedPane::Results);
        frame.render_widget(
            Block::default()
                .title(" Results ")
                .borders(Borders::ALL)
                .border_style(results_border),
            results_area,
        );

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
