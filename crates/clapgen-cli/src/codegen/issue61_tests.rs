use std::path::Path;

use crate::ir::{PluginIr, build_ir};
use crate::metadata::parse_metadata;

use super::{GenerationPlan, render, render_descriptors_for_plugins};

const SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.dispatch\" name=\"Dispatch\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"DispatchProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";

fn plan() -> GenerationPlan {
    let path = Path::new("plugin.kdl");
    let metadata = parse_metadata(path, SOURCE).expect("metadata should parse");
    let ir = build_ir(path, SOURCE, &metadata).expect("canonical IR should build");
    render(&ir)
}

fn entry_source(plan: &GenerationPlan) -> &str {
    let file = plan
        .files
        .iter()
        .find(|file| file.path == "clapgen_entry.cpp")
        .expect("entry source must be generated");
    std::str::from_utf8(&file.bytes).expect("entry source must be UTF-8")
}

fn plugin(id: &str) -> PluginIr {
    PluginIr {
        id: id.to_owned(),
        name: id.to_owned(),
        vendor: "Example".to_owned(),
        version: "1.0.0".to_owned(),
        url: None,
        manual_url: None,
        support_url: None,
        description: None,
        features: Vec::new(),
    }
}

#[test]
fn issue61_create_plugin_validates_inputs_version_and_matches_id_by_content() {
    let plan = plan();
    let entry = entry_source(&plan);

    for required in [
        "#include <cstring>",
        ".create_plugin = factory_create_plugin,",
        "factory != &generated_plugin_factory",
        "host == nullptr",
        "plugin_id == nullptr",
        "clap_version_is_compatible(host->clap_version)",
        "for (std::uint32_t index = 0; index < plugin_descriptor_count; ++index)",
        "std::strcmp(plugin_id, plugin_descriptors[index]->id) == 0",
        "return create_plugin_instance(index, host);",
    ] {
        assert!(entry.contains(required), "missing `{required}`:\n{entry}");
    }

    for forbidden in [
        ".create_plugin = nullptr,",
        "plugin_id == plugin_descriptors[index]->id",
        "host->get_extension",
        "host->request_restart",
        "host->request_process",
        "host->request_callback",
        "std::string",
        "std::vector",
        "std::map",
        "std::unordered_map",
        "std::function",
        "std::unique_ptr",
        "std::shared_ptr",
        "new ",
        "malloc(",
        "calloc(",
        "realloc(",
    ] {
        assert!(!entry.contains(forbidden), "unexpected `{forbidden}`:\n{entry}");
    }
}

#[test]
fn issue61_generic_dispatch_uses_sorted_descriptor_index_for_multiple_ids() {
    let descriptors = render_descriptors_for_plugins(&[
        plugin("com.example.zeta"),
        plugin("com.example.alpha"),
    ])
    .expect("descriptor collection should render");
    let entry = super::entry_cpp::source();

    let alpha = descriptors.find(".id = \"com.example.alpha\"").expect("alpha descriptor");
    let zeta = descriptors.find(".id = \"com.example.zeta\"").expect("zeta descriptor");
    assert!(alpha < zeta, "descriptors must be sorted by stable plugin ID:\n{descriptors}");
    assert!(descriptors.contains("&plugin_descriptor_0,"), "{descriptors}");
    assert!(descriptors.contains("&plugin_descriptor_1,"), "{descriptors}");

    for required in [
        "std::strcmp(plugin_id, plugin_descriptors[index]->id) == 0",
        "return create_plugin_instance(index, host);",
    ] {
        assert!(
            entry.contains(required),
            "generic routing must preserve the sorted descriptor index via `{required}`:\n{entry}"
        );
    }
}
