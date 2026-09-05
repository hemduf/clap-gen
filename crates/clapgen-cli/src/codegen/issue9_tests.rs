use std::path::Path;

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::render_for_output;

#[test]
fn issue9_exports_an_installable_cmake_package() {
    let package = include_str!("../../../../cmake/ClapGenPackage.cmake");
    let config = include_str!("../../../../cmake/ClapGenConfig.cmake.in");

    assert!(package.contains("ClapGenConfig.cmake.in"));
    assert!(package.contains("ClapGenConfigVersion.cmake"));
    assert!(package.contains("ClapGenTargets"));
    assert!(package.contains("ClapGenFunctions.cmake"));
    assert!(package.contains("EXPORT_NAME Runtime"));
    assert!(config.contains("ClapGenTargets.cmake"));
    assert!(config.contains("ClapGenFunctions.cmake"));
}

#[test]
fn issue9_registers_consumer_and_incremental_integration_tests() {
    let package = include_str!("../../../../cmake/ClapGenPackage.cmake");
    let tests = include_str!("../../../../tests/cmake/issue9/Issue9.cmake");

    assert!(package.contains("tests/cmake/issue9/Issue9.cmake"));
    assert!(tests.contains("consumer incremental cross"));
    assert!(tests.contains("clapgen.cmake.issue9.${_clapgen_issue9_mode}"));
    assert!(tests.contains("clapgen.cmake.issue9.real-codegen"));
}

#[test]
fn issue9_cmake_depfile_uses_physical_target_and_dependency_paths() {
    let source = concat!(
        "clapgen schema=\"1.0.0\"\n",
        "plugin id=\"com.example.issue9\" name=\"Issue9\" ",
        "vendor=\"Example\" version=\"1.0.0\"\n",
        "processor class=\"Issue9Processor\"\n",
        "parameters {}\n",
        "audio-ports {}\n",
        "note-ports {}\n",
        "state {}\n",
        "gui {}\n",
        "presets {}\n",
        "factories {}\n",
        "extensions {}\n",
    );
    let metadata_path = Path::new("/checkout/project/plugin.kdl");
    let metadata = parse_metadata(metadata_path, source).expect("metadata should parse");
    let ir = build_ir(metadata_path, source, &metadata).expect("IR should build");
    let plan = render_for_output(
        &ir,
        Path::new("/checkout/project"),
        Path::new("/work/build/clapgen/plugin"),
    );
    let depfile =
        plan.files.iter().find(|file| file.path == "clapgen.d").expect("depfile").bytes.as_slice();
    let depfile = std::str::from_utf8(depfile).expect("depfile should be UTF-8");

    assert!(depfile.starts_with("/work/build/clapgen/plugin/clapgen.manifest.kdl:"));
    assert!(depfile.contains("/checkout/project/plugin.kdl"));
}
