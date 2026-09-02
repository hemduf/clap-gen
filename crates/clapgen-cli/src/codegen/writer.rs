use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{GenerationPlan, OUTPUT_NAMES};

const MANIFEST: &str = "clapgen.manifest.kdl";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct StagedFile {
    path: &'static str,
    temporary: PathBuf,
}

pub(crate) fn write(plan: &GenerationPlan, directory: &Path) -> Result<(), String> {
    write_with_hook(plan, directory, |_| Ok(()))
}

pub(crate) fn write_with_hook<F>(
    plan: &GenerationPlan,
    directory: &Path,
    mut before_publish: F,
) -> Result<(), String>
where
    F: FnMut(&str) -> Result<(), String>,
{
    validate_plan(plan)?;

    let changed = changed_paths(plan, directory)?;
    if changed.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(directory).map_err(|error| {
        format!("failed to create generation output directory `{}`: {error}", directory.display())
    })?;

    let staged = match stage_changed_files(plan, directory, &changed) {
        Ok(staged) => staged,
        Err(error) => return Err(error),
    };

    let manifest_path = directory.join(MANIFEST);
    if let Err(error) = remove_if_exists(&manifest_path) {
        cleanup_temporary_files(&staged);
        return Err(error);
    }

    for staged_file in staged.iter().filter(|file| file.path != MANIFEST) {
        if let Err(error) = before_publish(staged_file.path) {
            cleanup_temporary_files(&staged);
            return Err(error);
        }
        if let Err(error) = replace_complete(&staged_file.temporary, &directory.join(staged_file.path)) {
            cleanup_temporary_files(&staged);
            return Err(error);
        }
    }

    let manifest = staged
        .iter()
        .find(|file| file.path == MANIFEST)
        .expect("validated generation plan always stages the manifest");
    if let Err(error) = before_publish(MANIFEST) {
        cleanup_temporary_files(&staged);
        return Err(error);
    }
    if let Err(error) = replace_complete(&manifest.temporary, &manifest_path) {
        cleanup_temporary_files(&staged);
        return Err(error);
    }

    cleanup_temporary_files(&staged);
    Ok(())
}

fn validate_plan(plan: &GenerationPlan) -> Result<(), String> {
    let valid = plan.files.len() == OUTPUT_NAMES.len()
        && plan.files.iter().zip(OUTPUT_NAMES).all(|(file, expected)| file.path == *expected);
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid generation output contract: expected exactly [{}] in canonical order",
            OUTPUT_NAMES.join(", ")
        ))
    }
}

fn changed_paths(plan: &GenerationPlan, directory: &Path) -> Result<Vec<&'static str>, String> {
    let mut changed = Vec::new();
    for file in &plan.files {
        let path = directory.join(file.path);
        match fs::read(&path) {
            Ok(existing) if existing == file.bytes => {}
            Ok(_) => changed.push(file.path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => changed.push(file.path),
            Err(error) => {
                return Err(format!("failed to read generated output `{}`: {error}", path.display()));
            }
        }
    }
    Ok(changed)
}

fn stage_changed_files(
    plan: &GenerationPlan,
    directory: &Path,
    changed: &[&'static str],
) -> Result<Vec<StagedFile>, String> {
    let mut staged = Vec::new();
    for file in &plan.files {
        if file.path != MANIFEST && !changed.contains(&file.path) {
            continue;
        }
        let temporary = match stage_file(directory, file.path, &file.bytes) {
            Ok(path) => path,
            Err(error) => {
                cleanup_temporary_files(&staged);
                return Err(error);
            }
        };
        staged.push(StagedFile { path: file.path, temporary });
    }
    Ok(staged)
}

fn stage_file(directory: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    for _ in 0..1024 {
        let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary = directory.join(format!(
            ".clapgen-{name}-{}-{nonce}.tmp",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&temporary) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
                    drop(file);
                    let _ = fs::remove_file(&temporary);
                    return Err(format!(
                        "failed to stage generated output `{}`: {error}",
                        temporary.display()
                    ));
                }
                return Ok(temporary);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(format!(
                    "failed to create staged generated output `{}`: {error}",
                    temporary.display()
                ));
            }
        }
    }
    Err(format!(
        "failed to allocate a temporary generated output name in `{}`",
        directory.display()
    ))
}

fn replace_complete(temporary: &Path, destination: &Path) -> Result<(), String> {
    match fs::rename(temporary, destination) {
        Ok(()) => Ok(()),
        Err(first_error) if destination.exists() => {
            replace_existing_with_backup(temporary, destination, first_error)
        }
        Err(error) => Err(format!(
            "failed to publish generated output `{}`: {error}",
            destination.display()
        )),
    }
}

fn replace_existing_with_backup(
    temporary: &Path,
    destination: &Path,
    first_error: std::io::Error,
) -> Result<(), String> {
    let backup = backup_path(destination);
    fs::rename(destination, &backup).map_err(|backup_error| {
        format!(
            "failed to replace generated output `{}` after rename error ({first_error}); failed to preserve old output as `{}`: {backup_error}",
            destination.display(),
            backup.display()
        )
    })?;

    match fs::rename(temporary, destination) {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
            Ok(())
        }
        Err(error) => {
            let restore = fs::rename(&backup, destination);
            if let Err(restore_error) = restore {
                return Err(format!(
                    "failed to publish generated output `{}`: {error}; failed to restore previous output from `{}`: {restore_error}",
                    destination.display(),
                    backup.display()
                ));
            }
            Err(format!("failed to publish generated output `{}`: {error}", destination.display()))
        }
    }
}

fn backup_path(destination: &Path) -> PathBuf {
    let name = destination.file_name().map_or_else(
        || "generated".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    for _ in 0..1024 {
        let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = destination.with_file_name(format!(
            ".clapgen-{name}-{}-{nonce}.backup",
            std::process::id()
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    destination.with_file_name(format!(".clapgen-{name}-{}.backup", std::process::id()))
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to invalidate generation manifest `{}`: {error}", path.display())),
    }
}

fn cleanup_temporary_files(staged: &[StagedFile]) {
    for file in staged {
        let _ = fs::remove_file(&file.temporary);
    }
}
