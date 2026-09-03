use std::path::Path;

use crate::ir::{PluginIr, build_ir};
use crate::metadata::parse_metadata;

use super::render_descriptors_for_plugins;

fn plugin() -> PluginIr {
    PluginIr {
        id: "com.example.collection".to_owned(),
        name: "Collection".to_owned(),
        vendor: "Example".to_owned(),
        version: "1.0.0".to_owned(),
        url: Some("https://example.test/plugin".to_owned()),
        manual_url: Some("https://example.test/manual".to_owned()),
        support_url: Some("https://example.test/support".to_owned()),
        description: Some("Collection descriptor".to_owned()),
        features: vec!["audio-effect".to_owned()],
    }
}

#[test]
fn collection_renderer_rejects_embedded_nul_in_every_descriptor_c_string() {
    for field in
        ["id", "name", "vendor", "version", "url", "manual-url", "support-url", "description"]
    {
        let mut value = plugin();
        match field {
            "id" => value.id = "com.example.collection\0hidden".to_owned(),
            "name" => value.name = "Collection\0Hidden".to_owned(),
            "vendor" => value.vendor = "Example\0Hidden".to_owned(),
            "version" => value.version = "1.0.0\0hidden".to_owned(),
            "url" => value.url = Some("https://example.test/\0hidden".to_owned()),
            "manual-url" => {
                value.manual_url = Some("https://example.test/manual\0hidden".to_owned());
            }
            "support-url" => {
                value.support_url = Some("https://example.test/support\0hidden".to_owned());
            }
            "description" => value.description = Some("Description\0Hidden".to_owned()),
            _ => unreachable!(),
        }

        let error = render_descriptors_for_plugins(&[value])
            .expect_err("collection renderer must reject embedded NUL before C++ generation");
        assert!(error.contains("NUL"), "{field}: {error}");
        assert!(error.contains(field), "{field}: {error}");
    }
}

#[test]
fn collection_renderer_rejects_embedded_nul_in_plugin_features() {
    let mut value = plugin();
    value.features = vec!["audio-effect\0hidden".to_owned()];

    let error = render_descriptors_for_plugins(&[value])
        .expect_err("collection renderer must reject feature NUL before C++ generation");
    assert!(error.contains("NUL"), "{error}");
    assert!(error.contains("feature"), "{error}");
}

#[test]
fn canonical_feature_nul_diagnostic_points_to_the_feature_line() {
    let source = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.diagnostic\" name=\"Diagnostic\" vendor=\"Example\" version=\"1.0.0\" {\n    feature \"audio-effect\\u{0}hidden\"\n}\nprocessor class=\"DiagnosticProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";
    let path = Path::new("plugin.kdl");
    let metadata = parse_metadata(path, source).expect("KDL unicode escape should parse");
    let error = build_ir(path, source, &metadata)
        .expect_err("descriptor feature containing NUL must fail semantic validation");

    assert!(error.contains("plugin.kdl:3"), "{error}");
    assert!(error.contains("feature"), "{error}");
    assert!(error.contains("NUL"), "{error}");
}
