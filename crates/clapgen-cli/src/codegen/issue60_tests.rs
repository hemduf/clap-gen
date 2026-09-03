use std::path::Path;

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::{GenerationPlan, render};

const SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.factory\" name=\"Factory\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"FactoryProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";

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

#[test]
fn issue60_generates_one_constant_initialized_immutable_factory() {
    let plan = plan();
    let entry = entry_source(&plan);

    for required in [
        "constinit const clap_plugin_factory_t generated_plugin_factory{",
        ".get_plugin_count = factory_get_plugin_count,",
        ".get_plugin_descriptor = factory_get_plugin_descriptor,",
    ] {
        assert!(entry.contains(required), "missing `{required}`:\n{entry}");
    }

    assert_eq!(
        entry.matches("generated_plugin_factory{").count(),
        1,
        "exactly one generated factory object is allowed:\n{entry}"
    );
}

#[test]
fn issue60_factory_enumeration_reuses_descriptor_storage_and_stays_allocation_free() {
    let plan = plan();
    let entry = entry_source(&plan);

    for required in [
        "return plugin_descriptor_count;",
        "return plugin_descriptors[index];",
        "factory != &generated_plugin_factory",
        "index >= plugin_descriptor_count",
    ] {
        assert!(entry.contains(required), "missing `{required}`:\n{entry}");
    }

    for forbidden in [
        "clap_plugin_descriptor_t copied_",
        "std::vector",
        "std::array",
        "std::map",
        "std::unordered_map",
        "std::mutex",
        "std::lock_guard",
        "new ",
        "malloc(",
        "calloc(",
        "realloc(",
        "fprintf(",
        "printf(",
    ] {
        assert!(!entry.contains(forbidden), "unexpected `{forbidden}`:\n{entry}");
    }
}
