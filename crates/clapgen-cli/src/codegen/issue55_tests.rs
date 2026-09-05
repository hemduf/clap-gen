#[test]
fn issue55_registers_a_real_generated_validation_plugin_and_runner() {
    let cmake = include_str!("../../../../CMakeLists.txt");
    assert!(
        cmake.contains("tests/codegen/issue55/Issue55.cmake"),
        "issue55 validation plugin must be registered:\n{cmake}"
    );

    let ci = include_str!("../../../../.github/workflows/ci.yml");
    for required in [
        "tests/codegen/issue55/validator_backend.cpp",
        "tools/run_clap_validator.py",
        "clapgen_issue55_validation.clap",
    ] {
        assert!(ci.contains(required), "CI is missing `{required}`:\n{ci}");
    }
}

#[test]
fn issue55_pins_and_integrity_checks_the_validator_release() {
    let runner = include_str!("../../../../tools/run_clap_validator.py");
    for required in [
        "0.4.1",
        "49edadcfb407ea0dd946ce418300e853fbd2660fa4b0d00e4f19ff8eef24ad90",
        "bbec8cd7d18274e549d5d8c12ece3cec54be966129388dd2e742b9957f2ba9f1",
        "d935c3af0a45c3911ea2e900f4aa5d6709dac82bb485f0c4ce28648ab2cd0c10",
        "--only-failed",
        "validate",
    ] {
        assert!(runner.contains(required), "validator runner is missing `{required}`:\n{runner}");
    }
}

#[test]
fn issue55_sanitizers_cover_the_generated_runtime() {
    let ci = include_str!("../../../../.github/workflows/ci.yml");
    let sanitizer = ci.find("name: Sanitizers").expect("sanitizer job");
    let required_gate = ci.find("name: Required CI gate").expect("required gate");
    let sanitizer_job = &ci[sanitizer..required_gate];
    assert!(
        sanitizer_job.contains("-DCLAPGEN_FETCH_CLAP=ON"),
        "sanitizers must build/test generated runtime fixtures:\n{sanitizer_job}"
    );
}

#[test]
fn issue55_documents_reproducible_local_validation_commands() {
    let docs = include_str!("../../../../docs/validation.md");
    for required in [
        "cmake -S . -B build/validation",
        "CLAPGEN_FETCH_CLAP=ON",
        "clapgen_issue55_validation_plugin",
        "tools/run_clap_validator.py",
    ] {
        assert!(docs.contains(required), "validation docs are missing `{required}`:\n{docs}");
    }
}
