//! Small operator-facing projection over the native inference lifecycle.
//!
//! The inference crate remains the owner of inventory and supervisor behavior.
//! This module only gives the operator a compact, explicit-consent vocabulary.

#![allow(dead_code)]
use std::process::ExitCode;

use clap::Subcommand;

use super::inference_surface;

#[derive(Subcommand)]
pub(crate) enum OperatorModelsCommand {
    /// List declared model profiles and whether each local weight is present.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Read the current local model supervisor state.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Start one local model profile after explicit operator consent.
    Start {
        /// Profile alias; `auto` selects a startable local profile.
        #[arg(default_value = "auto")]
        alias: String,
        #[arg(long)]
        json: bool,
        /// Explicitly authorize starting a local process.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        no_wait: bool,
        /// Optional OneShotCapability token (ed25519 hex) required for mutation.
        #[arg(long)]
        capability_token: Option<String>,
    },
    /// Stop the currently supervised local model after explicit operator consent.
    Stop {
        /// Optional profile alias; defaults to the running profile.
        alias: Option<String>,
        #[arg(long)]
        json: bool,
        /// Explicitly authorize stopping the local process.
        #[arg(long)]
        yes: bool,
    },
}

pub(crate) fn execute_operator_models(command: OperatorModelsCommand) -> ExitCode {
    match command {
        OperatorModelsCommand::List { json } => inference_surface::execute_inference_vault(json),
        OperatorModelsCommand::Status { json } => {
            inference_surface::execute_inference_status(json, 0)
        }
        OperatorModelsCommand::Start {
            alias,
            json,
            yes,
            no_wait,
            capability_token,
        } => inference_surface::execute_inference_start(
            Some(&alias),
            json,
            yes,
            no_wait,
            capability_token.as_deref(),
        ),
        OperatorModelsCommand::Stop { alias, json, yes } => {
            inference_surface::execute_inference_stop(alias.as_deref(), json, yes)
        }
    }
}
