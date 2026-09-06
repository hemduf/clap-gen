use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::{GenerationPlan, render};

const SOURCE: &str = r#"clapgen schema="1.0.0"
plugin id="com.example.issue10" name="Issue10" vendor="Example" version="1.0.0"
processor class="Issue10Processor"
parameters {
    param id="gain" name="Gain" min=0.0 max=2.0 default=1.0 flags="automatable,modulatable" unit="x"
    param id="mode" name="Mode" min=0.0 max=2.0 default=0.0 flags="automatable,stepped,enum" steps=3
}
audio-ports {}
note-ports {}
state {
    field "seed" type="u32" default="7" tag="seed"
}
gui {}
presets {}
factories {}
extensions {
    enable "clap.params"
    enable "clap.state"
}
"#;

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("clapgen-issue10-{}-{nonce}", std::process::id()))
}

fn render_source() -> GenerationPlan {
    let directory = temporary_directory();
    fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("plugin.kdl");
    fs::write(&path, SOURCE).expect("manifest");
    fs::write(
        directory.join("plugin.ids.kdl"),
        "ids version=1 next=3 {\n    entry kind=\"parameter\" key=\"gain\" value=1 tombstone=#false\n    entry kind=\"parameter\" key=\"mode\" value=2 tombstone=#false\n}\n",
    )
    .expect("registry");

    let metadata = parse_metadata(&path, SOURCE).expect("metadata should parse");
    let ir = build_ir(&path, SOURCE, &metadata).expect("canonical IR should build");
    let plan = render(&ir);
    fs::remove_dir_all(directory).expect("temporary directory cleanup");
    plan
}

fn generated_text<'a>(plan: &'a GenerationPlan, path: &str) -> &'a str {
    let file = plan.files.iter().find(|file| file.path == path).expect("generated file");
    std::str::from_utf8(&file.bytes).expect("generated file must be UTF-8")
}

#[test]
fn issue10_generates_native_parameter_metadata_and_extension_table() {
    let plan = render_source();
    let extensions = generated_text(&plan, "clapgen_extensions.hpp");

    for required in [
        "#include <clap/ext/params.h>",
        "struct GeneratedParameterSpec",
        "inline constexpr std::array<GeneratedParameterSpec, 2> generated_parameter_specs",
        "clap_id{1u}",
        "clap_id{2u}",
        "CLAP_PARAM_IS_AUTOMATABLE",
        "CLAP_PARAM_IS_MODULATABLE",
        "CLAP_PARAM_IS_STEPPED",
        "CLAP_PARAM_IS_ENUM",
    ] {
        assert!(extensions.contains(required), "missing `{required}`:\n{extensions}");
    }

    for forbidden in ["ParamEvent", "ParameterEvent", "ParamInfo", "ProcessBlock"] {
        assert!(
            !extensions.contains(forbidden),
            "public ABI mirror `{forbidden}` leaked:\n{extensions}"
        );
    }
}

#[test]
fn issue10_runtime_exposes_native_params_and_state_callbacks_without_audio_thread_containers() {
    let plan = render_source();
    let backend = generated_text(&plan, "clapgen_instance_backend.hpp");

    for required in [
        "clap_plugin_params_t",
        "CLAP_EXT_PARAMS",
        "params_count_plugin",
        "params_get_info_plugin",
        "params_get_value_plugin",
        "params_value_to_text_plugin",
        "params_text_to_value_plugin",
        "params_flush_plugin",
        "clap_plugin_state_t",
        "CLAP_EXT_STATE",
        "state_save_plugin",
        "state_load_plugin",
        "clap_ostream_t",
        "clap_istream_t",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }

    for forbidden in ["std::vector", "std::map", "std::unordered_map", "std::mutex", "new Param"] {
        assert!(
            !backend.contains(forbidden),
            "realtime-sensitive runtime uses `{forbidden}`:\n{backend}"
        );
    }
}

#[test]
fn issue10_routes_native_param_events_and_keeps_modulation_separate_from_base_values() {
    let plan = render_source();
    let backend = generated_text(&plan, "clapgen_instance_backend.hpp");

    for required in [
        "CLAP_EVENT_PARAM_VALUE",
        "CLAP_EVENT_PARAM_MOD",
        "clap_event_param_value_t",
        "clap_event_param_mod_t",
        "header->time",
        "process->frames_count",
        "on_parameter_event",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }

    assert!(
        backend.contains("parameter_values_[parameter_index] = value_event->value")
            || backend.contains(
                "parameter_values_[static_cast<std::size_t>(parameter_index)] = value_event->value"
            ),
        "automation must update the base value:\n{backend}"
    );
    assert!(
        !backend.contains("parameter_values_[parameter_index] = mod_event->amount"),
        "modulation must not overwrite the base parameter value:\n{backend}"
    );
}

#[test]
fn issue10_state_format_is_tagged_bounded_versioned_and_transactional() {
    let plan = render_source();
    let backend = generated_text(&plan, "clapgen_instance_backend.hpp");

    for required in [
        "kStateMagic",
        "kStateSchemaVersion",
        "kMaxStateBytes",
        "StateRecordHeader",
        "StateRecordKind::Parameter",
        "StateRecordKind::Field",
        "candidate_parameter_values",
        "commit_loaded_state",
        "unknown",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }
}

#[test]
fn issue10_state_wire_is_explicit_little_endian_and_not_native_object_layout() {
    let plan = render_source();
    let backend = generated_text(&plan, "clapgen_instance_backend.hpp");

    for required in [
        "write_u16_le",
        "write_u32_le",
        "write_u64_le",
        "read_u16_le",
        "read_u32_le",
        "read_u64_le",
        "std::bit_cast<std::uint64_t>",
        "std::bit_cast<double>",
        "kStateHeaderBytes",
        "kStateRecordHeaderBytes",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }

    for forbidden in [
        "write_all(stream, &header, sizeof(header))",
        "write_all(stream, &record, sizeof(record))",
        "write_all(stream, &instance->parameter_values_[index], sizeof(double))",
        "read_all(stream, &header, sizeof(header))",
        "read_all(stream, &record, sizeof(record))",
        "read_all(stream, &value, sizeof(value))",
    ] {
        assert!(!backend.contains(forbidden), "native object-layout state leak `{forbidden}`:\n{backend}");
    }
}

#[test]
fn issue10_state_load_rescans_host_values_and_stale_parameter_events_are_ignored() {
    let plan = render_source();
    let backend = generated_text(&plan, "clapgen_instance_backend.hpp");

    for required in [
        "clap_host_params_t",
        "CLAP_PARAM_RESCAN_VALUES",
        "cache_host_params",
        "notify_host_parameter_values_changed",
        "continue; // unknown/stale parameter id",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }
}

#[test]
fn issue10_parameter_conversion_preserves_units_and_stepped_domain() {
    let plan = render_source();
    let extensions = generated_text(&plan, "clapgen_extensions.hpp");
    let backend = generated_text(&plan, "clapgen_instance_backend.hpp");

    for required in ["const char* unit", "std::int64_t steps", "\"x\"", "3"] {
        assert!(extensions.contains(required), "missing `{required}`:\n{extensions}");
    }
    for required in [
        "parameter_value_is_valid",
        "parameter_text_suffix_matches",
        "spec.unit",
        "spec.steps",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }
}
