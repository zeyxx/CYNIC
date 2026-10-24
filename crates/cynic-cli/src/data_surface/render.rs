//! ASCII table formatting and rendering for CLI tabular query outputs.

use cynic_storage::TabularQueryResult;

/// Prints an aligned ASCII table to stdout from query columns and row values.
pub fn print_ascii_table(res: &TabularQueryResult) {
    if res.columns.is_empty() {
        println!("(No columns returned)");
        return;
    }

    // Determine column widths
    let mut widths: Vec<usize> = res.columns.iter().map(|c| c.len()).collect();
    for row in &res.rows {
        for (i, val) in row.iter().enumerate() {
            if i < widths.len() {
                let s = format_val(val);
                if s.len() > widths[i] {
                    widths[i] = s.len().min(60);
                }
            }
        }
    }

    // Border line
    let border = format!(
        "+{}+",
        widths
            .iter()
            .map(|w| "-".repeat(w + 2))
            .collect::<Vec<_>>()
            .join("+")
    );

    println!("{border}");

    // Header
    let header = format!(
        "|{}|",
        res.columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let w = widths[i];
                format!(" {:<w$} ", col, w = w)
            })
            .collect::<Vec<_>>()
            .join("|")
    );
    println!("{header}");
    println!("{border}");

    // Rows
    for row in &res.rows {
        let row_str = format!(
            "|{}|",
            row.iter()
                .enumerate()
                .map(|(i, val)| {
                    let w = widths.get(i).copied().unwrap_or(10);
                    let s = format_val(val);
                    let truncated = if s.len() > w {
                        format!("{}...", &s[..w.saturating_sub(3)])
                    } else {
                        s
                    };
                    format!(" {:<w$} ", truncated, w = w)
                })
                .collect::<Vec<_>>()
                .join("|")
        );
        println!("{row_str}");
    }

    println!("{border}");
}

fn format_val(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => val.to_string(),
    }
}
