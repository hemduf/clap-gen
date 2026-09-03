use std::path::Path;

use crate::ir::{PluginIr, build_ir};
use crate::metadata::parse_metadata;

use super::{GenerationPlan, render, render_descriptors_for_plugins};

const RICH_SOURCE: &str = r#"clapgen schema="1.0.0"
plugin id="com.example.descriptor" name="Descriptor" vendor="Example Labs" version="2.3.4" url="https://example.test/plugin" manual-url="https://example.test/manual" support-url="https://example.test/support" description="Immutable CLAP descriptor" {
    feature "audio-effect"
    feature "stereo"
}
processor class="DescriptorProcessor"
parameters {}
audio-ports {}
note-ports {}
state {}
gui {}
presets {}
factories {}
extensions {}
"#;

const MINIMAL_SOURCE: &str = r#"clapgen schema="1.0.0"
plugin id="com.example.minimal" name="Minimal" vendor="Example" version="1.0.0"
processor class="MinimalProcessor"
parameters {}
audio-ports {}
note-ports {}
state {}
gui {}
presets {}
factories {}
extensions {}
"#;

fn plan_from(source: &str) -> GenerationPlan {
    let path = Path::new("plugin.kdl");
    let metadata = parse_metadata(path, source).expect("metadata should parse");
    let ir = build_ir(path, source, &metadata).expect("canonical IR should build");
    render(&ir)
}

fn generated_text<'a>(plan: &'a GenerationPlan, path: &str) -> &'a str {
    let file = plan.files.iter().find(|file| file.path == path).expect("generated file");
    std::str::from_utf8(&file.bytes).expect("generated files must be UTF-8")
}

fn plugin(id: &str, name: &str, feature: &str) -> PluginIr {
    PluginIr {
        id: id.to_owned(),
        name: name.to_owned(),
        vendor: "Example".to_owned(),
        version: "1.0.0".to_owned(),
        url: None,
        manual_url: None,
        support_url: None,
        description: None,
        features: vec![feature.to_owned()],
    }
}

#[test]
fn generated_descriptor_maps_every_canonical_plugin_field_to_native_clap() {
    let plan = plan_from(RICH_SOURCE);
    let descriptors = generated_text(&plan, "clapgen_descriptors.hpp");

    for required in [
        "#include <clap/clap.h>",
        "inline constexpr clap_plugin_descriptor_t plugin_descriptor_0{",
        ".clap_version = CLAP_VERSION,",
        ".id = \"com.example.descriptor\",",
        ".name = \"Descriptor\",",
        ".vendor = \"Example Labs\",",
        ".url = \"https://example.test/plugin\",",
        ".manual_url = \"https://example.test/manual\",",
        ".support_url = \"https://example.test/support\",",
        ".version = \"2.3.4\",",
        ".description = \"Immutable CLAP descriptor\",",
        ".features = plugin_features_0,",
    ] {
        assert!(descriptors.contains(required), "missing `{required}`:\n{descriptors}");
    }

    for forbidden in [
        "clap_entry",
        "clap_plugin_factory_t",
        "create_plugin",
        "activate(",
        "process(",
        "new ",
        "malloc(",
        "std::vector",
        "std::string",
    ] {
        assert!(!descriptors.contains(forbidden), "unexpected `{forbidden}`:\n{descriptors}");
    }
}

#[test]
fn feature_storage_is_immutable_static_and_null_terminated() {
    let plan = plan_from(RICH_SOURCE);
    let descriptors = generated_text(&plan, "clapgen_descriptors.hpp");

    assert!(
        descriptors.contains(
            "inline constexpr const char* const plugin_features_0[] = {\n    \"audio-effect\",\n    \"stereo\",\n    nullptr,\n};"
        ),
        "{descriptors}"
    );
    assert!(
        descriptors.contains(
            "inline constexpr const clap_plugin_descriptor_t* const plugin_descriptors[] = {"
        ),
        "{descriptors}"
    );
    assert!(descriptors.contains("&plugin_descriptor_0,"), "{descriptors}");
    assert!(
        descriptors.contains("inline constexpr std::uint32_t plugin_descriptor_count = 1u;"),
        "{descriptors}"
    );
}

#[test]
fn absent_optional_fields_and_empty_features_use_null_without_runtime_initialization() {
    let plan = plan_from(MINIMAL_SOURCE);
    let descriptors = generated_text(&plan, "clapgen_descriptors.hpp");

    for field in [
        ".url = nullptr,",
        ".manual_url = nullptr,",
        ".support_url = nullptr,",
        ".description = nullptr,",
    ] {
        assert!(descriptors.contains(field), "missing `{field}`:\n{descriptors}");
    }
    assert!(
        descriptors.contains(
            "inline constexpr const char* const plugin_features_0[] = {\n    nullptr,\n};"
        ),
        "{descriptors}"
    );
}

#[test]
fn descriptor_exposed_c_strings_reject_embedded_nul_before_codegen() {
    for (label, source) in [
        (
            "name",
            RICH_SOURCE.replace(
                "name=\"Descriptor\"",
                "name=\"Descriptor\\u{0}Hidden\"",
            ),
        ),
        (
            "feature",
            RICH_SOURCE.replace(
                "feature \"audio-effect\"",
                "feature \"audio-effect\\u{0}hidden\"",
            ),
        ),
    ] {
        let path = Path::new("plugin.kdl");
        let metadata = parse_metadata(path, &source)
            .unwrap_or_else(|error| panic!("{label} KDL unicode escape should parse: {error}"));
        let error = build_ir(path, &source, &metadata)
            .expect_err("descriptor-exposed strings containing NUL must be rejected");
        assert!(error.contains("NUL"), "{label}: {error}");
        assert!(error.contains("plugin"), "{label}: {error}");
        assert!(error.contains("hint:"), "{label}: {error}");
    }
}

#[test]
fn cxx_string_literal_escaping_has_one_codegen_authority() {
    let descriptor_renderer = include_str!("descriptor_cpp.rs");
    let metadata_renderer = include_str!("metadata_cpp.rs");

    assert!(
        !descriptor_renderer.contains("fn cpp_string(")
            && !metadata_renderer.contains("fn cpp_string("),
        "C++ string literal escaping must live in one shared codegen helper"
    );
}

#[test]
fn descriptor_generation_is_deterministic() {
    let first = plan_from(RICH_SOURCE);
    let second = plan_from(RICH_SOURCE);

    assert_eq!(first, second);
}

#[test]
fn multi_descriptor_renderer_sorts_by_id_and_builds_a_stable_pointer_table() {
    let plugins = vec![
        plugin("com.example.zeta", "Zeta", "instrument"),
        plugin("com.example.alpha", "Alpha", "audio-effect"),
        plugin("com.example.middle", "Middle", "note-effect"),
    ];

    let first = render_descriptors_for_plugins(&plugins);
    let second = render_descriptors_for_plugins(&plugins);
    assert_eq!(first, second, "multi-descriptor rendering must be deterministic");

    let alpha = first.find(".id = \"com.example.alpha\",").expect("alpha descriptor");
    let middle = first.find(".id = \"com.example.middle\",").expect("middle descriptor");
    let zeta = first.find(".id = \"com.example.zeta\",").expect("zeta descriptor");
    assert!(
        alpha < middle && middle < zeta,
        "descriptors must be ordered by stable plugin id:\n{first}"
    );

    for required in [
        "inline constexpr clap_plugin_descriptor_t plugin_descriptor_0{",
        "inline constexpr clap_plugin_descriptor_t plugin_descriptor_1{",
        "inline constexpr clap_plugin_descriptor_t plugin_descriptor_2{",
        "&plugin_descriptor_0,",
        "&plugin_descriptor_1,",
        "&plugin_descriptor_2,",
        "inline constexpr std::uint32_t plugin_descriptor_count = 3u;",
    ] {
        assert!(first.contains(required), "missing `{required}`:\n{first}");
    }
}
