use crate::ir::PluginIr;

use super::render_descriptors_for_plugins;

fn plugin(id: &str, name: &str) -> PluginIr {
    PluginIr {
        id: id.to_owned(),
        name: name.to_owned(),
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
fn issue52_three_plugin_fixture_is_exact_deterministic_descriptor_output() {
    let rendered = render_descriptors_for_plugins(&[
        plugin("com.example.zeta", "Zeta"),
        plugin("com.example.alpha", "Alpha"),
        plugin("com.example.fail", "Fail"),
    ])
    .expect("three unique plugin descriptors should render");
    let fixture = include_str!("../../../../tests/codegen/issue52/clapgen_descriptors.hpp")
        .replace("\r\n", "\n");

    assert_eq!(rendered, fixture, "the runtime bundle fixture must remain generated-shape exact");

    let alpha = rendered.find(".id = \"com.example.alpha\"").expect("alpha descriptor");
    let failing = rendered.find(".id = \"com.example.fail\"").expect("failing descriptor");
    let zeta = rendered.find(".id = \"com.example.zeta\"").expect("zeta descriptor");
    assert!(alpha < failing && failing < zeta, "descriptor IDs must be sorted deterministically");
    assert!(rendered.contains("inline constexpr std::uint32_t plugin_descriptor_count = 3u;"));
}
