use std::path::Path;

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::{GenerationPlan, OUTPUT_NAMES, instance_backend_cpp, render};

const SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.issue51\" name=\"Issue51\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"Issue51Processor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";

fn render_source(source: &str) -> GenerationPlan {
    let path = Path::new("plugin.kdl");
    let metadata = parse_metadata(path, source).expect("metadata should parse");
    let ir = build_ir(path, source, &metadata).expect("canonical IR should build");
    render(&ir)
}

fn generated_text<'a>(plan: &'a GenerationPlan, path: &str) -> &'a str {
    let file = plan.files.iter().find(|file| file.path == path).expect("generated file");
    std::str::from_utf8(&file.bytes).expect("generated file must be UTF-8")
}

#[test]
fn issue51_adds_a_static_extension_surface_and_wires_plugin_dispatch() {
    assert!(OUTPUT_NAMES.contains(&"clapgen_extensions.hpp"), "{OUTPUT_NAMES:?}");

    let plan = render_source(SOURCE);
    let extensions = generated_text(&plan, "clapgen_extensions.hpp");
    for required in [
        "struct PluginExtensionBinding",
        "lookup_plugin_extension(",
        "generated_plugin_extension(",
        "std::strcmp(extension_id, bindings[index].id) == 0",
        "plugin_extension_count = 0u",
    ] {
        assert!(extensions.contains(required), "missing `{required}`:\n{extensions}");
    }

    for forbidden in ["std::map", "std::unordered_map", "std::vector", "new ", "malloc(", "mutex"] {
        assert!(!extensions.contains(forbidden), "unexpected `{forbidden}`:\n{extensions}");
    }

    let backend = instance_backend_cpp::header();
    for required in [
        "#include \"clapgen_extensions.hpp\"",
        ".get_extension = get_extension_plugin,",
        "static const void* CLAP_ABI get_extension_plugin(",
        "return generated_plugin_extension(extension_id);",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }
    assert!(!backend.contains("unavailable_get_extension"), "{backend}");
}

#[test]
fn issue51_declared_but_unowned_extensions_are_not_emitted_or_exposed() {
    let source = SOURCE
        .replace("extensions {}", "extensions { enable \"clap.latency\"; enable \"clap.tail\" }");
    let plan = render_source(&source);
    let extensions = generated_text(&plan, "clapgen_extensions.hpp");

    for forbidden in [
        "#include <clap/ext/latency.h>",
        "#include <clap/ext/tail.h>",
        "clap_plugin_latency_t",
        "clap_plugin_tail_t",
        "CLAP_EXT_LATENCY",
        "CLAP_EXT_TAIL",
    ] {
        assert!(
            !extensions.contains(forbidden),
            "declared-but-unowned extension leaked through `{forbidden}`:\n{extensions}"
        );
    }
    assert!(extensions.contains("plugin_extension_count = 0u"), "{extensions}");
}

#[test]
fn issue51_review_requires_null_binding_safety_and_deterministic_owned_binding_order() {
    let implementation = include_str!("extension_cpp.rs");

    assert!(
        implementation.contains("if (bindings == nullptr || extension_id == nullptr)"),
        "lookup must reject a null binding table even when count is non-zero:\n{implementation}"
    );
    assert!(
        implementation.contains("owned.sort_by(|left, right| left.id.cmp(right.id));"),
        "future owned bindings must be sorted by extension id before emission:\n{implementation}"
    );
    assert!(
        implementation.contains("cpp_literal::utf8_c_string(binding.id)"),
        "extension ids must reuse the ABI C-string literal authority:\n{implementation}"
    );
}
