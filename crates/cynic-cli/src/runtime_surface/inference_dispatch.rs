use std::process::ExitCode;

use super::{RuntimeCommand, inference_surface};

pub(crate) fn try_execute_inference(command: RuntimeCommand) -> Result<ExitCode, RuntimeCommand> {
    match command {
        RuntimeCommand::Inference { json } => Ok(inference_surface::execute_inference(json)),
        RuntimeCommand::InferenceVault { json } => {
            Ok(inference_surface::execute_inference_vault(json))
        }
        RuntimeCommand::InferenceStatus { json, tail } => {
            Ok(inference_surface::execute_inference_status(json, tail))
        }
        RuntimeCommand::InferenceStart {
            alias,
            json,
            yes,
            no_wait,
        } => Ok(inference_surface::execute_inference_start(
            Some(alias.as_str()),
            json,
            yes,
            no_wait,
        )),
        RuntimeCommand::InferenceStop { alias, json, yes } => Ok(
            inference_surface::execute_inference_stop(alias.as_deref(), json, yes),
        ),
        RuntimeCommand::InferenceDiscover {
            json,
            apply,
            open_webui,
        } => Ok(inference_surface::execute_inference_discover(
            json, apply, open_webui,
        )),
        RuntimeCommand::InferenceModels {
            json,
            persist,
            ensure_profiles,
        } => Ok(inference_surface::execute_inference_models(
            json,
            persist,
            ensure_profiles,
        )),
        RuntimeCommand::InferenceModelsConsolidate { json, yes } => Ok(
            inference_surface::execute_inference_models_consolidate(json, yes),
        ),
        RuntimeCommand::InferenceHfStatus { json } => {
            Ok(inference_surface::execute_inference_hf_status(json))
        }
        RuntimeCommand::InferenceHfDownload {
            repo,
            json,
            revision,
            include,
            exclude,
            repo_type,
            cache_only,
            alias,
            register_profile,
            no_profile,
        } => Ok(inference_surface::execute_inference_hf_download(
            &repo,
            json,
            &inference_surface::HfDownloadArgs {
                revision: revision.as_deref(),
                include: &include,
                exclude: &exclude,
                repo_type: repo_type.as_deref(),
                cache_only,
                alias: alias.as_deref(),
                register_profile: register_profile && !no_profile,
            },
        )),
        RuntimeCommand::InferenceHfCategories { json } => {
            Ok(inference_surface::execute_inference_hf_categories(json))
        }
        RuntimeCommand::InferenceHardware { json } => {
            Ok(inference_surface::execute_inference_hardware(json))
        }
        RuntimeCommand::InferenceHfSearch {
            json,
            category,
            query,
            author,
            limit,
        } => Ok(inference_surface::execute_inference_hf_search(
            json,
            category.as_deref(),
            query.as_deref(),
            author.as_deref(),
            limit,
        )),
        RuntimeCommand::InferenceHfSuggest {
            json,
            category,
            limit,
        } => Ok(inference_surface::execute_inference_hf_suggest(
            json, &category, limit,
        )),
        RuntimeCommand::InferencePlan { alias, json, apply } => Ok(
            inference_surface::execute_inference_plan(&alias, json, apply),
        ),
        RuntimeCommand::InferenceForge {
            report_path,
            source_note,
            json,
            yes,
        } => Ok(inference_surface::execute_inference_forge(
            &report_path, &source_note, json, yes,
        )),
        other => Err(other),
    }
}
