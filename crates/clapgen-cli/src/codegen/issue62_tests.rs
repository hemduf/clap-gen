use super::entry_cpp;

#[test]
fn issue62_generates_defensive_entry_callbacks_and_one_global_export() {
    let source = entry_cpp::source();

    for required in [
        "constinit std::uint32_t entry_init_depth = 0u;",
        "bool CLAP_ABI entry_init(const char*) {",
        "++entry_init_depth;",
        "if (entry_init_depth > 0u) {",
        "--entry_init_depth;",
        "entry_init_depth == 0u",
        "factory_id == nullptr",
        "std::strcmp(factory_id, CLAP_PLUGIN_FACTORY_ID) != 0",
        "return &generated_plugin_factory;",
        "extern \"C\" {",
        "CLAP_EXPORT constinit const clap_plugin_entry_t clap_entry{",
        ".clap_version = CLAP_VERSION,",
        ".init = clapgen::generated::detail::entry_init,",
        ".deinit = clapgen::generated::detail::entry_deinit,",
        ".get_factory = clapgen::generated::detail::entry_get_factory,",
    ] {
        assert!(source.contains(required), "missing `{required}`:\n{source}");
    }

    assert_eq!(
        source.matches("CLAP_EXPORT constinit const clap_plugin_entry_t clap_entry{").count(),
        1,
        "exactly one exported clap_entry definition is allowed:\n{source}"
    );

    let namespace_end = source
        .find("} // namespace clapgen::generated::detail")
        .expect("detail namespace must close");
    let export = source
        .find("CLAP_EXPORT constinit const clap_plugin_entry_t clap_entry{")
        .expect("clap_entry export must exist");
    assert!(
        export > namespace_end,
        "clap_entry must be defined at global namespace, after detail closes:\n{source}"
    );

    for forbidden in [
        "clapgen::generated::detail::clap_entry",
        "std::mutex",
        "std::recursive_mutex",
        "std::atomic",
        "std::vector",
        "std::map",
        "std::unordered_map",
        "std::string",
        "new ",
        "malloc(",
        "calloc(",
        "realloc(",
        "fprintf(",
        "printf(",
    ] {
        assert!(!source.contains(forbidden), "unexpected `{forbidden}`:\n{source}");
    }
}
