use std::path::Path;

use crate::metadata::ParsedMetadata;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CanonicalIr {
    pub(crate) version: u32,
}

pub(crate) fn build_ir(_path: &Path, _source: &str, _metadata: &ParsedMetadata) -> Result<CanonicalIr, String> {
    Err("canonical IR not implemented".to_owned())
}

pub(crate) fn serialize_ir_kdl(_ir: &CanonicalIr) -> String {
    String::new()
}

pub(crate) fn capability_report_kdl(_ir: &CanonicalIr) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::metadata::parse_metadata;

    use super::{build_ir, capability_report_kdl, serialize_ir_kdl};

    fn build(source: &str) -> Result<super::CanonicalIr, String> {
        let path = Path::new("plugin.kdl");
        let parsed = parse_metadata(path, source)?;
        build_ir(path, source, &parsed)
    }

    const PREFIX: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"SynthProcessor\"\n";

    #[test]
    fn semantic_equivalence_produces_identical_canonical_ir() {
        let a = format!(
            "{PREFIX}parameters {{\n    param \"cutoff\" id=\"cutoff\" min=20.0 max=20000.0 default=1000.0 flags=\"modulatable,automatable\" unit=\"Hz\"\n    param \"gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=\"automatable\"\n}}\naudio-ports {{ input \"main\" id=\"in\" channels=2; output \"main\" id=\"out\" channels=2 }}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.params\" }}\n"
        );
        let b = format!(
            "{PREFIX}parameters {{\n    param \"gain\" default=0.5 max=1.0 min=0.0 id=\"gain\" flags=\"automatable\"\n    param \"cutoff\" default=1000.0 max=20000.0 min=20.0 id=\"cutoff\" unit=\"hz\" flags=\"automatable, modulatable\"\n}}\naudio-ports {{ output \"main\" channels=2 id=\"out\"; input \"main\" channels=2 id=\"in\" }}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.params\" }}\n"
        );

        let a = build(&a).expect("first manifest should build");
        let b = build(&b).expect("equivalent manifest should build");
        assert_eq!(serialize_ir_kdl(&a), serialize_ir_kdl(&b));
    }

    #[test]
    fn rejects_raw_numeric_clap_flag_bitmasks() {
        let source = format!(
            "{PREFIX}parameters {{ param \"gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=3 }}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        );
        let error = build(&source).expect_err("raw C bitmask must be rejected");
        assert!(error.contains("flags"), "{error}");
        assert!(error.contains("named"), "{error}");
        assert!(error.contains("gain"), "{error}");
    }

    #[test]
    fn cross_reference_error_identifies_source_and_missing_target() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{\n    input \"main\" id=\"in\" channels=2\n    output \"main\" id=\"out\" channels=2 in-place-pair=\"missing-input\"\n}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        );
        let error = build(&source).expect_err("missing target must fail");
        assert!(error.contains("out"), "{error}");
        assert!(error.contains("missing-input"), "{error}");
        assert!(error.contains("in-place-pair"), "{error}");
    }

    #[test]
    fn draft_extensions_require_exact_abi_id_and_version_pin() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.webview\" draft=#true }}\n"
        );
        let error = build(&source).expect_err("unpinned draft must fail");
        assert!(error.contains("draft"), "{error}");
        assert!(error.contains("exact ABI"), "{error}");
        assert!(error.contains("version"), "{error}");

        let valid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{ api \"web\" }}\npresets {{}}\nextensions {{ enable \"clap.webview/3\" version=\"3\" draft=#true }}\n"
        );
        build(&valid).expect("exact draft ABI should be accepted");
    }

    #[test]
    fn capability_dependencies_are_validated_and_reported() {
        let invalid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.note-expression\" }}\n"
        );
        let error = build(&invalid).expect_err("note expression requires note ports");
        assert!(error.contains("clap.note-expression"), "{error}");
        assert!(error.contains("note port"), "{error}");

        let valid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{ input \"notes\" id=\"notes-in\" dialects=\"clap\" preferred=\"clap\" }}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.note-expression\" }}\n"
        );
        let ir = build(&valid).expect("dependency should be satisfied");
        let report = capability_report_kdl(&ir);
        assert!(report.contains("clap.note-expression"));
        assert!(report.contains("stable"));
    }

    #[test]
    fn ir_serialization_has_a_versioned_compatibility_marker() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        );
        let ir = build(&source).expect("manifest should build");
        let serialized = serialize_ir_kdl(&ir);
        assert!(serialized.starts_with("ir version=1\n"), "{serialized}");
        assert_eq!(serialized, serialize_ir_kdl(&ir));
    }
}
