use std::collections::VecDeque;
use std::sync::mpsc;

use cynic_inference::{ChatService, ChatSessionStore, PrincipalPaths};

use crate::chat_surface::ChatRuntime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Scrollback,
    Prompt,
}

#[derive(Debug, Clone)]
pub(super) enum TuiTaskEvent {
    ChatDone(cynic_inference::ChatTurnReport),
    ModelAliasDone {
        alias: String,
        endpoint: String,
        model: String,
        detail: String,
        log_path: Option<String>,
        service: ChatService,
    },
    ModelRemoteDone {
        model: String,
        endpoint: String,
        service: ChatService,
    },
    TaskError(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum TuiWindow {
    Chat,
    AgentLog,
}

#[derive(Debug, Clone)]
pub(super) struct ChatLine {
    pub role: String,
    pub text: String,
}

#[derive(Debug)]
pub(super) struct App {
    pub paths: PrincipalPaths,
    pub runtime: ChatRuntime,
    pub store: std::sync::Arc<dyn ChatSessionStore>,
    pub service: ChatService,
    pub session_id: String,
    pub lines: Vec<ChatLine>,
    pub input: String,
    pub cursor: usize,
    pub scroll: u16,
    pub focus: Focus,
    pub status: String,
    pub busy: bool,
    pub should_quit: bool,
    pub error: Option<String>,
    pub help_open: bool,
    pub model_picker_open: bool,
    pub model_picker_items: Vec<String>,
    pub model_picker_idx: usize,
    pub task_tx: mpsc::Sender<TuiTaskEvent>,
    pub task_rx: mpsc::Receiver<TuiTaskEvent>,
    pub task_window: TuiWindow,
    pub agent_log: VecDeque<String>,
}
