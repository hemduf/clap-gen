#[test]
fn issue9_exports_an_installable_cmake_package() {
    let root = include_str!("../../../../CMakeLists.txt");
    for required in [
        "ClapGenConfig.cmake.in",
        "ClapGenConfigVersion.cmake",
        "ClapGenTargets",
        "ClapGenFunctions.cmake",
        "EXPORT_NAME Runtime",
    ] {
        assert!(
            root.contains(required),
            "root CMake must install/export the first-class ClapGen package; missing `{required}`:\n{root}"
        );
    }
}

#[test]
fn issue9_registers_consumer_and_incremental_integration_tests() {
    let root = include_str!("../../../../CMakeLists.txt");
    for required in [
        "tests/cmake/issue9/Issue9.cmake",
        "clapgen.cmake.issue9.consumer",
        "clapgen.cmake.issue9.incremental",
        "clapgen.cmake.issue9.cross",
    ] {
        assert!(
            root.contains(required),
            "#9 must exercise installed consumers, no-op/depfile regeneration and cross-host tools; missing `{required}`:\n{root}"
        );
    }
}
