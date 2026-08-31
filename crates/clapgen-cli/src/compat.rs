use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode, KdlValue};

use crate::ir::{CanonicalIr, serialize_ir_kdl};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    Compatible,
    Sensitive,
    Forbidden,
}

impl Class {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Sensitive => "sensitive",
            Self::Forbidden => "forbidden",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    class: Class,
    subject: String,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Report {
    changes: Vec<Change>,
}

impl Report {
    pub(crate) fn has_forbidden(&self) -> bool {
        self.changes.iter().any(|change| change.class == Class::Forbidden)
    }

    pub(crate) fn text(&self) -> String {
        if self.changes.is_empty() {
            return "compatible: no semantic compatibility changes".to_owned();
        }
        self.changes
            .iter()
            .map(|change| format!("{} {}: {}", change.class.as_str(), change.subject, change.detail))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn json(&self) -> String {
        let body = self
            .changes
            .iter()
            .map(|change| {
                format!(
                    "{{\"class\":{},\"subject\":{},\"detail\":{}}}",
                    json_string(change.class.as_str()),
                    json_string(&change.subject),
                    json_string(&change.detail)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"changes\":[{body}],\"forbidden\":{}}}", self.has_forbidden())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Parameter {
    min: String,
    max: String,
    default: String,
    flags: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Port {
    direction: String,
    channels: Option<String>,
    dialects: Option<String>,
    preferred: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Snapshot {
    plugin_id: String,
    parameters: BTreeMap<String, Parameter>,
    audio_ports: BTreeMap<String, Port>,
    note_ports: BTreeMap<String, Port>,
    state_fields: BTreeMap<String, String>,
    draft_extensions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryEntry {
    kind: String,
    key: String,
    value: u32,
    tombstone: bool,
}

pub(crate) fn compare(
    baseline: &CanonicalIr,
    current: &CanonicalIr,
    baseline_manifest: &Path,
    current_manifest: &Path,
) -> Result<Report, String> {
    let baseline = snapshot(baseline)?;
    let current = snapshot(current)?;
    let mut changes = Vec::new();

    if baseline.plugin_id != current.plugin_id {
        changes.push(Change {
            class: Class::Forbidden,
            subject: "plugin.id".to_owned(),
            detail: format!("changed from `{}` to `{}`", baseline.plugin_id, current.plugin_id),
        });
    }

    compare_parameters(&baseline.parameters, &current.parameters, &mut changes);
    compare_ports("audio-port", &baseline.audio_ports, &current.audio_ports, &mut changes);
    compare_ports("note-port", &baseline.note_ports, &current.note_ports, &mut changes);
    compare_state(&baseline.state_fields, &current.state_fields, &mut changes);
    compare_drafts(&baseline.draft_extensions, &current.draft_extensions, &mut changes);
    compare_id_registries(baseline_manifest, current_manifest, &mut changes)?;

    changes.sort_by(|a, b| (&a.subject, a.class, &a.detail).cmp(&(&b.subject, b.class, &b.detail)));
    Ok(Report { changes })
}

fn compare_parameters(
    baseline: &BTreeMap<String, Parameter>,
    current: &BTreeMap<String, Parameter>,
    changes: &mut Vec<Change>,
) {
    for (id, old) in baseline {
        let Some(new) = current.get(id) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("parameter.{id}"),
                detail: "released parameter was removed".to_owned(),
            });
            continue;
        };
        if old.min != new.min || old.max != new.max {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{id}.range"),
                detail: format!("changed from {}..{} to {}..{}", old.min, old.max, new.min, new.max),
            });
        }
        if old.default != new.default {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{id}.default"),
                detail: format!("changed from {} to {}", old.default, new.default),
            });
        }
        if old.flags != new.flags {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{id}.flags"),
                detail: format!("changed from `{}` to `{}`", old.flags, new.flags),
            });
        }
    }
    for id in current.keys().filter(|id| !baseline.contains_key(*id)) {
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("parameter.{id}"),
            detail: "new parameter added".to_owned(),
        });
    }
}

fn compare_ports(
    kind: &str,
    baseline: &BTreeMap<String, Port>,
    current: &BTreeMap<String, Port>,
    changes: &mut Vec<Change>,
) {
    for (id, old) in baseline {
        let Some(new) = current.get(id) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("{kind}.{id}"),
                detail: "released port was removed".to_owned(),
            });
            continue;
        };
        if old != new {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("{kind}.{id}"),
                detail: "released port topology changed".to_owned(),
            });
        }
    }
    for id in current.keys().filter(|id| !baseline.contains_key(*id)) {
        changes.push(Change {
            class: Class::Sensitive,
            subject: format!("{kind}.{id}"),
            detail: "new port changes host-visible topology".to_owned(),
        });
    }
}

fn compare_state(
    baseline: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
    changes: &mut Vec<Change>,
) {
    for (identity, old_name) in baseline {
        let Some(new_name) = current.get(identity) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("state.{identity}"),
                detail: "released persistent state tag was removed or changed".to_owned(),
            });
            continue;
        };
        if old_name != new_name {
            changes.push(Change {
                class: Class::Compatible,
                subject: format!("state.{identity}"),
                detail: format!("symbol renamed from `{old_name}` to `{new_name}` while preserving its persistent tag"),
            });
        }
    }
    for (identity, name) in current.iter().filter(|(identity, _)| !baseline.contains_key(*identity)) {
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("state.{identity}"),
            detail: format!("new state field `{name}` added"),
        });
    }
}

fn compare_drafts(
    baseline: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
    changes: &mut Vec<Change>,
) {
    for (id, version) in baseline {
        match current.get(id) {
            Some(current_version) if current_version == version => {}
            Some(current_version) => changes.push(Change {
                class: Class::Forbidden,
                subject: format!("draft.{id}"),
                detail: format!("ABI version changed from `{version}` to `{current_version}`"),
            }),
            None => changes.push(Change {
                class: Class::Forbidden,
                subject: format!("draft.{id}"),
                detail: "released draft ABI capability was removed or replaced".to_owned(),
            }),
        }
    }
    for (id, version) in current.iter().filter(|(id, _)| !baseline.contains_key(*id)) {
        changes.push(Change {
            class: Class::Sensitive,
            subject: format!("draft.{id}"),
            detail: format!("new draft ABI capability version `{version}`"),
        });
    }
}

fn compare_id_registries(
    baseline_manifest: &Path,
    current_manifest: &Path,
    changes: &mut Vec<Change>,
) -> Result<(), String> {
    let baseline_path = sibling_registry(baseline_manifest);
    if !baseline_path.exists() {
        return Ok(());
    }
    let current_path = sibling_registry(current_manifest);
    if !current_path.exists() {
        changes.push(Change {
            class: Class::Forbidden,
            subject: "clap-ids.registry".to_owned(),
            detail: "released plugin.ids.kdl registry is missing from current baseline".to_owned(),
        });
        return Ok(());
    }

    let baseline = read_registry(&baseline_path)?;
    let current = read_registry(&current_path)?;
    let old_by_value = baseline.iter().map(|entry| (entry.value, entry)).collect::<BTreeMap<_, _>>();
    let new_by_value = current.iter().map(|entry| (entry.value, entry)).collect::<BTreeMap<_, _>>();
    let old_by_symbol = baseline
        .iter()
        .map(|entry| ((entry.kind.as_str(), entry.key.as_str()), entry.value))
        .collect::<BTreeMap<_, _>>();
    let new_by_symbol = current
        .iter()
        .map(|entry| ((entry.kind.as_str(), entry.key.as_str()), entry.value))
        .collect::<BTreeMap<_, _>>();

    for (symbol, old_value) in &old_by_symbol {
        if let Some(new_value) = new_by_symbol.get(symbol)
            && new_value != old_value
        {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("clap-id.{}:{}", symbol.0, symbol.1),
                detail: format!("numeric ID changed from `{old_value}` to `{new_value}`"),
            });
        }
    }

    for (value, old) in &old_by_value {
        let Some(new) = new_by_value.get(value) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("clap-id.{value}"),
                detail: format!("released numeric ID for `{}:{}` disappeared", old.kind, old.key),
            });
            continue;
        };
        if old.kind != new.kind {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("clap-id.{value}"),
                detail: format!("ID kind changed from `{}` to `{}`", old.kind, new.kind),
            });
        }
        if old.tombstone && !new.tombstone {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("clap-id.{value}"),
                detail: "permanent tombstone was resurrected".to_owned(),
            });
        } else if !old.tombstone && new.tombstone {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("clap-id.{value}"),
                detail: format!("released ID `{}:{}` was retired as a tombstone", old.kind, old.key),
            });
        } else if old.key != new.key && old.kind == new.kind {
            changes.push(Change {
                class: Class::Compatible,
                subject: format!("clap-id.{value}"),
                detail: format!("symbol renamed from `{}` to `{}` with numeric ID preserved", old.key, new.key),
            });
        }
    }

    for (value, entry) in new_by_value.iter().filter(|(value, _)| !old_by_value.contains_key(value)) {
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("clap-id.{value}"),
            detail: format!("new numeric ID allocated to `{}:{}`", entry.kind, entry.key),
        });
    }
    Ok(())
}

fn sibling_registry(manifest: &Path) -> PathBuf {
    manifest.parent().unwrap_or_else(|| Path::new(".")).join("plugin.ids.kdl")
}

fn read_registry(path: &Path) -> Result<Vec<RegistryEntry>, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{}`: {error}", path.display()))?;
    let document = KdlDocument::parse_v2(&source)
        .map_err(|error| format!("{}: invalid KDL 2.0 ID registry: {error}", path.display()))?;
    let root = document.get("ids").ok_or_else(|| format!("{}: missing `ids` root", path.display()))?;
    let version = integer_prop(root, "version").ok_or_else(|| format!("{}: missing registry version", path.display()))?;
    if version != 1 {
        return Err(format!("{}: unsupported registry version `{version}`", path.display()));
    }
    let mut entries = Vec::new();
    if let Some(children) = root.children() {
        for node in children.nodes().iter().filter(|node| node.name().value() == "entry") {
            let kind = required_string(node, "kind")?;
            let key = required_string(node, "key")?;
            let value = integer_prop(node, "value")
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| format!("{}: invalid registry ID value", path.display()))?;
            let tombstone = bool_prop(node, "tombstone").unwrap_or(false);
            entries.push(RegistryEntry { kind, key, value, tombstone });
        }
    }
    entries.sort_by_key(|entry| entry.value);
    Ok(entries)
}

fn snapshot(ir: &CanonicalIr) -> Result<Snapshot, String> {
    let source = serialize_ir_kdl(ir);
    let document = KdlDocument::parse_v2(&source)
        .map_err(|error| format!("internal canonical IR parse failed: {error}"))?;
    let plugin = document.get("plugin").ok_or_else(|| "internal IR is missing plugin".to_owned())?;
    let plugin_id = required_string(plugin, "id")?;
    let mut result = Snapshot { plugin_id, ..Snapshot::default() };

    if let Some(parameters) = document.get("parameters").and_then(KdlNode::children) {
        for node in parameters.nodes().iter().filter(|node| node.name().value() == "param") {
            let id = required_string(node, "id")?;
            result.parameters.insert(
                id,
                Parameter {
                    min: value_prop(node, "min")?,
                    max: value_prop(node, "max")?,
                    default: value_prop(node, "default")?,
                    flags: string_prop(node, "flags").unwrap_or_default().to_owned(),
                },
            );
        }
    }
    collect_ports(document.get("audio-ports"), false, &mut result.audio_ports)?;
    collect_ports(document.get("note-ports"), true, &mut result.note_ports)?;

    if let Some(state) = document.get("state").and_then(KdlNode::children) {
        for node in state.nodes().iter().filter(|node| node.name().value() == "field") {
            let name = first_string(node).ok_or_else(|| "internal state field missing name".to_owned())?;
            let identity = string_prop(node, "tag").unwrap_or(name).to_owned();
            result.state_fields.insert(identity, name.to_owned());
        }
    }
    if let Some(extensions) = document.get("extensions").and_then(KdlNode::children) {
        for node in extensions.nodes().iter().filter(|node| node.name().value() == "draft") {
            let id = first_string(node).ok_or_else(|| "internal draft extension missing id".to_owned())?;
            result.draft_extensions.insert(id.to_owned(), required_string(node, "version")?);
        }
    }
    Ok(result)
}

fn collect_ports(
    root: Option<&KdlNode>,
    note: bool,
    output: &mut BTreeMap<String, Port>,
) -> Result<(), String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok(());
    };
    for node in children.nodes().iter().filter(|node| matches!(node.name().value(), "input" | "output")) {
        let id = required_string(node, "id")?;
        output.insert(
            id,
            Port {
                direction: node.name().value().to_owned(),
                channels: (!note).then(|| value_prop(node, "channels")).transpose()?,
                dialects: note.then(|| string_prop(node, "dialects").unwrap_or_default().to_owned()),
                preferred: note.then(|| string_prop(node, "preferred").unwrap_or_default().to_owned()),
            },
        );
    }
    Ok(())
}

fn prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a KdlValue> {
    node.entries().iter().rev().find_map(|entry| {
        let name = entry.name()?;
        (name.value() == key).then_some(entry.value())
    })
}

fn required_string(node: &KdlNode, key: &str) -> Result<String, String> {
    string_prop(node, key)
        .map(str::to_owned)
        .ok_or_else(|| format!("internal IR node `{}` missing `{key}`", node.name().value()))
}

fn string_prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a str> {
    match prop(node, key)? {
        KdlValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn integer_prop(node: &KdlNode, key: &str) -> Option<i128> {
    match prop(node, key)? {
        KdlValue::Integer(value) => Some(*value),
        _ => None,
    }
}

fn bool_prop(node: &KdlNode, key: &str) -> Option<bool> {
    match prop(node, key)? {
        KdlValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn value_prop(node: &KdlNode, key: &str) -> Result<String, String> {
    match prop(node, key) {
        Some(KdlValue::Integer(value)) => Ok(value.to_string()),
        Some(KdlValue::Float(value)) => Ok(value.to_string()),
        Some(value) => Err(format!("internal IR `{key}` has unexpected value `{value:?}`")),
        None => Err(format!("internal IR node `{}` missing `{key}`", node.name().value())),
    }
}

fn first_string(node: &KdlNode) -> Option<&str> {
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

fn json_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::ir::build_ir;
    use crate::metadata::parse_metadata;

    use super::{Class, compare};

    fn temp_dir() -> PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        env::temp_dir().join(format!("clapgen-compat-{}-{nonce}", std::process::id()))
    }

    fn ir(source: &str) -> crate::ir::CanonicalIr {
        let path = Path::new("plugin.kdl");
        let metadata = parse_metadata(path, source).expect("metadata");
        build_ir(path, source, &metadata).expect("IR")
    }

    fn manifest(parameter: &str, audio: &str, state: &str, extensions: &str) -> String {
        format!(
            "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0\"\nprocessor class=\"P\"\nparameters {{ {parameter} }}\naudio-ports {{ {audio} }}\nnote-ports {{}}\nstate {{ {state} }}\ngui {{ api \"web\" }}\npresets {{}}\nfactories {{}}\nextensions {{ {extensions} }}\n"
        )
    }

    #[test]
    fn state_symbol_rename_preserves_persistent_tag() {
        let baseline_source = manifest("", "", "field \"old-name\" type=\"string\" tag=\"state-1\"", "");
        let current_source = manifest("", "", "field \"new-name\" type=\"string\" tag=\"state-1\"", "");
        let baseline = ir(&baseline_source);
        let current = ir(&current_source);
        let report = compare(&baseline, &current, Path::new("baseline/plugin.kdl"), Path::new("current/plugin.kdl")).expect("compare");
        assert!(!report.has_forbidden(), "{}", report.text());
        assert!(report.text().contains("symbol renamed"), "{}", report.text());
    }

    #[test]
    fn numeric_registry_id_change_is_forbidden_but_symbol_rename_is_compatible() {
        let dir = temp_dir();
        let old_dir = dir.join("old");
        let new_dir = dir.join("new");
        fs::create_dir_all(&old_dir).expect("old dir");
        fs::create_dir_all(&new_dir).expect("new dir");
        fs::write(old_dir.join("plugin.ids.kdl"), "ids version=1 next=2 { entry kind=\"parameter\" key=\"cutoff\" value=1 tombstone=#false }\n").expect("old registry");
        fs::write(new_dir.join("plugin.ids.kdl"), "ids version=1 next=2 { entry kind=\"parameter\" key=\"filter-cutoff\" value=1 tombstone=#false }\n").expect("new registry");
        let source = manifest("", "", "", "");
        let baseline = ir(&source);
        let current = ir(&source);
        let report = compare(&baseline, &current, &old_dir.join("plugin.kdl"), &new_dir.join("plugin.kdl")).expect("compare");
        assert!(!report.has_forbidden(), "{}", report.text());
        assert!(report.text().contains("clap-id.1"), "{}", report.text());
        assert!(report.text().contains("numeric ID preserved"), "{}", report.text());

        fs::write(new_dir.join("plugin.ids.kdl"), "ids version=1 next=3 { entry kind=\"parameter\" key=\"cutoff\" value=2 tombstone=#false }\n").expect("new registry changed");
        let report = compare(&baseline, &current, &old_dir.join("plugin.kdl"), &new_dir.join("plugin.kdl")).expect("compare changed");
        assert!(report.has_forbidden());
        assert!(report.text().contains("numeric ID changed"), "{}", report.text());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn classifies_parameter_and_topology_changes() {
        let baseline_source = manifest(
            "param \"Gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=\"automatable\"",
            "output \"Main\" id=\"out\" channels=2",
            "",
            "",
        );
        let current_source = manifest(
            "param \"Gain\" id=\"gain\" min=0.0 max=2.0 default=0.75 flags=\"automatable,modulatable\"; param \"Mix\" id=\"mix\" min=0.0 max=1.0 default=1.0",
            "",
            "",
            "",
        );
        let report = compare(&ir(&baseline_source), &ir(&current_source), Path::new("baseline/plugin.kdl"), Path::new("current/plugin.kdl")).expect("compare");
        assert!(report.text().contains("sensitive parameter.gain.range"));
        assert!(report.text().contains("forbidden audio-port.out"));
        assert!(report.changes.iter().any(|change| change.class == Class::Forbidden));
        assert_eq!(report.json(), report.json());
    }
}