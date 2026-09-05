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
