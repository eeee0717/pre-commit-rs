use std::fmt::Write;
use std::path::PathBuf;

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::cli::ExitStatus;
use crate::config::Stage;
use crate::hook::{install_hooks, run_hooks, Hook, Project};
use crate::printer::Printer;
use crate::store::Store;

pub(crate) async fn run(
    config: Option<PathBuf>,
    hook_id: Option<String>,
    hook_stage: Option<Stage>,
    from_ref: Option<String>,
    to_ref: Option<String>,
    all_files: bool,
    files: Vec<PathBuf>,
    printer: Printer,
) -> Result<ExitStatus> {
    let store = Store::from_settings()?.init()?;
    let project = Project::current(config)?;

    // TODO: check .pre-commit-config.yaml status and git status
    // TODO: fill env vars
    // TODO: impl staged_files_only

    let lock = store.lock_async().await?;
    let hooks = project.prepare_hooks(&store, printer).await?;

    let hooks: Vec<_> = hooks
        .into_iter()
        .filter(|h| {
            if let Some(ref hook) = hook_id {
                &h.id == hook || &h.alias == hook
            } else {
                true
            }
        })
        .filter(|h| {
            if let Some(stage) = hook_stage {
                h.stages.contains(&stage)
            } else {
                true
            }
        })
        .collect();

    if hooks.is_empty() && hook_id.is_some() {
        if let Some(hook_stage) = hook_stage {
            writeln!(
                printer.stderr(),
                "No hook found for id `{}` and stage `{}`",
                hook_id.unwrap().cyan(),
                hook_stage.cyan()
            )?;
        } else {
            writeln!(
                printer.stderr(),
                "No hook found for id `{}`",
                hook_id.unwrap().cyan()
            )?;
        }
        return Ok(ExitStatus::Failure);
    }

    let skips = get_skips();
    let to_install = hooks
        .iter()
        .filter(|h| !skips.contains(&h.id) && !skips.contains(&h.alias))
        .cloned()
        .collect::<Vec<_>>();

    install_hooks(&to_install, printer).await?;
    drop(lock);

    run_hooks(&hooks, &skips, printer).await?;

    Ok(ExitStatus::Success)
}

fn get_skips() -> Vec<String> {
    match std::env::var_os("SKIP") {
        Some(s) if !s.is_empty() => s
            .to_string_lossy()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>(),
        _ => vec![],
    }
}

// Get all filenames to run hooks on.
// fn all_filenames(
//     hook_stage: Option<Stage>,
//     from_ref: Option<String>,
//     to_ref: Option<String>,
//     all_files: bool,
//     files: Vec<PathBuf>,
//     commit_msg_filename: Option<String>,
// ) -> impl Iterator<Item = String> {
//     if hook_stage.is_some_and(|stage| !stage.operate_on_files()) {
//         return iter::empty();
//     }
//     if hook_stage.is_some_and(|stage| matches!(stage, Stage::PrepareCommitMsg | Stage::CommitMsg)) {
//         return iter::once(commit_msg_filename.unwrap());
//     }
//     match (from_ref, to_ref) {
//         (Some(from_ref), Some(to_ref)) => {
//             return get_changed_files(from_ref, to_ref);
//         }
//         _ => {}
//     }
//     if !files.is_empty() {
//         return files.into_iter().map(|f| f.to_string_lossy().to_string());
//     }
//     if all_files {
//         return get_all_files();
//     }
//     if is_in_merge_conflict() {
//         return get_conflicted_files();
//     }
//     get_staged_files()
// }
