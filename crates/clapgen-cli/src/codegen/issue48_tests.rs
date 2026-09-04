use super::instance_backend_cpp;

#[test]
fn issue48_generates_private_raii_instance_ownership() {
    let header = instance_backend_cpp::header();

    for required in [
        "#include \"clapgen_descriptors.hpp\"",
        "#include \"clapgen_processor.hpp\"",
        "#include <memory>",
        "template <NativeProcessor Processor>",
        "class PluginInstance final",
        "static const clap_plugin_t* create(",
        "static PluginInstance* from_plugin(const clap_plugin_t* plugin) noexcept",
        "Processor& processor() noexcept",
        "const clap_host_t* host() const noexcept",
        "std::unique_ptr<PluginInstance> instance{new PluginInstance(host)};",
        "if (!instance->configure(descriptor))",
        ".desc = descriptor,",
        ".plugin_data = this,",
        ".destroy = destroy_plugin,",
        "static void CLAP_ABI destroy_plugin(const clap_plugin_t* plugin)",
        "auto* instance = from_plugin(plugin);",
        "delete instance;",
        "Processor processor_{};",
        "const clap_host_t* host_ = nullptr;",
        "clap_plugin_t plugin_{};",
        "const clap_plugin_t* create_plugin_instance_for(",
        "return PluginInstance<Processor>::create(plugin_descriptors[descriptor_index], host);",
    ] {
        assert!(header.contains(required), "missing `{required}`:\n{header}");
    }

    let processor = header.find("Processor processor_{};").expect("processor storage");
    let host = header.find("const clap_host_t* host_ = nullptr;").expect("host storage");
    let plugin = header.find("clap_plugin_t plugin_{};").expect("plugin storage");
    assert!(processor < host && host < plugin, "construction order must be explicit:\n{header}");

    for forbidden in [
        "namespace clapgen {\nclass PluginInstance",
        "FactoryContext",
        "PluginHandle",
        "HostWrapper",
        "std::shared_ptr",
        "std::vector",
        "std::map",
        "std::unordered_map",
        "std::mutex",
        "std::recursive_mutex",
        "std::condition_variable",
    ] {
        assert!(!header.contains(forbidden), "unexpected `{forbidden}`:\n{header}");
    }
}

#[test]
fn issue48_keeps_non_lifecycle_main_thread_callback_fail_closed() {
    let header = instance_backend_cpp::header();

    for required in [
        ".get_extension = get_extension_plugin,",
        ".on_main_thread = unavailable_on_main_thread,",
        "static void CLAP_ABI unavailable_on_main_thread(const clap_plugin_t*)",
    ] {
        assert!(header.contains(required), "missing `{required}`:\n{header}");
    }

    for forbidden in ["HostWrapper", "FactoryContext"] {
        assert!(!header.contains(forbidden), "#48 boundary regressed via `{forbidden}`:\n{header}");
    }
}
