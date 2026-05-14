use crate::db::types::QueryResult;

pub fn format_cell(result: &QueryResult, row: usize, col: usize) -> Option<String> {
    let col_name = result.columns.get(col)?;
    let r = result.rows.get(row)?;
    r.get(col_name).map(|v| v.to_string())
}

pub fn format_row(result: &QueryResult, row: usize) -> Option<String> {
    let r = result.rows.get(row)?;
    let vals: Vec<String> = result
        .columns
        .iter()
        .map(|c| r.get(c).map(|v| v.to_string()).unwrap_or_default())
        .collect();
    Some(vals.join("\t"))
}

pub fn format_all(result: &QueryResult) -> String {
    let header = result.columns.join("\t");
    let rows: Vec<String> = result
        .rows
        .iter()
        .enumerate()
        .map(|(i, _)| format_row(result, i).unwrap_or_default())
        .collect();
    std::iter::once(header)
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_csv(result: &QueryResult) -> String {
    let header = result
        .columns
        .iter()
        .map(|c| csv_escape(c))
        .collect::<Vec<_>>()
        .join(",");
    let rows: Vec<String> = result
        .rows
        .iter()
        .map(|r| {
            result
                .columns
                .iter()
                .map(|c| {
                    r.get(c)
                        .map(|v| csv_escape(&v.to_string()))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    std::iter::once(header)
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_json(result: &QueryResult) -> String {
    let objects: Vec<String> = result
        .rows
        .iter()
        .map(|r| {
            let pairs: Vec<String> = result
                .columns
                .iter()
                .map(|c| {
                    let val = r.get(c).map(|v| v.to_string()).unwrap_or_default();
                    format!("{}:{}", json_escape(c), json_escape(&val))
                })
                .collect();
            format!("{{{}}}", pairs.join(","))
        })
        .collect();
    format!("[{}]", objects.join(","))
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn json_escape(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
