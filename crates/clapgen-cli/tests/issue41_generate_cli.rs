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

fn write_project(root: &Path) -> (PathBuf, Vec<PathBuf>) {
    let source = root.join("source tree");
    let shared = source.join("shared");
    let assets = source.join("assets");
    let processor = source.join("src");
    fs::create_dir_all(&shared).expect("shared directory");
    fs::create_dir_all(&assets).expect("asset directory");
    fs::create_dir_all(&processor).expect("processor directory");

    let imported = shared.join("common.kdl");
    fs::write(
        &imported,
        "clapgen schema=\"1.0.0\"\nparameters {\n    param \"Gain\" id=\"gain\" min=0 max=1 default=0.5\n}\ngui {\n    resource \"../assets/panel.svg\" mime=\"image/svg+xml\"\n}\n",
    )
    .expect("imported metadata");
    let resource = assets.join("panel.svg");
    fs::write(&resource, [0_u8, 0xff, b'<', b's', b'v', b'g', b'>']).expect("resource");
    let processor_source = processor.join("CodegenProcessor.cpp");
    fs::write(&processor_source, b"// user-owned DSP source\n").expect("processor source");
    let manifest = source.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.cli\" name=\"CLI\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"CodegenProcessor\"\nimport \"shared/common.kdl\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    )
    .expect("root metadata");

    (manifest, vec![imported, resource, processor_source])
}

fn snapshot(paths: &[PathBuf]) -> Vec<Vec<u8>> {
    paths.iter().map(|path| fs::read(path).expect("source file should be readable")).collect()
}

fn output_mtimes(directory: &Path) -> Vec<SystemTime> {
    OUTPUTS
        .iter()
        .map(|name| {
            fs::metadata(directory.join(name))
                .expect("generated output metadata")
                .modified()
                .expect("generated output modified time")
        })
        .collect()
}

#[test]
fn generate_writes_only_to_explicit_out_and_preserves_user_owned_files_and_mtimes() {
    let root = temporary_directory("issue41-source-boundary");
    fs::create_dir_all(&root).expect("temporary root");
    let (manifest, mut user_files) = write_project(&root);
    user_files.insert(0, manifest.clone());
    let before = snapshot(&user_files);
    let out = root.join("build tree/generated output");

    let first = clapgen()
        .args(["generate", "--metadata"])
        .arg(&manifest)
        .arg("--out")
        .arg(&out)
        .output()
        .expect("clapgen generate should run");
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));

    for name in OUTPUTS {
        assert!(out.join(name).is_file(), "missing generated output: {name}");
        assert!(!manifest.parent().expect("source directory").join(name).exists());
    }
    assert_eq!(before, snapshot(&user_files), "user-owned files changed after generation");
    let first_mtimes = output_mtimes(&out);

    let second = clapgen()
        .args(["generate", "--metadata"])
        .arg(&manifest)
        .arg("--out")
        .arg(&out)
        .output()
        .expect("clapgen generate should run twice");
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));
    assert_eq!(first_mtimes, output_mtimes(&out), "no-op CLI generation changed mtimes");
    assert_eq!(before, snapshot(&user_files), "user-owned files changed after no-op generation");

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}

#[test]
fn generate_accepts_only_the_documented_argument_shape() {
    let root = temporary_directory("issue41-cli-shape");
    fs::create_dir_all(&root).expect("temporary root");
    let (manifest, _) = write_project(&root);
    let out = root.join("build");

    for arguments in [
        vec![
            "generate".to_owned(),
            "--metadata".to_owned(),
            manifest.to_string_lossy().into_owned(),
        ],
        vec![
            "generate".to_owned(),
            "--out".to_owned(),
            out.to_string_lossy().into_owned(),
            "--metadata".to_owned(),
            manifest.to_string_lossy().into_owned(),
        ],
        vec![
            "generate".to_owned(),
            "--metadata".to_owned(),
            manifest.to_string_lossy().into_owned(),
            "--out".to_owned(),
            out.to_string_lossy().into_owned(),
            "extra".to_owned(),
        ],
    ] {
        let output = clapgen().args(arguments).output().expect("clapgen should run");
        assert!(!output.status.success(), "invalid argument shape unexpectedly succeeded");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unknown command or arguments"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let help = clapgen().arg("help").output().expect("help should run");
    assert!(help.status.success());
    assert!(
        String::from_utf8_lossy(&help.stdout)
            .contains("generate --metadata <file> --out <build-dir>")
    );

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}

#[test]
fn generate_preserves_author_location_in_semantic_validation_diagnostics() {
    let root = temporary_directory("issue41-diagnostics");
    fs::create_dir_all(&root).expect("temporary root");
    let manifest = root.join("invalid.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.invalid\" name=\"Invalid\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"InvalidProcessor\"\nparameters {\n    param \"Gain\" id=\"gain\" min=0 max=1 default=2\n}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    )
    .expect("invalid metadata");
    let out = root.join("build");

    let output = clapgen()
        .args(["generate", "--metadata"])
        .arg(&manifest)
        .arg("--out")
        .arg(&out)
        .output()
        .expect("clapgen generate should run");
    assert!(!output.status.success(), "invalid semantic metadata unexpectedly generated");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&format!("{}:5:", manifest.display())), "{stderr}");
    assert!(stderr.contains("parameter `gain` has invalid range/default"), "{stderr}");
    assert!(!out.exists(), "semantic validation failure created the output directory");

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}
