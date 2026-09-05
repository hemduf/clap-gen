use std::path::Path;

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::render_for_output;

#[test]
fn issue9_exports_an_installable_cmake_package() {
    let package = include_str!("../../../../cmake/ClapGenPackage.cmake");
    let config = include_str!("../../../../cmake/ClapGenConfig.cmake.in");
    for required in [
        "ClapGenConfig.cmake.in",
        "ClapGenConfigVersion.cmake",
        "ClapGenTargets",
        "ClapGenFunctions.cmake",
        "EXPORT_NAME Runtime",
    ] {
        assert!(
            package.contains(required) || config.contains(required),
            "installable ClapGen package is missing `{required}`"
        );
    }
}

#[test]
fn issue9_registers_consumer_and_incremental_integration_tests() {
    let package = include_str!("../../../../cmake/ClapGenPackage.cmake");
    let tests = include_str!("../../../../tests/cmake/issue9/Issue9.cmake");
    for required in [
        "tests/cmake/issue9/Issue9.cmake",
        "clapgen.cmake.issue9.consumer",
        "clapgen.cmake.issue9.incremental",
        "clapgen.cmake.issue9.cross",
    ] {
        assert!(
            package.contains(required) || tests.contains(required),
            "#9 integration gates are missing `{required}`"
        );
    }
}

#[test]
fn issue9_cmake_depfile_uses_physical_target_and_dependency_paths() {
    let source = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.issue9\" name=\"Issue9\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"Issue9Processor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";
    let metadata_path = Path::new("/checkout/project/plugin.kdl");
    let metadata = parse_metadata(metadata_path, source).expect("metadata should parse");
    let ir = build_ir(metadata_path, source, &metadata).expect("IR should build");
    let plan = render_for_output(
        &ir,
        Path::new("/checkout/project"),
        Path::new("/work/build/clapgen/plugin"),
    );
    let depfile = plan
        .files
        .iter()
        .find(|file| file.path == "clapgen.d")
        .expect("depfile")
        .bytes
        .as_slice();
    let depfile = std::str::from_utf8(depfile).expect("depfile should be UTF-8");

    assert!(
        depfile.starts_with("/work/build/clapgen/plugin/clapgen.manifest.kdl:"),
        "CMake must see the exact custom-command output as the depfile target: {depfile}"
    );
    assert!(
        depfile.contains("/checkout/project/plugin.kdl"),
        "CMake must resolve imported/resource dependencies independently of its binary dir: {depfile}"
    );
}
