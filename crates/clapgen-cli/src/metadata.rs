use std::path::{Path, PathBuf};

use kdl::KdlDocument;

#[derive(Debug, Clone)]
pub(crate) struct ParsedMetadata {
    pub(crate) document: KdlDocument,
    pub(crate) imports: Vec<PathBuf>,
}

pub(crate) fn parse_metadata(_path: &Path, _source: &str) -> Result<ParsedMetadata, String> {
    Err("metadata parser not implemented".to_owned())
}

pub(crate) fn format_metadata(_path: &Path, _source: &str) -> Result<String, String> {
    Err("metadata formatter not implemented".to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{format_metadata, parse_metadata};

    const VALID: &str = r#"// user comment
clapgen schema="1.0.0"
import "shared/common.kdl"
plugin id="com.example.gain" name="Gain" vendor="Example" version="1.0.0"
processor class="GainProcessor"
parameters {
    param "gain" id="gain" min=0.0 max=2.0 default=1.0
}
audio-ports {
    input "main" id="audio-in" channels=2
    output "main" id="audio-out" channels=2
}
note-ports {}
state {}
gui {}
presets {}
extensions {
    namespace "acme"
}
/- disabled-feature reason="kept for migration"
acme.widget acme.mode="fast"
"#;

    #[test]
    fn parses_canonical_metadata_and_tracks_imports() {
        let parsed = parse_metadata(Path::new("plugin.kdl"), VALID).expect("valid KDL should parse");
        assert_eq!(vec![PathBuf::from("shared/common.kdl")], parsed.imports);
        assert!(parsed.document.get("plugin").is_some());
        assert!(parsed.document.get("processor").is_some());
    }

    #[test]
    fn formatting_preserves_comments_and_slashdash_and_is_idempotent() {
        let source = VALID.replace("    param", "        param");
        let once = format_metadata(Path::new("plugin.kdl"), &source).expect("format should work");
        let twice = format_metadata(Path::new("plugin.kdl"), &once).expect("second format should work");

        assert_eq!(once, twice);
        assert!(once.contains("// user comment"));
        assert!(once.contains("/- disabled-feature"));
    }

    #[test]
    fn rejects_unknown_nodes_with_stable_location_and_hint() {
        let source = "clapgen schema=\"1.0.0\"\nplugin id=\"x\" name=\"x\" vendor=\"x\" version=\"1\"\nmystery value=1\n";
        let error = parse_metadata(Path::new("bad/plugin.kdl"), source).expect_err("unknown node must fail");

        assert!(error.contains("bad/plugin.kdl:3"), "{error}");
        assert!(error.contains("node `mystery`"), "{error}");
        assert!(error.contains("unknown node"), "{error}");
        assert!(error.contains("hint:"), "{error}");
    }

    #[test]
    fn rejects_unknown_properties_unless_namespaced() {
        let source = "clapgen schema=\"1.0.0\"\nplugin id=\"x\" name=\"x\" vendor=\"x\" version=\"1\" surprise=1\n";
        let error = parse_metadata(Path::new("plugin.kdl"), source).expect_err("unknown property must fail");
        assert!(error.contains("property `surprise`"), "{error}");

        let namespaced = "clapgen schema=\"1.0.0\"\nextensions { namespace \"acme\" }\nplugin id=\"x\" name=\"x\" vendor=\"x\" version=\"1\" acme.mode=\"fast\"\nacme.extra enabled=#true\n";
        parse_metadata(Path::new("plugin.kdl"), namespaced).expect("declared extension namespace should be accepted");
    }

    #[test]
    fn rejects_yaml_and_kdl1_with_migration_guidance() {
        let yaml = "plugin:\n  id: com.example.gain\n";
        let yaml_error = parse_metadata(Path::new("plugin.yaml"), yaml).expect_err("YAML must fail");
        assert!(yaml_error.contains("YAML"), "{yaml_error}");
        assert!(yaml_error.contains("KDL 2.0"), "{yaml_error}");

        let kdl1 = "clapgen schema=\"1.0.0\"\nplugin id=\"x\" name=\"x\" vendor=\"x\" version=\"1\" enabled=true\n";
        let kdl1_error = parse_metadata(Path::new("plugin.kdl"), kdl1).expect_err("KDL 1 style literal must fail");
        assert!(kdl1_error.contains("KDL 1"), "{kdl1_error}");
        assert!(kdl1_error.contains("KDL 2.0"), "{kdl1_error}");
    }
}
