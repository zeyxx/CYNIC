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

include!("app_core.rs");
include!("app_chat.rs");
include!("app_slash.rs");
