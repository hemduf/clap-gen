use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn run(directory: &Path, manifest: &str, arguments: &[&str]) -> Output {
    fs::create_dir_all(directory).expect("temporary directory should be created");
    let path = directory.join("plugin.kdl");
    fs::write(&path, manifest).expect("manifest should be written");

    let mut command = Command::new(env!("CARGO_BIN_EXE_clapgen"));
    command.args(arguments).arg(&path);
    command.output().expect("clapgen should run")
}

fn base_manifest(body: &str) -> String {
    format!(
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.review\" name=\"Review\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"ReviewProcessor\"\n{body}"
    )
}

#[test]
fn stable_versioned_extension_from_pinned_sdk_is_not_misclassified_as_draft() {
    let directory = temporary_directory("stable-versioned-extension");
    let manifest = base_manifest(
        "parameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.preset-load/2\" version=\"2\" }\n",
    );
    let output = run(&directory, &manifest, &["validate"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn unknown_draft_extension_is_rejected_against_the_pinned_sdk_contract() {
    let directory = temporary_directory("unknown-draft-extension");
    let manifest = base_manifest(
        "parameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.this-does-not-exist/99\" version=\"99\" draft=#true }\n",
    );
    let output = run(&directory, &manifest, &["validate"]);
    assert!(!output.status.success(), "unknown draft ABI must fail validation");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("pinned CLAP SDK"), "{stderr}");
    assert!(stderr.contains("clap.this-does-not-exist/99"), "{stderr}");
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn audio_port_ids_may_overlap_across_directions_and_in_place_pair_targets_the_opposite_side() {
    let directory = temporary_directory("audio-port-directional-ids");
    let manifest = base_manifest(
        "parameters {}\naudio-ports {\n    input \"Main In\" id=\"main\" channels=2 flags=\"main\"\n    output \"Main Out\" id=\"main\" channels=2 flags=\"main\" in-place-pair=\"main\"\n}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    );
    let output = run(&directory, &manifest, &["validate"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn note_port_ids_may_overlap_across_directions() {
    let directory = temporary_directory("note-port-directional-ids");
    let manifest = base_manifest(
        "parameters {}\naudio-ports {}\nnote-ports {\n    input \"Notes In\" id=\"notes\" dialects=\"clap\" preferred=\"clap\"\n    output \"Notes Out\" id=\"notes\" dialects=\"clap\" preferred=\"clap\"\n}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    );
    let output = run(&directory, &manifest, &["validate"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn bypass_parameter_requires_stepped_and_is_unique() {
    let directory = temporary_directory("bypass-rules");
    let missing_stepped = base_manifest(
        "parameters { param \"Bypass\" id=\"bypass\" min=0 max=1 default=0 flags=\"bypass\" }\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    );
    let output = run(&directory, &missing_stepped, &["validate"]);
    assert!(!output.status.success(), "bypass without stepped must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("stepped"));

    let duplicate = base_manifest(
        "parameters {\n    param \"Bypass A\" id=\"bypass-a\" min=0 max=1 default=0 flags=\"bypass,stepped\"\n    param \"Bypass B\" id=\"bypass-b\" min=0 max=1 default=0 flags=\"bypass,stepped\"\n}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    );
    let output = run(&directory, &duplicate, &["validate"]);
    assert!(!output.status.success(), "multiple bypass parameters must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("only one bypass"));
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn enum_parameter_requires_stepped() {
    let directory = temporary_directory("enum-rules");
    let manifest = base_manifest(
        "parameters { param \"Mode\" id=\"mode\" min=0 max=2 default=0 flags=\"enum\" }\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    );
    let output = run(&directory, &manifest, &["validate"]);
    assert!(!output.status.success(), "enum without stepped must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("stepped"));
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn audio_main_port_is_unique_per_direction_and_serializes_at_index_zero() {
    let directory = temporary_directory("main-port-rules");
    let duplicate_main = base_manifest(
        "parameters {}\naudio-ports {\n    input \"Main A\" id=\"a\" channels=2 flags=\"main\"\n    input \"Main B\" id=\"b\" channels=2 flags=\"main\"\n}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    );
    let output = run(&directory, &duplicate_main, &["validate"]);
    assert!(!output.status.success(), "multiple main inputs must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("main"));

    let ordered = base_manifest(
        "parameters {}\naudio-ports {\n    input \"Aux\" id=\"a-aux\" channels=2\n    input \"Main\" id=\"z-main\" channels=2 flags=\"main\"\n}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    );
    let output = run(&directory, &ordered, &["inspect", "--format", "kdl"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let main = stdout.find("input id=\"z-main\"").expect("main port should be serialized");
    let aux = stdout.find("input id=\"a-aux\"").expect("aux port should be serialized");
    assert!(main < aux, "main port must occupy index zero:\n{stdout}");
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn imports_are_semantically_merged_into_the_canonical_ir() {
    let directory = temporary_directory("semantic-imports");
    fs::create_dir_all(directory.join("shared")).expect("shared directory should be created");
    fs::write(
        directory.join("shared/common.kdl"),
        "clapgen schema=\"1.0.0\"\nparameters { param \"Shared\" id=\"shared\" min=0 max=1 default=0.5 flags=\"automatable\" }\n",
    )
    .expect("import should be written");

    let manifest = "clapgen schema=\"1.0.0\"\nimport \"shared/common.kdl\"\nplugin id=\"com.example.review\" name=\"Review\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"ReviewProcessor\"\nparameters { param \"Local\" id=\"local\" min=0 max=1 default=0.5 }\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n".to_string();
    let output = run(&directory, &manifest, &["inspect", "--format", "kdl"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("param id=\"local\""), "{stdout}");
    assert!(stdout.contains("param id=\"shared\""), "{stdout}");
    assert!(stdout.contains("parameters count=2"), "{stdout}");
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}
