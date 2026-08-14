use crate::command_surface::{HostCommandSurface, argv_command, expectation};

use super::integration_plan_types::{IntegrationExecutionPlan, IntegrationPlanAction};
use super::integration_types::{IntegrationObservation, IntegrationObservationContext};

pub(crate) const HF_INSTALL_URL: &str = "https://hf.co/cli/install.sh";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationRecipe {
    pub requires_apply: bool,
    pub privilege_required: bool,
    pub proposal: IntegrationExecutionPlan,
    pub connect_execution: Vec<HostCommandSurface>,
    pub adopt_execution: Vec<HostCommandSurface>,
}

pub fn build_integration_recipe(
    observation: &IntegrationObservation,
    context: &IntegrationObservationContext,
    action: IntegrationPlanAction,
) -> IntegrationRecipe {
    match observation.id.as_str() {
        "github" => github_recipe(observation, action),
        "huggingface" => huggingface_recipe(observation, context, action),
        _ => IntegrationRecipe {
            requires_apply: false,
            privilege_required: false,
            proposal: IntegrationExecutionPlan {
                preflight: vec![],
                apply: vec![],
                observe: vec![],
                proofs_required: observation.proofs.clone(),
                rollback_intent: vec![],
            },
            connect_execution: vec![],
            adopt_execution: vec![],
        },
    }
}

fn github_recipe(
    observation: &IntegrationObservation,
    action: IntegrationPlanAction,
) -> IntegrationRecipe {
    let cli = observation
        .path_or_endpoint
        .clone()
        .unwrap_or_else(|| "gh".to_string());
    let version = argv_command("github_cli_version", vec![cli.clone(), "--version".into()]);
    let auth_status = argv_command(
        "github_auth_status",
        vec![cli, "auth".into(), "status".into()],
    );
    let requires_apply = action != IntegrationPlanAction::None;
    IntegrationRecipe {
        requires_apply,
        privilege_required: false,
        proposal: IntegrationExecutionPlan {
            preflight: vec![version.clone()],
            apply: if requires_apply {
                vec![
                    expectation(
                        "install_or_import_gh",
                        "install or import the operator-local `gh` CLI before attempting GitHub adoption",
                    ),
                    expectation(
                        "connect_gh_account",
                        "connect the operator-local GitHub account with `gh auth login` or an existing credential store",
                    ),
                ]
            } else {
                vec![]
            },
            observe: vec![auth_status],
            proofs_required: vec![
                "github_cli_present".into(),
                "github_auth_status_connected".into(),
            ],
            rollback_intent: if requires_apply {
                vec![
                    "logout the operator-local GitHub session if the wrong account was adopted"
                        .into(),
                ]
            } else {
                vec![]
            },
        },
        connect_execution: vec![
            expectation(
                "connect_github_account",
                "run `gh auth login` under the operator account or import an existing GitHub credential store without exposing the token",
            ),
            expectation(
                "verify_github_account",
                "re-run `gh auth status` and verify the adopted operator account is the intended relay identity",
            ),
        ],
        adopt_execution: vec![],
    }
}

fn huggingface_recipe(
    observation: &IntegrationObservation,
    context: &IntegrationObservationContext,
    action: IntegrationPlanAction,
) -> IntegrationRecipe {
    let privilege_required = context.operator_user != context.runtime_user;
    let runtime_hf = format!("{}/.local/bin/hf", context.runtime_home);
    let preflight = runtime_home_preflight(context, privilege_required);
    let observe = runtime_hf_observe(context, &runtime_hf, privilege_required);
    let requires_apply = action != IntegrationPlanAction::None;

    let proposal_apply = match action {
        IntegrationPlanAction::ConnectRuntimeCli => vec![expectation(
            "connect_runtime_hf",
            "connect the runtime principal with `hf auth login` or inject HF_TOKEN through env/file without exposing the secret",
        )],
        IntegrationPlanAction::AdoptRuntimeCli => vec![
            expectation(
                "install_or_import_runtime_hf",
                "install or import `hf` under the runtime principal instead of relying on an operator-local binary",
            ),
            expectation(
                "connect_runtime_hf",
                "connect the runtime principal with `hf auth login` or inject HF_TOKEN through env/file without exposing the secret",
            ),
        ],
        _ => vec![],
    };

    let rollback_intent = match action {
        IntegrationPlanAction::ConnectRuntimeCli => vec![
            "remove or rotate the runtime credential material if the wrong account was connected"
                .into(),
        ],
        IntegrationPlanAction::AdoptRuntimeCli => vec![
            "remove the incorrect runtime Hugging Face session with `hf auth logout` under the runtime principal"
                .into(),
        ],
        _ => vec![],
    };

    IntegrationRecipe {
        requires_apply,
        privilege_required,
        proposal: IntegrationExecutionPlan {
            preflight,
            apply: proposal_apply,
            observe,
            proofs_required: vec![
                "runtime_hf_cli_present".into(),
                "runtime_hf_account_observed_without_secret".into(),
            ],
            rollback_intent,
        },
        connect_execution: vec![
            expectation(
                "connect_runtime_hf",
                "run `hf auth login` as the runtime principal or inject HF_TOKEN through env/file without exposing the secret value",
            ),
            expectation(
                "verify_runtime_hf_account",
                "re-run `hf auth whoami --format json` as the runtime principal and verify the adopted account matches intent",
            ),
        ],
        adopt_execution: hf_adopt_execution(observation, context),
    }
}

fn hf_adopt_execution(
    observation: &IntegrationObservation,
    context: &IntegrationObservationContext,
) -> Vec<HostCommandSurface> {
    let runtime_bin = format!("{}/.local/bin", context.runtime_home);
    let runtime_hf = format!("{runtime_bin}/hf");
    let bootstrap_dir = format!("{}/.local/share/cynic/bootstrap", context.runtime_home);
    let installer_path = format!("{bootstrap_dir}/hf-cli-install.sh");
    let needs_install = observation.observed_principal.is_none() || !observation.principal_matches;

    let mut commands = vec![
        argv_command(
            "runtime_home_present",
            vec!["test".into(), "-d".into(), context.runtime_home.clone()],
        ),
        argv_command(
            "runtime_principal_sudo_check",
            runtime_wrapped_argv(context, &["true"], false),
        ),
        argv_command(
            "ensure_runtime_bin_dir",
            runtime_wrapped_argv(
                context,
                &["install", "-d", "-m", "0755", runtime_bin.as_str()],
                false,
            ),
        ),
        argv_command(
            "ensure_hf_bootstrap_dir",
            runtime_wrapped_argv(
                context,
                &["install", "-d", "-m", "0755", bootstrap_dir.as_str()],
                false,
            ),
        ),
    ];

    if needs_install {
        commands.push(argv_command(
            "download_hf_install_script",
            runtime_wrapped_argv(
                context,
                &[
                    "curl",
                    "-LsSf",
                    HF_INSTALL_URL,
                    "-o",
                    installer_path.as_str(),
                ],
                false,
            ),
        ));
        commands.push(argv_command(
            "run_hf_install_script",
            runtime_wrapped_argv(context, &["bash", installer_path.as_str()], false),
        ));
    }

    commands.push(argv_command(
        "runtime_hf_version",
        runtime_wrapped_argv(context, &[runtime_hf.as_str(), "version"], true),
    ));
    commands
}

fn runtime_home_preflight(
    context: &IntegrationObservationContext,
    privilege_required: bool,
) -> Vec<HostCommandSurface> {
    let mut preflight = vec![argv_command(
        "runtime_home_present",
        vec!["test".into(), "-d".into(), context.runtime_home.clone()],
    )];
    if privilege_required {
        preflight.push(argv_command(
            "runtime_principal_sudo_check",
            wrap_runtime_argv(context, &["true"]),
        ));
    }
    preflight
}

fn runtime_hf_observe(
    context: &IntegrationObservationContext,
    runtime_hf: &str,
    privilege_required: bool,
) -> Vec<HostCommandSurface> {
    let version_argv = if privilege_required {
        wrap_runtime_hf_argv(context, runtime_hf, &["version"])
    } else {
        vec![runtime_hf.into(), "version".into()]
    };
    let whoami_argv = if privilege_required {
        wrap_runtime_hf_argv(context, runtime_hf, &["auth", "whoami", "--format", "json"])
    } else {
        vec![
            runtime_hf.into(),
            "auth".into(),
            "whoami".into(),
            "--format".into(),
            "json".into(),
        ]
    };

    vec![
        argv_command("runtime_hf_version", version_argv),
        argv_command("runtime_hf_auth_whoami", whoami_argv),
    ]
}

fn wrap_runtime_hf_argv(
    context: &IntegrationObservationContext,
    runtime_hf: &str,
    arguments: &[&str],
) -> Vec<String> {
    let mut argv = wrap_runtime_argv(context, &[runtime_hf]);
    argv.extend(arguments.iter().map(|argument| (*argument).to_string()));
    argv
}

fn wrap_runtime_argv(context: &IntegrationObservationContext, arguments: &[&str]) -> Vec<String> {
    let mut argv = vec![
        "sudo".into(),
        "-n".into(),
        "-u".into(),
        context.runtime_user.clone(),
        "--".into(),
        "env".into(),
        format!("HOME={}", context.runtime_home),
    ];
    argv.extend(arguments.iter().map(|argument| (*argument).to_string()));
    argv
}

fn runtime_wrapped_argv(
    context: &IntegrationObservationContext,
    arguments: &[&str],
    preserve_hf_token: bool,
) -> Vec<String> {
    let mut argv = vec!["sudo".to_string()];
    if preserve_hf_token && std::env::var_os("HF_TOKEN").is_some() {
        argv.push("--preserve-env=HF_TOKEN".to_string());
    }
    argv.extend([
        "-n".to_string(),
        "-u".to_string(),
        context.runtime_user.clone(),
        "--".to_string(),
        "env".to_string(),
        format!("HOME={}", context.runtime_home),
    ]);
    argv.extend(arguments.iter().map(|argument| (*argument).to_string()));
    argv
}
