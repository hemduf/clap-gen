use super::entry_cpp;

#[test]
fn issue63_discovery_callbacks_gate_stale_factory_usage_and_stay_bounded() {
    let source = entry_cpp::source();

    for required in [
        "if (entry_init_depth == 0u || factory != &generated_plugin_factory) {",
        "if (entry_init_depth == 0u || factory != &generated_plugin_factory ||",
        "index >= plugin_descriptor_count",
        "for (std::uint32_t index = 0; index < plugin_descriptor_count; ++index)",
    ] {
        assert!(source.contains(required), "missing `{required}`:\n{source}");
    }

    for forbidden in [
        "std::mutex",
        "std::recursive_mutex",
        "std::condition_variable",
        "std::vector",
        "std::map",
        "std::unordered_map",
        "std::string",
        "std::filesystem",
        "std::fstream",
        "fopen(",
        "fprintf(",
        "printf(",
        "new ",
        "malloc(",
        "calloc(",
        "realloc(",
    ] {
        assert!(!source.contains(forbidden), "unexpected `{forbidden}`:\n{source}");
    }
}
