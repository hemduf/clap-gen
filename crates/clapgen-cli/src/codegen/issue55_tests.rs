#[test]
fn issue55_qualification_is_registered_in_build_and_ci() {
    let cmake = include_str!("../../../../CMakeLists.txt");
    assert!(
        cmake.contains("tests/codegen/issue55/Issue55.cmake"),
        "#55 qualification CMake module must be registered:\n{cmake}"
    );

    let ci = include_str!("../../../../.github/workflows/ci.yml");
    for required in [
        "CLAP_VALIDATOR_REV:",
        "clap-validator validate",
        "clapgen_issue55_minimal.clap",
        "validator-${{ runner.os }}-${{ matrix.build_type }}",
    ] {
        assert!(ci.contains(required), "missing #55 CI contract `{required}`:\n{ci}");
    }
}

#[test]
fn issue55_local_validator_reproduction_is_documented_and_pinned() {
    let qualification = include_str!("../../../../tests/codegen/issue55/README.md");
    for required in [
        "clap-validator validate",
        "CLAP_VALIDATOR_REV",
        "cmake -S . -B build/issue55",
        "cmake --build build/issue55",
        "ctest --test-dir build/issue55",
    ] {
        assert!(
            qualification.contains(required),
            "missing reproducible #55 instruction `{required}`:\n{qualification}"
        );
    }
}

#[test]
fn issue55_minimal_module_uses_the_generated_native_runtime() {
    let cmake = include_str!("../../../../tests/codegen/issue55/Issue55.cmake");
    for required in [
        "clapgen_issue55_minimal",
        "clapgen_entry.cpp",
        "clapgen_instance_backend.hpp",
        "minimal_backend.cpp",
        "SUFFIX \".clap\"",
        "WINDOWS_EXPORT_ALL_SYMBOLS OFF",
    ] {
        assert!(cmake.contains(required), "missing minimal module contract `{required}`:\n{cmake}");
    }

    let backend = include_str!("../../../../tests/codegen/issue55/minimal_backend.cpp");
    for required in [
        "create_plugin_instance_for<MinimalProcessor>",
        "clap_process_status process(const clap_process_t* process)",
        "CLAP_PROCESS_CONTINUE",
    ] {
        assert!(backend.contains(required), "missing native runtime use `{required}`:\n{backend}");
    }
    for forbidden in ["ProcessBlock", "ProcessStatus", "ParamEvent", "AudioConfigId", "RenderMode"] {
        assert!(!backend.contains(forbidden), "public ABI mirror `{forbidden}` leaked into fixture");
    }
}
