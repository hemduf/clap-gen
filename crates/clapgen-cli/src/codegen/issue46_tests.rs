use std::path::Path;

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::{GenerationPlan, render};

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
    assert!(descriptors.contains("inline constexpr std::uint32_t plugin_descriptor_count = 1u;"), "{descriptors}");
}

#[test]
fn absent_optional_fields_and_empty_features_use_null_without_runtime_initialization() {
    let plan = plan_from(MINIMAL_SOURCE);
    let descriptors = generated_text(&plan, "clapgen_descriptors.hpp");

    for field in [".url = nullptr,", ".manual_url = nullptr,", ".support_url = nullptr,", ".description = nullptr,"] {
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
fn descriptor_generation_is_deterministic() {
    let first = plan_from(RICH_SOURCE);
    let second = plan_from(RICH_SOURCE);

    assert_eq!(first, second);
}
