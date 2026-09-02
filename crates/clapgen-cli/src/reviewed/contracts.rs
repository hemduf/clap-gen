use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::metadata::parse_metadata;

use super::build_ir;

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn relative_temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    PathBuf::from("target").join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn root_manifest(extra: &str) -> String {
    format!(
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.issue34\" name=\"Issue34\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"Issue34Processor\"\n{extra}"
    )
}

fn build_file(path: &Path) -> Result<super::CanonicalIr, String> {
    let source = fs::read_to_string(path).expect("metadata should be readable");
    let metadata = parse_metadata(path, &source)?;
    build_ir(path, &source, &metadata)
}

#[test]
fn typed_ir_is_available_without_serialization_or_reparsing() {
    let directory = temporary_directory("typed-ir");
    fs::create_dir_all(&directory).expect("directory");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "parameters { param \"Gain\" id=\"gain\" min=0 max=1 default=0.5 flags=\"automatable\" }\naudio-ports { input \"Main In\" id=\"main\" channels=2 flags=\"main\"; output \"Main Out\" id=\"main\" channels=2 flags=\"main\" }\nnote-ports {}\nstate { field \"mode\" type=\"integer\" default=0 tag=\"mode\" }\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.preset-load/2\" version=\"2\" }\n",
        ),
    )
    .expect("manifest");

    let ir = build_file(&manifest).expect("IR should build");
    assert_eq!("com.example.issue34", ir.plugin().id);
    assert_eq!("Issue34Processor", ir.processor().class);
    assert_eq!(1, ir.parameters().len());
    assert_eq!("gain", ir.parameters()[0].id);
    assert_eq!(2, ir.audio_ports().len());
    assert_eq!(1, ir.state_fields().len());
    assert_eq!(1, ir.stable_extension_items().len());
    assert_eq!("clap.preset-load/2", ir.stable_extension_items()[0].id);

    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn stable_core_extension_exposes_pinned_sdk_header() {
    let directory = temporary_directory("stable-core-header");
    fs::create_dir_all(&directory).expect("directory");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "parameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.params\" }\n",
        ),
    )
    .expect("manifest");

    let ir = build_file(&manifest).expect("IR should build");
    let params = ir
        .stable_extension_items()
        .iter()
        .find(|extension| extension.id == "clap.params")
        .expect("params extension");
    assert_eq!(Some("clap/ext/params.h"), params.header);

    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn normalized_extension_id_keeps_header_mapping() {
    let directory = temporary_directory("normalized-extension-header");
    fs::create_dir_all(&directory).expect("directory");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "parameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \" clap.params \" }\n",
        ),
    )
    .expect("manifest");

    let ir = build_file(&manifest).expect("IR should build");
    let params = ir.stable_extension_items().first().expect("params extension");
    assert_eq!("clap.params", params.id);
    assert_eq!(Some("clap/ext/params.h"), params.header);

    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn provenance_keys_use_canonical_parameter_ids() {
    let directory = temporary_directory("canonical-provenance-key");
    fs::create_dir_all(&directory).expect("directory");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "parameters { param \"Gain\" id=\" gain \" min=0 max=1 default=0.5 }\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
        ),
    )
    .expect("manifest");

    let ir = build_file(&manifest).expect("IR should build");
    assert_eq!("gain", ir.parameters()[0].id);
    assert!(ir.sources().iter().any(|source| source.key == "parameter:gain"));
    assert!(!ir.sources().iter().any(|source| source.key == "parameter: gain "));

    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn relative_root_path_is_preserved_in_dependencies_and_import_sources() {
    let directory = relative_temporary_directory("relative-root");
    let config = directory.join("config");
    let shared = directory.join("shared");
    fs::create_dir_all(&config).expect("config directory");
    fs::create_dir_all(&shared).expect("shared directory");
    fs::write(
        shared.join("common.kdl"),
        "clapgen schema=\"1.0.0\"\nparameters { param \"Shared\" id=\"shared\" min=0 max=1 default=0.5 }\n",
    )
    .expect("shared import");
    let manifest = config.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "import \"../shared/common.kdl\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
        ),
    )
    .expect("manifest");

    let ir = build_file(&manifest).expect("IR should build");
    let root = manifest.to_string_lossy().replace('\\', "/");
    let imported = shared.join("common.kdl").to_string_lossy().replace('\\', "/");
    assert_eq!(&[root.as_str(), imported.as_str()], ir.dependencies());
    let source =
        ir.sources().iter().find(|source| source.key == "parameter:shared").expect("shared source");
    assert_eq!(imported, source.path);

    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn transitive_imports_are_exposed_as_deterministic_dependencies() {
    let directory = temporary_directory("dependencies");
    fs::create_dir_all(directory.join("shared/nested")).expect("directory");
    fs::write(
        directory.join("shared/nested/params.kdl"),
        "clapgen schema=\"1.0.0\"\nparameters { param \"Shared\" id=\"shared\" min=0 max=1 default=0.5 }\n",
    )
    .expect("nested import");
    fs::write(
        directory.join("shared/common.kdl"),
        "clapgen schema=\"1.0.0\"\nimport \"nested/params.kdl\"\naudio-ports { output \"Aux\" id=\"aux\" channels=2 }\n",
    )
    .expect("import");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "import \"shared/common.kdl\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
        ),
    )
    .expect("manifest");

    let ir = build_file(&manifest).expect("IR should build");
    assert_eq!(&["plugin.kdl", "shared/common.kdl", "shared/nested/params.kdl"], ir.dependencies());

    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn imported_semantic_nodes_retain_parser_span_provenance() {
    let directory = temporary_directory("provenance");
    fs::create_dir_all(directory.join("shared")).expect("directory");
    fs::write(
        directory.join("shared/common.kdl"),
        "clapgen schema=\"1.0.0\"\nparameters {\n    param \"Shared\" id=\"shared\" min=0 max=1 default=0.5\n}\n",
    )
    .expect("import");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "import \"shared/common.kdl\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
        ),
    )
    .expect("manifest");

    let ir = build_file(&manifest).expect("IR should build");
    let source = ir
        .sources()
        .iter()
        .find(|entry| entry.key == "parameter:shared")
        .expect("imported parameter source");
    assert_eq!("shared/common.kdl", source.path);
    assert_eq!(3, source.line);
    assert!(source.column >= 1);

    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn unknown_official_versioned_stable_extension_is_rejected() {
    let directory = temporary_directory("unknown-stable-extension");
    fs::create_dir_all(&directory).expect("directory");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        root_manifest(
            "parameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.this-does-not-exist/99\" version=\"99\" }\n",
        ),
    )
    .expect("manifest");

    let error = build_file(&manifest).expect_err("unknown official stable extension must fail");
    assert!(error.contains("pinned CLAP SDK"), "{error}");
    assert!(error.contains("clap.this-does-not-exist/99"), "{error}");

    fs::remove_dir_all(directory).expect("cleanup");
}
