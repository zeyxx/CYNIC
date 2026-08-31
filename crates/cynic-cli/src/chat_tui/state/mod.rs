use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use super::actions::{TuiAction, TuiEffect};
use crate::chat_surface::{ChatRuntime, resolve_selector_to_alias};
use cynic_inference::{
    ChatEvent, ChatEventKind, ChatService, ChatSessionStore, LocalRuntimeStatus, PrincipalPaths,
};

mod helpers;
mod types;

pub use helpers::*;
pub use types::{App, ChatLine, Focus, TuiTaskEvent, TuiWindow};

// App impl stays here for now — next slice will split 1100 impl into app_chat/app_model/slash (<400 each)
// For this slice: types 69 + helpers 72 + mod facade 1100 still >400 but types/helpers extracted as first step
// Re-include original App impl via include! for minimal risk (keeps behavior, proves modular boundary)
include!("app_impl.rs");
