use std::path::Path;

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::{GenerationPlan, OUTPUT_NAMES, render};

const SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.entry\" name=\"Entry\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"EntryProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";

fn plan() -> GenerationPlan {
    let path = Path::new("plugin.kdl");
    let metadata = parse_metadata(path, SOURCE).expect("metadata should parse");
    let ir = build_ir(path, SOURCE, &metadata).expect("canonical IR should build");
    render(&ir)
}

fn generated_text<'a>(plan: &'a GenerationPlan, path: &str) -> &'a str {
    let file = plan.files.iter().find(|file| file.path == path).expect("generated file");
    std::str::from_utf8(&file.bytes).expect("generated files must be UTF-8")
}

#[test]
fn issue59_adds_fixed_entry_and_backend_outputs() {
    for required in
        ["clapgen_entry.cpp", "clapgen_instance_backend.hpp", "clapgen_instance_backend.cpp"]
    {
        assert!(OUTPUT_NAMES.contains(&required), "missing fixed output `{required}`");
    }

    let plan = plan();
    for required in
        ["clapgen_entry.cpp", "clapgen_instance_backend.hpp", "clapgen_instance_backend.cpp"]
    {
        assert!(!generated_text(&plan, required).is_empty(), "`{required}` must not be empty");
    }
}

#[test]
fn issue59_private_backend_seam_uses_only_native_clap_types() {
    let plan = plan();
    let header = generated_text(&plan, "clapgen_instance_backend.hpp");
    let source = generated_text(&plan, "clapgen_instance_backend.cpp");

    for required in [
        "#include <clap/clap.h>",
        "#include <cstdint>",
        "namespace clapgen::generated::detail",
        "const clap_plugin_t* create_plugin_instance(",
        "std::uint32_t descriptor_index",
        "const clap_host_t* host",
    ] {
        assert!(header.contains(required), "missing `{required}`:\n{header}");
    }

    assert!(source.contains("#include \"clapgen_instance_backend.hpp\""), "{source}");
    assert!(source.contains("return nullptr;"), "fallback backend must return nullptr:\n{source}");

    for forbidden in [
        "FactoryContext",
        "PluginHandle",
        "HostWrapper",
        "std::function",
        "std::vector",
        "std::unique_ptr",
        "std::shared_ptr",
        "new ",
        "malloc(",
        "calloc(",
    ] {
        assert!(
            !header.contains(forbidden),
            "unexpected `{forbidden}` in backend header:\n{header}"
        );
        assert!(
            !source.contains(forbidden),
            "unexpected `{forbidden}` in backend source:\n{source}"
        );
    }
}

#[test]
fn issue59_backend_does_not_preempt_later_abi_exception_containment() {
    let plan = plan();
    let header = generated_text(&plan, "clapgen_instance_backend.hpp");
    let source = generated_text(&plan, "clapgen_instance_backend.cpp");

    assert!(
        !header.contains("create_plugin_instance(") || !header.contains("host) noexcept"),
        "the private creation backend must be allowed to report C++ failures to the future C ABI guard:\n{header}"
    );
    assert!(
        !source.contains("create_plugin_instance(") || !source.contains(") noexcept"),
        "the fallback definition must match the throwable private seam:\n{source}"
    );
}

#[test]
fn issue59_entry_output_stays_native_and_does_not_implement_instance_lifecycle() {
    let plan = plan();
    let entry = generated_text(&plan, "clapgen_entry.cpp");

    for required in [
        "#include <clap/clap.h>",
        "#include \"clapgen_descriptors.hpp\"",
        "#include \"clapgen_instance_backend.hpp\"",
    ] {
        assert!(entry.contains(required), "missing `{required}`:\n{entry}");
    }

    for forbidden in [
        "struct FactoryContext",
        "struct PluginHandle",
        "class PluginInstance",
        "plugin_data =",
        ".destroy =",
        ".activate =",
        ".process =",
        "get_extension",
    ] {
        assert!(
            !entry.contains(forbidden),
            "#59 pulled instance lifecycle work via `{forbidden}`:\n{entry}"
        );
    }
}
