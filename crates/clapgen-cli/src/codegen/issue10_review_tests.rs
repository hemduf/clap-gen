use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::render;

const VALID_SOURCE: &str = r#"clapgen schema="1.0.0"
plugin id="com.example.issue10.review" name="Issue10 Review" vendor="Example" version="1.0.0"
processor class="Issue10ReviewProcessor"
parameters {
    param id="gain" name="Gain" min=0.0 max=2.0 default=1.0 flags="automatable,modulatable"
    param id="mode" name="Mode" min=0.0 max=2.0 default=0.0 flags="automatable,stepped,enum" steps=3
    param id="bypass" name="Bypass" min=0.0 max=1.0 default=0.0 flags="automatable,stepped,bypass" steps=2
}
audio-ports {}
note-ports {}
state { field "seed" type="u32" default="7" tag="seed" }
gui {}
presets {}
factories {}
extensions { enable "clap.params" enable "clap.state" }
"#;

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("clapgen-issue10-review-{}-{nonce}", std::process::id()))
}

fn build(source: &str) -> Result<crate::ir::CanonicalIr, String> {
    let directory = temporary_directory();
    fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("plugin.kdl");
    fs::write(&path, source).expect("manifest");
    fs::write(
        directory.join("plugin.ids.kdl"),
        "ids version=1 next=5 {\n    entry kind=\"parameter\" key=\"gain\" value=1 tombstone=#false\n    entry kind=\"parameter\" key=\"mode\" value=2 tombstone=#false\n    entry kind=\"parameter\" key=\"bypass\" value=3 tombstone=#false\n    entry kind=\"state-field\" key=\"seed\" value=4 tombstone=#false\n}\n",
    )
    .expect("registry");
    let metadata = parse_metadata(&path, source)?;
    let result = build_ir(&path, source, &metadata).and_then(|ir| {
        super::render::validate_runtime_ids(&ir)?;
        Ok(ir)
    });
    fs::remove_dir_all(directory).expect("temporary directory cleanup");
    result
}

fn generated_text<'a>(plan: &'a super::GenerationPlan, path: &str) -> &'a str {
    let file = plan.files.iter().find(|file| file.path == path).expect("generated file");
    std::str::from_utf8(&file.bytes).expect("generated UTF-8")
}

#[test]
fn issue10_shared_parameter_snapshot_is_explicitly_lock_free() {
    let ir = build(VALID_SOURCE).expect("valid issue10 metadata");
    let plan = render(&ir);
    let extensions = generated_text(&plan, "clapgen_extensions.hpp");

    for required in [
        "#include <atomic>",
        "std::atomic<std::uint64_t>",
        "is_always_lock_free",
        "load_parameter_value",
        "store_parameter_value",
        "GeneratedParameterValues",
    ] {
        assert!(
            extensions.contains(required),
            "missing realtime parameter snapshot guard `{required}`:\n{extensions}"
        );
    }
    assert!(
        !extensions.contains("std::array<double, generated_parameter_specs.size()>"),
        "plain shared double storage reintroduced across main/audio threads:\n{extensions}"
    );
}

#[test]
fn issue10_post_commit_state_hook_is_noexcept_by_contract() {
    let ir = build(VALID_SOURCE).expect("valid issue10 metadata");
    let plan = render(&ir);
    let processor = generated_text(&plan, "clapgen_processor.hpp");

    assert!(processor.contains("StateLoadedHookSafe"), "missing post-load hook safety concept:\n{processor}");
    assert!(
        processor.contains("processor.on_state_loaded() } noexcept"),
        "state load may report failure after committing state if the post-commit hook can throw:\n{processor}"
    );
}

#[test]
fn issue10_enum_requires_stepped_flag() {
    let source = VALID_SOURCE.replace(
        "flags=\"automatable,stepped,enum\" steps=3",
        "flags=\"automatable,enum\" steps=3",
    );
    let error = build(&source).expect_err("enum without stepped must fail metadata validation");
    assert!(error.contains("enum") && error.contains("stepped"), "{error}");
}

#[test]
fn issue10_bypass_requires_native_zero_one_stepped_domain() {
    for source in [
        VALID_SOURCE.replace(
            "min=0.0 max=1.0 default=0.0 flags=\"automatable,stepped,bypass\" steps=2",
            "min=0.0 max=2.0 default=0.0 flags=\"automatable,stepped,bypass\" steps=3",
        ),
        VALID_SOURCE.replace(
            "flags=\"automatable,stepped,bypass\" steps=2",
            "flags=\"automatable,bypass\" steps=2",
        ),
    ] {
        let error = build(&source).expect_err("invalid bypass metadata must fail");
        assert!(error.contains("bypass"), "{error}");
    }
}

#[test]
fn issue10_stepped_parameters_use_integer_plain_values() {
    let source = VALID_SOURCE.replace(
        "min=0.0 max=2.0 default=0.0 flags=\"automatable,stepped,enum\" steps=3",
        "min=0.0 max=2.5 default=0.5 flags=\"automatable,stepped,enum\" steps=3",
    );
    let error = build(&source).expect_err("stepped parameter with non-integer plain values must fail");
    assert!(error.contains("stepped") && error.contains("integer"), "{error}");
}
