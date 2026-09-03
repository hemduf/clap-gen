use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const OUTPUTS: &[&str] = &[
    "clapgen.d",
    "clapgen.manifest.kdl",
    "clapgen.sources.kdl",
    "clapgen_metadata.cpp",
    "clapgen_metadata.hpp",
    "clapgen_resources.hpp",
];

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn clapgen() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clapgen"))
}

fn write_project(root: &Path, extension: bool) -> (PathBuf, PathBuf) {
    let source = root.join("source tree");
    let shared = source.join("shared metadata");
    let assets = source.join("assets");
    fs::create_dir_all(&shared).expect("shared directory");
    fs::create_dir_all(&assets).expect("asset directory");
    fs::write(assets.join("panel.svg"), b"<svg/>\n").expect("resource");
    fs::write(
        shared.join("common.kdl"),
        "clapgen schema=\"1.0.0\"\nparameters {\n    param \"Gain\" id=\"gain\" min=0 max=1 default=0.5\n}\ngui {\n    resource \"../assets/panel.svg\" mime=\"image/svg+xml\"\n}\n",
    )
    .expect("imported metadata");
    let manifest = source.join("plugin.kdl");
    let extensions =
        if extension { "extensions { enable \"clap.params\" }" } else { "extensions {}" };
    fs::write(
        &manifest,
        format!(
            "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.qualification\" name=\"Qualification\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"QualificationProcessor\"\nimport \"shared metadata/common.kdl\"\nparameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\n{extensions}\n"
        ),
    )
    .expect("root metadata");
    (manifest, source)
}

fn generate(metadata: &Path, out: &Path) {
    let output = clapgen()
        .args(["generate", "--metadata"])
        .arg(metadata)
        .arg("--out")
        .arg(out)
        .output()
        .expect("clapgen generate should run");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

fn snapshot(out: &Path) -> Vec<(&'static str, Vec<u8>)> {
    OUTPUTS
        .iter()
        .copied()
        .map(|name| (name, fs::read(out.join(name)).expect("generated output should be readable")))
        .collect()
}

#[test]
fn absolute_metadata_paths_produce_build_relative_depfiles_without_leaking_machine_paths() {
    let root = temporary_directory("issue42-absolute path");
    fs::create_dir_all(&root).expect("temporary root");
    let (manifest, source) = write_project(&root, false);
    let out = root.join("build tree/deep/generated");
    generate(&manifest, &out);

    let depfile = fs::read_to_string(out.join("clapgen.d")).expect("depfile");
    let generation_manifest =
        fs::read_to_string(out.join("clapgen.manifest.kdl")).expect("generation manifest");
    let source_map = fs::read_to_string(out.join("clapgen.sources.kdl")).expect("source map");
    let root_text = root.to_string_lossy().replace('\\', "/");

    assert!(depfile.contains("../../../source\\ tree/plugin.kdl"), "{depfile}");
    assert!(depfile.contains("../../../source\\ tree/shared\\ metadata/common.kdl"), "{depfile}");
    assert!(depfile.contains("../../../source\\ tree/assets/panel.svg"), "{depfile}");
    assert!(!depfile.replace('\\', "/").contains(&root_text), "absolute path leaked: {depfile}");

    assert!(generation_manifest.contains("dependency \"plugin.kdl\""), "{generation_manifest}");
    assert!(
        generation_manifest.contains("dependency \"shared metadata/common.kdl\""),
        "{generation_manifest}"
    );
    assert!(
        generation_manifest.contains("dependency \"assets/panel.svg\""),
        "{generation_manifest}"
    );
    assert!(!generation_manifest.contains(&root_text), "{generation_manifest}");
    assert!(!source_map.contains(&root_text), "{source_map}");
    assert_eq!(source, manifest.parent().expect("source directory"));

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}

#[test]
fn repeated_generation_is_byte_identical_and_capability_changes_are_narrowly_scoped() {
    let root = temporary_directory("issue42-determinism");
    fs::create_dir_all(&root).expect("temporary root");
    let (manifest, _) = write_project(&root, false);
    let out = root.join("build/generated");

    generate(&manifest, &out);
    let first = snapshot(&out);
    generate(&manifest, &out);
    assert_eq!(first, snapshot(&out), "repeated generation changed output bytes");

    let with_params = fs::read_to_string(&manifest)
        .expect("manifest")
        .replace("extensions {}", "extensions { enable \"clap.params\" }");
    fs::write(&manifest, with_params).expect("manifest update");
    generate(&manifest, &out);
    let second = snapshot(&out);
    let changed = first
        .iter()
        .zip(&second)
        .filter_map(|((name, before), (_, after))| (before != after).then_some(*name))
        .collect::<Vec<_>>();

    assert_eq!(changed, ["clapgen.sources.kdl", "clapgen_metadata.cpp", "clapgen_metadata.hpp"]);
    for (name, bytes) in &second {
        let text = String::from_utf8_lossy(bytes);
        for forbidden in ["Generated at", "timestamp", "hostname", "pid=", ".clapgen-"] {
            assert!(!text.contains(forbidden), "nondeterministic data in {name}: {text}");
        }
    }

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}
