use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::outputs::{GeneratedFile, GenerationPlan, OUTPUT_NAMES};
use super::writer::{write, write_with_hook};

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn plan(marker: &str) -> GenerationPlan {
    GenerationPlan {
        files: OUTPUT_NAMES
            .iter()
            .copied()
            .map(|path| GeneratedFile {
                path,
                bytes: format!("{path}:{marker}\n").into_bytes(),
            })
            .collect(),
    }
}

fn read(path: &Path, file: &str) -> Vec<u8> {
    fs::read(path.join(file)).expect("generated file should be readable")
}

fn temp_entries(path: &Path) -> Vec<String> {
    fs::read_dir(path)
        .expect("output directory should be readable")
        .map(|entry| entry.expect("directory entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".clapgen-") || name.ends_with(".tmp"))
        .collect()
}

#[test]
fn writer_publishes_the_complete_plan_and_manifest_last() {
    let directory = temporary_directory("issue40-publish-order");
    let current = plan("v1");
    let mut order = Vec::new();

    write_with_hook(&current, &directory, |path| {
        order.push(path.to_owned());
        Ok(())
    })
    .expect("generation plan should publish");

    assert_eq!(OUTPUT_NAMES.len(), order.len());
    assert_eq!(Some(&"clapgen.manifest.kdl".to_owned()), order.last());
    for file in &current.files {
        assert_eq!(file.bytes, read(&directory, file.path));
    }
    assert!(temp_entries(&directory).is_empty(), "temporary files leaked");

    fs::remove_dir_all(directory).expect("temporary directory should be removable");
}

#[test]
fn no_op_generation_preserves_every_output_timestamp() {
    let directory = temporary_directory("issue40-no-op");
    let current = plan("stable");
    write(&current, &directory).expect("initial generation should publish");
    let before = current
        .files
        .iter()
        .map(|file| {
            (
                file.path,
                fs::metadata(directory.join(file.path))
                    .expect("metadata")
                    .modified()
                    .expect("modified time"),
            )
        })
        .collect::<Vec<_>>();

    let mut order = Vec::new();
    write_with_hook(&current, &directory, |path| {
        order.push(path.to_owned());
        Ok(())
    })
    .expect("no-op generation should succeed");

    assert!(order.is_empty(), "no-op generation published files: {order:?}");
    for (path, modified) in before {
        assert_eq!(
            modified,
            fs::metadata(directory.join(path)).expect("metadata").modified().expect("modified time"),
            "mtime changed for {path}"
        );
    }
    assert!(temp_entries(&directory).is_empty(), "temporary files leaked");

    fs::remove_dir_all(directory).expect("temporary directory should be removable");
}

#[test]
fn changed_generation_invalidates_old_manifest_until_new_manifest_is_published() {
    let directory = temporary_directory("issue40-manifest-commit");
    let first = plan("v1");
    let second = plan("v2");
    write(&first, &directory).expect("initial generation should publish");
    let old_manifest = read(&directory, "clapgen.manifest.kdl");

    let error = write_with_hook(&second, &directory, |path| {
        if path == "clapgen_metadata.hpp" {
            return Err("injected failure before manifest publication".to_owned());
        }
        Ok(())
    })
    .expect_err("injected failure should abort publication");

    assert!(error.contains("injected failure"), "{error}");
    assert!(!directory.join("clapgen.manifest.kdl").exists(), "old manifest remained valid");
    assert_ne!(old_manifest, second.files[1].bytes, "test plans must use distinct manifest bytes");
    assert!(temp_entries(&directory).is_empty(), "temporary files leaked after failure");

    write(&second, &directory).expect("retry should publish the complete generation");
    for file in &second.files {
        assert_eq!(file.bytes, read(&directory, file.path));
    }

    fs::remove_dir_all(directory).expect("temporary directory should be removable");
}

#[test]
fn a_single_changed_output_republishes_manifest_last_but_leaves_other_files_untouched() {
    let directory = temporary_directory("issue40-targeted-change");
    let first = plan("stable");
    write(&first, &directory).expect("initial generation should publish");
    let before = first
        .files
        .iter()
        .map(|file| {
            (
                file.path,
                fs::metadata(directory.join(file.path))
                    .expect("metadata")
                    .modified()
                    .expect("modified time"),
            )
        })
        .collect::<Vec<_>>();

    let mut second = first.clone();
    let metadata = second
        .files
        .iter_mut()
        .find(|file| file.path == "clapgen_metadata.cpp")
        .expect("metadata source");
    metadata.bytes = b"changed metadata\n".to_vec();
    let mut order = Vec::new();
    write_with_hook(&second, &directory, |path| {
        order.push(path.to_owned());
        Ok(())
    })
    .expect("targeted change should publish");

    assert_eq!(order, ["clapgen_metadata.cpp", "clapgen.manifest.kdl"]);
    assert_eq!(b"changed metadata\n", read(&directory, "clapgen_metadata.cpp").as_slice());
    for (path, modified) in before {
        if matches!(path, "clapgen_metadata.cpp" | "clapgen.manifest.kdl") {
            continue;
        }
        assert_eq!(
            modified,
            fs::metadata(directory.join(path)).expect("metadata").modified().expect("modified time"),
            "unchanged output was rewritten: {path}"
        );
    }

    fs::remove_dir_all(directory).expect("temporary directory should be removable");
}

#[test]
fn writer_rejects_non_contract_paths_before_touching_the_destination() {
    let directory = temporary_directory("issue40-contract-validation");
    let invalid = GenerationPlan {
        files: vec![GeneratedFile { path: "../user-owned.cpp", bytes: b"do not write\n".to_vec() }],
    };

    let error = write(&invalid, &directory).expect_err("invalid output contract must fail");
    assert!(error.contains("generation output contract"), "{error}");
    assert!(!directory.exists(), "invalid plan must not create the destination");
}
