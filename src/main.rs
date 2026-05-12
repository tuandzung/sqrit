pub mod db;
pub mod config;
pub mod mode;
pub mod app;
pub mod picker;
pub mod editor;
pub mod sql;
pub mod results;
pub mod clipboard;
pub mod explorer;

use app::App;

fn main() -> anyhow::Result<()> {
    let mut app = App::new()?;
    // tokio runtime for async DB operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app.run())
}
