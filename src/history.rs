use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub ts: String,
    pub sql: String,
    pub duration_ms: u64,
    pub status: HistoryStatus,
    pub rows: Option<u64>,
}

pub const HISTORY_CAP: usize = 500;

pub fn history_path_for(sqrit_dir: &Path, connection_name: &str) -> PathBuf {
    let sanitized: String = connection_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    let basename = if trimmed.is_empty() {
        "unnamed-conn"
    } else {
        trimmed
    };
    sqrit_dir
        .join("history")
        .join(format!("{}.jsonl", basename))
}

pub struct HistoryStore {
    path: PathBuf,
    cap: usize,
}

impl HistoryStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cap: HISTORY_CAP,
        }
    }

    pub fn with_cap(path: PathBuf, cap: usize) -> Self {
        Self { path, cap }
    }

    pub fn append(&self, entry: &HistoryEntry) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
        drop(file);
        self.trim_to_cap()?;
        Ok(())
    }

    fn trim_to_cap(&self) -> anyhow::Result<()> {
        let entries = self.load()?;
        if entries.len() <= self.cap {
            return Ok(());
        }
        let tail = &entries[entries.len() - self.cap..];
        let mut file = std::fs::File::create(&self.path)?;
        for entry in tail {
            let line = serde_json::to_string(entry)?;
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }

    pub fn load(&self) -> anyhow::Result<Vec<HistoryEntry>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let file = std::fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: HistoryEntry = serde_json::from_str(&line)?;
            out.push(entry);
        }
        Ok(out)
    }
}
