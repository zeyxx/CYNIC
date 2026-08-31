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

mod app_chat;
mod app_core;
mod app_slash;
mod helpers;
mod types;

pub use helpers::*;
pub use types::{App, ChatLine, Focus, TuiTaskEvent, TuiWindow};
