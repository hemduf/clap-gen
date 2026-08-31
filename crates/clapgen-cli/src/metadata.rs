use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode, KdlValue};

pub(crate) const DEFAULT_MANIFEST: &str = r#"// clap-gen metadata — KDL 2.0
clapgen schema="1.0.0"
plugin id="com.example.plugin" name="Plugin" vendor="Example" version="0.1.0"
processor class="PluginProcessor"
parameters {}
audio-ports {
    input "main" id="audio-in" channels=2
    output "main" id="audio-out" channels=2
}
note-ports {}
state {}
gui {}
presets {}
extensions {}
"#;

const ROOT_NODES: &[&str] = &[
    "clapgen",
    "import",
    "plugin",
    "processor",
    "parameters",
    "audio-ports",
    "note-ports",
    "state",
    "gui",
    "presets",
    "extensions",
];

#[derive(Debug, Clone)]
pub(crate) struct ParsedMetadata {
    pub(crate) document: KdlDocument,
    pub(crate) imports: Vec<PathBuf>,
}

pub(crate) fn parse_metadata(path: &Path, source: &str) -> Result<ParsedMetadata, String> {
    reject_legacy_input(path, source)?;

    let document = KdlDocument::parse_v2(source).map_err(|error| {
        format!(
            "{}:1: KDL 2.0 parse error: {error}\nhint: fix the syntax and run `clapgen fmt {}`",
            path.display(),
            path.display()
        )
    })?;

    validate_schema_marker(path, source, &document)?;
    let namespaces = collect_extension_namespaces(path, source, &document)?;
    validate_document(path, source, &document, &namespaces)?;
    let imports = collect_imports(path, source, &document)?;

    Ok(ParsedMetadata { document, imports })
}

pub(crate) fn format_metadata(path: &Path, source: &str) -> Result<String, String> {
    let mut parsed = parse_metadata(path, source)?;
    parsed.document.autoformat();
    Ok(parsed.document.to_string())
}

fn reject_legacy_input(path: &Path, source: &str) -> Result<(), String> {
    let extension = path.extension().and_then(|value| value.to_str());
    if matches!(extension, Some("yaml" | "yml")) || source.trim_start().starts_with("---") {
        return Err(format!(
            "{}:1: YAML metadata is not supported\nhint: migrate the manifest to canonical KDL 2.0 metadata",
            path.display()
        ));
    }

    if contains_legacy_kdl_literal(source) {
        return Err(format!(
            "{}:1: KDL 1-style literal detected\nhint: migrate KDL 1 literals to KDL 2.0 (`true` → `#true`, `false` → `#false`, `null` → `#null`)",
            path.display()
        ));
    }

    Ok(())
}

fn contains_legacy_kdl_literal(source: &str) -> bool {
    source
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, '=' | ';' | '{' | '}' | '(' | ')' | ',' | '\\')
        })
        .any(|token| matches!(token, "true" | "false" | "null"))
}

fn validate_schema_marker(path: &Path, source: &str, document: &KdlDocument) -> Result<(), String> {
    let Some(node) = document.get("clapgen") else {
        return Err(format!(
            "{}:1: missing `clapgen` schema marker\nhint: add `clapgen schema=\"1.0.0\"` as the first metadata node",
            path.display()
        ));
    };

    let Some(schema) = property_string(node, "schema") else {
        return Err(node_diagnostic(
            path,
            source,
            node.name().value(),
            "missing string property `schema`",
            "use `clapgen schema=\"1.0.0\"`",
        ));
    };

    if schema != "1.0.0" {
        return Err(node_diagnostic(
            path,
            source,
            node.name().value(),
            &format!("unsupported clap-gen metadata schema `{schema}`"),
            "use schema `1.0.0` or migrate with the matching clap-gen release",
        ));
    }

    Ok(())
}

fn collect_extension_namespaces(
    path: &Path,
    source: &str,
    document: &KdlDocument,
) -> Result<BTreeSet<String>, String> {
    let mut namespaces = BTreeSet::new();
    let Some(extensions) = document.get("extensions") else {
        return Ok(namespaces);
    };
    let Some(children) = extensions.children() else {
        return Ok(namespaces);
    };

    for node in children.nodes() {
        if node.name().value() != "namespace" {
            continue;
        }
        let Some(namespace) = first_string_argument(node) else {
            return Err(node_diagnostic(
                path,
                source,
                "namespace",
                "extension namespace requires one string argument",
                "declare it as `namespace \"vendor\"`",
            ));
        };
        if namespace.is_empty()
            || !namespace
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        {
            return Err(node_diagnostic(
                path,
                source,
                "namespace",
                &format!("invalid extension namespace `{namespace}`"),
                "use ASCII letters, digits, `-`, or `_`",
            ));
        }
        namespaces.insert(namespace.to_owned());
    }

    Ok(namespaces)
}

fn collect_imports(
    path: &Path,
    source: &str,
    document: &KdlDocument,
) -> Result<Vec<PathBuf>, String> {
    let mut imports = Vec::new();
    for node in document.nodes() {
        if node.name().value() != "import" {
            continue;
        }
        let Some(import) = first_string_argument(node) else {
            return Err(node_diagnostic(
                path,
                source,
                "import",
                "import requires one string path argument",
                "use `import \"relative/file.kdl\"`",
            ));
        };
        imports.push(PathBuf::from(import));
    }
    Ok(imports)
}

fn validate_document(
    path: &Path,
    source: &str,
    document: &KdlDocument,
    namespaces: &BTreeSet<String>,
) -> Result<(), String> {
    for node in document.nodes() {
        validate_node(path, source, node, None, namespaces)?;
    }
    Ok(())
}

fn validate_node(
    path: &Path,
    source: &str,
    node: &KdlNode,
    parent: Option<&str>,
    namespaces: &BTreeSet<String>,
) -> Result<(), String> {
    let name = node.name().value();
    if belongs_to_namespace(name, namespaces) {
        return Ok(());
    }

    let known = match parent {
        None => ROOT_NODES.contains(&name),
        Some(parent_name) => allowed_children(parent_name).contains(&name),
    };

    if !known {
        return Err(node_diagnostic(
            path,
            source,
            name,
            "unknown node",
            "remove the node or declare its prefix with `extensions { namespace \"vendor\" }`",
        ));
    }

    let allowed_properties = allowed_properties(parent, name);
    for entry in node.entries() {
        let Some(property) = entry.name() else {
            continue;
        };
        let property = property.value();
        if !allowed_properties.contains(&property) && !belongs_to_namespace(property, namespaces) {
            return Err(node_diagnostic(
                path,
                source,
                name,
                &format!("unknown property `{property}`"),
                "remove the property or qualify it with a declared extension namespace",
            ));
        }
    }

    if let Some(children) = node.children() {
        for child in children.nodes() {
            validate_node(path, source, child, Some(name), namespaces)?;
        }
    }

    Ok(())
}

fn allowed_children(node: &str) -> &'static [&'static str] {
    match node {
        "plugin" => &["feature"],
        "parameters" => &["param"],
        "audio-ports" => &["input", "output"],
        "note-ports" => &["input", "output", "note-name"],
        "state" => &["field"],
        "gui" => &["api", "resource"],
        "presets" => &["location", "format"],
        "extensions" => &["namespace", "enable"],
        _ => &[],
    }
}

fn allowed_properties(parent: Option<&str>, node: &str) -> &'static [&'static str] {
    match (parent, node) {
        (None, "clapgen") => &["schema"],
        (None, "import") => &["optional"],
        (None, "plugin") => &[
            "id",
            "name",
            "vendor",
            "version",
            "url",
            "manual-url",
            "support-url",
            "description",
        ],
        (None, "processor") => &["class", "features"],
        (Some("parameters"), "param") => &[
            "id", "name", "min", "max", "default", "flags", "unit", "steps",
        ],
        (Some("audio-ports"), "input" | "output") => &[
            "id",
            "name",
            "channels",
            "type",
            "flags",
            "in-place-pair",
        ],
        (Some("note-ports"), "input" | "output") => &[
            "id",
            "name",
            "dialects",
            "preferred",
        ],
        (Some("note-ports"), "note-name") => &["key", "channel", "port"],
        (Some("state"), "field") => &["name", "type", "default", "tag"],
        (Some("gui"), "api") => &["name", "floating", "embedded"],
        (Some("gui"), "resource") => &["path", "mime"],
        (Some("presets"), "location") => &["kind", "path"],
        (Some("presets"), "format") => &["extension", "mime"],
        (Some("extensions"), "enable") => &["id", "version", "draft"],
        _ => &[],
    }
}

fn belongs_to_namespace(value: &str, namespaces: &BTreeSet<String>) -> bool {
    namespaces.iter().any(|namespace| {
        value
            .strip_prefix(namespace)
            .is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with(':'))
    })
}

fn property_string<'a>(node: &'a KdlNode, key: &str) -> Option<&'a str> {
    node.entries().iter().rev().find_map(|entry| {
        let name = entry.name()?;
        if name.value() != key {
            return None;
        }
        match entry.value() {
            KdlValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    })
}

fn first_string_argument(node: &KdlNode) -> Option<&str> {
    node.entries().iter().find_map(|entry| {
        if entry.name().is_some() {
            return None;
        }
        match entry.value() {
            KdlValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    })
}

fn node_diagnostic(
    path: &Path,
    source: &str,
    node: &str,
    message: &str,
    hint: &str,
) -> String {
    let line = find_node_line(source, node);
    format!(
        "{}:{line}: node `{node}`: {message}\nhint: {hint}",
        path.display()
    )
}

fn find_node_line(source: &str, node: &str) -> usize {
    source
        .lines()
        .enumerate()
        .find_map(|(index, line)| {
            let line = line.trim_start();
            let unquoted = line
                .strip_prefix(node)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with('{'));
            let quoted_name = format!("\"{node}\"");
            let quoted = line.strip_prefix(&quoted_name).is_some_and(|rest| {
                rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with('{')
            });
            (unquoted || quoted).then_some(index + 1)
        })
        .unwrap_or(1)
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

    #[test]
    fn rejects_missing_or_future_schema_markers() {
        let missing = "plugin id=\"x\" name=\"x\" vendor=\"x\" version=\"1\"\n";
        let error = parse_metadata(Path::new("plugin.kdl"), missing).expect_err("schema marker required");
        assert!(error.contains("missing `clapgen` schema marker"), "{error}");

        let future = "clapgen schema=\"2.0.0\"\nplugin id=\"x\" name=\"x\" vendor=\"x\" version=\"1\"\n";
        let error = parse_metadata(Path::new("plugin.kdl"), future).expect_err("unknown schema must fail");
        assert!(error.contains("unsupported clap-gen metadata schema"), "{error}");
    }
}
