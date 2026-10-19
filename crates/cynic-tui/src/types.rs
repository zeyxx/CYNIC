//! Dashboard model: plain data, no rendering, no I/O. Every domain TUI
//! (fleet, audit, agents, builds) projects onto this and lets the kit draw it.

/// One table panel: headers plus rows of plain strings.
#[derive(Debug, Clone)]
pub struct TablePanel {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// One text panel: preformatted lines.
#[derive(Debug, Clone)]
pub struct TextPanel {
    pub title: String,
    pub lines: Vec<String>,
}

/// Ordered dashboard content. Panels render top to bottom in order.
#[derive(Debug, Clone)]
pub enum Panel {
    Table(TablePanel),
    Text(TextPanel),
}

/// A full dashboard snapshot with a title and a status-bar hint.
#[derive(Debug, Clone)]
pub struct Dashboard {
    pub title: String,
    pub panels: Vec<Panel>,
    pub status_hint: String,
}

impl Dashboard {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            panels: Vec::new(),
            status_hint: String::from("q quit · r refresh"),
        }
    }

    pub fn push(&mut self, panel: Panel) {
        self.panels.push(panel);
    }
}
