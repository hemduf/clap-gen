use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::dependency::{depfile_escape, normalize_path};
use super::{GenerationPlan, render};

const SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.codegen\" name=\"Codegen\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"CodegenProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";

fn ir_from_at(path: &Path, source: &str) -> crate::ir::CanonicalIr {
    let metadata = parse_metadata(path, source).expect("metadata should parse");
    build_ir(path, source, &metadata).expect("canonical IR should build")
}

fn build_file(path: &Path) -> crate::ir::CanonicalIr {
    let source = fs::read_to_string(path).expect("metadata should be readable");
    ir_from_at(path, &source)
}

fn generated_text<'a>(plan: &'a GenerationPlan, path: &str) -> &'a str {
    let file = plan.files.iter().find(|file| file.path == path).expect("generated file");
    std::str::from_utf8(&file.bytes).expect("generated files must be UTF-8")
}

fn relative_temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    PathBuf::from("target").join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

#[test]
fn manifest_and_depfile_match_golden_files() {
    let plan = render(&ir_from_at(Path::new("plugin.kdl"), SOURCE));
    let manifest = generated_text(&plan, "clapgen.manifest.kdl");
    let depfile = generated_text(&plan, "clapgen.d");

    assert_eq!(include_str!("../../tests/golden/issue39-manifest.kdl"), manifest);
    assert_eq!(include_str!("../../tests/golden/issue39-depfile.d"), depfile);
    kdl::KdlDocument::parse_v2(manifest).expect("generation manifest must remain valid KDL 2.0");
}

#[test]
fn depfile_and_manifest_include_transitive_metadata_and_owner_relative_resources() {
    let directory = relative_temporary_directory("issue39 dependency #$");
    let config = directory.join("config dir");
    let shared = directory.join("shared data");
    fs::create_dir_all(&config).expect("config directory");
    fs::create_dir_all(&shared).expect("shared directory");

    let imported = shared.join("common #$.kdl");
    fs::write(
        &imported,
        "clapgen schema=\"1.0.0\"\ngui {\n    resource \"../assets/my #panel$.svg\" mime=\"image/svg+xml\"\n}\n",
    )
    .expect("shared metadata");
    let root = config.join("plugin root.kdl");
    fs::write(
        &root,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.codegen\" name=\"Codegen\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"CodegenProcessor\"\nimport \"../shared data/common #$.kdl\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    )
    .expect("root metadata");

    let plan = render(&build_file(&root));
    let manifest = generated_text(&plan, "clapgen.manifest.kdl");
    let depfile = generated_text(&plan, "clapgen.d");
    let root_path = normalize_path(&root.to_string_lossy());
    let import_path = normalize_path(&imported.to_string_lossy());
    let resource_path = normalize_path(&directory.join("assets/my #panel$.svg").to_string_lossy());

    for dependency in [&resource_path, &root_path, &import_path] {
        assert!(manifest.contains(&format!("dependency \"{dependency}\"")), "{manifest}");
        assert!(depfile.contains(&depfile_escape(dependency)), "{depfile}");
    }
    assert!(depfile.starts_with("clapgen.manifest.kdl: "), "{depfile}");
    assert_eq!(1, depfile.lines().count(), "{depfile}");

    let resource_position = manifest.find(&format!("dependency \"{resource_path}\"")).unwrap();
    let root_position = manifest.find(&format!("dependency \"{root_path}\"")).unwrap();
    let import_position = manifest.find(&format!("dependency \"{import_path}\"")).unwrap();
    assert!(resource_position < root_position && root_position < import_position, "{manifest}");

    for forbidden in ["Generated at", "timestamp", "hostname", "pid=", ".tmp"] {
        assert!(!manifest.contains(forbidden), "{manifest}");
        assert!(!depfile.contains(forbidden), "{depfile}");
    }
    fs::remove_dir_all(directory).expect("temporary directory should be removable");
}

#[test]
fn portable_dependency_paths_normalize_windows_separators_and_escape_depfile_tokens() {
    let windows = r"C:\Program Files\Acme\config\..\plugin #$.kdl";
    let normalized = normalize_path(windows);
    assert_eq!("C:/Program Files/Acme/plugin #$.kdl", normalized);
    assert_eq!(r"C\:/Program\ Files/Acme/plugin\ \#$$.kdl", depfile_escape(&normalized));

    assert_eq!("plugin.kdl", normalize_path(r"config\..\plugin.kdl"));
    assert_eq!(r"dir/my\ \#file$$.kdl", depfile_escape("dir/my #file$.kdl"));
}
