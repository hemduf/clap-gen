use super::{OUTPUT_NAMES, entry_cpp, instance_backend_cpp};

#[test]
fn issue65_final_entry_factory_abi_and_routing_conformity() {
    for required in [
        "clapgen_descriptors.hpp",
        "clapgen_entry.cpp",
        "clapgen_instance_backend.cpp",
        "clapgen_instance_backend.hpp",
    ] {
        assert!(OUTPUT_NAMES.contains(&required), "missing fixed output `{required}`");
    }

    let entry = entry_cpp::source();
    for required in [
        "constinit std::uint32_t entry_init_depth = 0u;",
        "bool CLAP_ABI entry_init(const char*);",
        "void CLAP_ABI entry_deinit();",
        "const void* CLAP_ABI entry_get_factory(const char* factory_id);",
        "std::uint32_t CLAP_ABI factory_get_plugin_count(const clap_plugin_factory_t* factory);",
        "const clap_plugin_descriptor_t* CLAP_ABI factory_get_plugin_descriptor(",
        "const clap_plugin_t* CLAP_ABI factory_create_plugin(",
        "constinit const clap_plugin_factory_t generated_plugin_factory{",
        "entry_init_depth == std::numeric_limits<std::uint32_t>::max()",
        "if (entry_init_depth > 0u) {",
        "entry_init_depth == 0u",
        "std::strcmp(factory_id, CLAP_PLUGIN_FACTORY_ID) != 0",
        "return plugin_descriptor_count;",
        "return plugin_descriptors[index];",
        "clap_version_is_compatible(host->clap_version)",
        "std::strcmp(plugin_id, plugin_descriptors[index]->id) == 0",
        "return create_plugin_instance(index, host);",
        "extern \"C\" {",
        "CLAP_EXPORT extern constinit const clap_plugin_entry_t clap_entry{",
    ] {
        assert!(entry.contains(required), "missing `{required}`:\n{entry}");
    }

    assert_eq!(
        entry.matches("generated_plugin_factory{").count(),
        1,
        "exactly one generated factory object is allowed:\n{entry}"
    );
    assert_eq!(
        entry
            .matches("CLAP_EXPORT extern constinit const clap_plugin_entry_t clap_entry{")
            .count(),
        1,
        "exactly one exported clap_entry definition with external linkage is allowed:\n{entry}"
    );

    let namespace_end = entry
        .find("} // namespace clapgen::generated::detail")
        .expect("detail namespace must close");
    let export = entry
        .find("CLAP_EXPORT extern constinit const clap_plugin_entry_t clap_entry{")
        .expect("clap_entry export with external linkage must exist");
    assert!(
        export > namespace_end,
        "clap_entry must be global and outside detail namespace:\n{entry}"
    );
}

#[test]
fn issue65_final_discovery_safety_and_backend_boundary_conformity() {
    let entry = entry_cpp::source();
    let backend_header = instance_backend_cpp::header();
    let backend_source = instance_backend_cpp::source();

    for required in [
        "#include <clap/clap.h>",
        "const clap_plugin_t* create_plugin_instance(",
        "std::uint32_t descriptor_index",
        "const clap_host_t* host",
    ] {
        assert!(
            backend_header.contains(required),
            "missing `{required}` from private backend seam:\n{backend_header}"
        );
    }
    assert!(
        backend_source.contains("return nullptr;"),
        "production fallback must remain allocation-free and fail closed:\n{backend_source}"
    );
    assert!(
        !backend_header.contains("host) noexcept") && !backend_source.contains(") noexcept"),
        "private backend must not preempt future C ABI exception containment"
    );

    for forbidden in [
        "FactoryContext",
        "PluginHandle",
        "HostWrapper",
        "std::atomic",
        "std::mutex",
        "std::recursive_mutex",
        "std::condition_variable",
        "std::function",
        "std::vector",
        "std::map",
        "std::unordered_map",
        "std::shared_ptr",
        "std::filesystem",
        "std::fstream",
        "malloc(",
        "calloc(",
        "realloc(",
        "fopen(",
        "fprintf(",
        "printf(",
        "host->get_extension",
        "host->request_restart",
        "host->request_process",
        "host->request_callback",
    ] {
        assert!(!entry.contains(forbidden), "unexpected `{forbidden}` in entry:\n{entry}");
        assert!(
            !backend_header.contains(forbidden),
            "unexpected `{forbidden}` in backend header:\n{backend_header}"
        );
        assert!(
            !backend_source.contains(forbidden),
            "unexpected `{forbidden}` in backend source:\n{backend_source}"
        );
    }

    for forbidden in [
        "std::unique_ptr",
        "new ",
        "delete ",
        ".destroy =",
        ".activate =",
        ".start_processing =",
        ".process =",
    ] {
        assert!(!entry.contains(forbidden), "discovery entry gained `{forbidden}`:\n{entry}");
        assert!(
            !backend_source.contains(forbidden),
            "fallback backend gained #48 ownership behavior via `{forbidden}`:\n{backend_source}"
        );
    }
}
