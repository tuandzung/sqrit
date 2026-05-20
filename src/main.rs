pub mod app;
pub mod autocomplete;
pub mod clipboard;
pub mod config;
pub mod db;
pub mod editor;
pub mod explorer;
pub mod mode;
pub mod picker;
pub mod results;
pub mod sql;
pub mod theme;

use app::App;

fn main() -> anyhow::Result<()> {
    let mut app = App::new()?;
    // tokio runtime for async DB operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app.run())
}
