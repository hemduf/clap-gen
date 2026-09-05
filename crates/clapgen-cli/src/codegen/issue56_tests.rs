#[test]
fn issue56_extension_dispatch_smoke_obeys_clap_init_precondition() {
    let smoke = include_str!("../../../../tests/codegen/issue51/extension_dispatch_smoke.cpp");
    let create = smoke
        .find("create_plugin_instance_for<Processor>")
        .expect("issue51 smoke must create a generated plugin instance");
    let after_create = &smoke[create..];
    let init = after_create
        .find("plugin->init(plugin)")
        .expect("CLAP get_extension must not be queried before plugin init");
    let query = after_create
        .find("plugin->get_extension(plugin")
        .expect("issue51 smoke must exercise get_extension");

    assert!(
        init < query,
        "CLAP 1.2.10 forbids get_extension before init; initialize the plugin first:\n{after_create}"
    );
}
