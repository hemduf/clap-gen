use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode, KdlValue};

use crate::ids::{RegistryEntry, read_entries};
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
            .map(|change| {
                format!("{} {}: {}", change.class.as_str(), change.subject, change.detail)
            })
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
    media_type: Option<String>,
    flags: String,
    in_place_pair: Option<String>,
    dialects: Option<String>,
    preferred: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StateField {
    tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Snapshot {
    plugin_id: String,
    parameters: BTreeMap<String, Parameter>,
    audio_ports: BTreeMap<String, Port>,
    note_ports: BTreeMap<String, Port>,
    state_fields: BTreeMap<String, StateField>,
    draft_extensions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryRename {
    kind: String,
    from: String,
    to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct RegistryRelation {
    renames: Vec<RegistryRename>,
    current: Option<Vec<RegistryEntry>>,
}

impl RegistryRelation {
    fn renamed_symbol<'a>(&'a self, kinds: &[&str], from: &str) -> Option<&'a str> {
        self.renames
            .iter()
            .find(|rename| kinds.contains(&rename.kind.as_str()) && rename.from == from)
            .map(|rename| rename.to.as_str())
    }

    fn is_rename_target(&self, kinds: &[&str], target: &str) -> bool {
        self.renames
            .iter()
            .any(|rename| kinds.contains(&rename.kind.as_str()) && rename.to == target)
    }
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
    let relation = compare_id_registries(baseline_manifest, current_manifest, &mut changes)?;

    if baseline.plugin_id != current.plugin_id {
        changes.push(Change {
            class: Class::Forbidden,
            subject: "plugin.id".to_owned(),
            detail: format!("changed from `{}` to `{}`", baseline.plugin_id, current.plugin_id),
        });
    }

    compare_parameters(&baseline.parameters, &current.parameters, &relation, &mut changes);
    compare_ports(
        "audio-port",
        &["audio-port", "port"],
        &baseline.audio_ports,
        &current.audio_ports,
        &relation,
        &mut changes,
    );
    compare_ports(
        "note-port",
        &["note-port", "port"],
        &baseline.note_ports,
        &current.note_ports,
        &relation,
        &mut changes,
    );
    compare_state(&baseline.state_fields, &current.state_fields, &relation, &mut changes);
    compare_drafts(&baseline.draft_extensions, &current.draft_extensions, &mut changes);
    validate_current_registry_coverage(&current, &relation, &mut changes);

    changes.sort_by(|a, b| (&a.subject, a.class, &a.detail).cmp(&(&b.subject, b.class, &b.detail)));
    Ok(Report { changes })
}

fn compare_parameters(
    baseline: &BTreeMap<String, Parameter>,
    current: &BTreeMap<String, Parameter>,
    relation: &RegistryRelation,
    changes: &mut Vec<Change>,
) {
    const KINDS: &[&str] = &["parameter"];
    for (id, old) in baseline {
        let Some((current_id, new)) = resolve_current(id, current, relation, KINDS) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("parameter.{id}"),
                detail: "released parameter was removed".to_owned(),
            });
            continue;
        };
        if current_id != id {
            changes.push(Change {
                class: Class::Compatible,
                subject: format!("parameter.{id}"),
                detail: format!(
                    "symbol renamed to `{current_id}` while preserving its numeric CLAP ID"
                ),
            });
        }
        if old.min != new.min || old.max != new.max {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{current_id}.range"),
                detail: format!(
                    "changed from {}..{} to {}..{}",
                    old.min, old.max, new.min, new.max
                ),
            });
        }
        if old.default != new.default {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{current_id}.default"),
                detail: format!("changed from {} to {}", old.default, new.default),
            });
        }
        if old.flags != new.flags {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{current_id}.flags"),
                detail: format!("changed from `{}` to `{}`", old.flags, new.flags),
            });
        }
    }

    for id in current.keys() {
        if baseline.contains_key(id) || relation.is_rename_target(KINDS, id) {
            continue;
        }
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("parameter.{id}"),
            detail: "new parameter added".to_owned(),
        });
    }
}

fn compare_ports(
    label: &str,
    registry_kinds: &[&str],
    baseline: &BTreeMap<String, Port>,
    current: &BTreeMap<String, Port>,
    relation: &RegistryRelation,
    changes: &mut Vec<Change>,
) {
    for (id, old) in baseline {
        let Some((current_id, new)) = resolve_current(id, current, relation, registry_kinds) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("{label}.{id}"),
                detail: "released port was removed".to_owned(),
            });
            continue;
        };
        if current_id != id {
            changes.push(Change {
                class: Class::Compatible,
                subject: format!("{label}.{id}"),
                detail: format!(
                    "symbol renamed to `{current_id}` while preserving its numeric CLAP ID"
                ),
            });
        }
        if !ports_equivalent(old, new, relation, registry_kinds) {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("{label}.{current_id}"),
                detail: "released port topology changed".to_owned(),
            });
        }
    }

    for id in current.keys() {
        if baseline.contains_key(id) || relation.is_rename_target(registry_kinds, id) {
            continue;
        }
        changes.push(Change {
            class: Class::Sensitive,
            subject: format!("{label}.{id}"),
            detail: "new port changes host-visible topology".to_owned(),
        });
    }
}

fn ports_equivalent(
    old: &Port,
    new: &Port,
    relation: &RegistryRelation,
    registry_kinds: &[&str],
) -> bool {
    old.direction == new.direction
        && old.channels == new.channels
        && old.media_type == new.media_type
        && old.flags == new.flags
        && pair_equivalent(
            old.in_place_pair.as_deref(),
            new.in_place_pair.as_deref(),
            relation,
            registry_kinds,
        )
        && old.dialects == new.dialects
        && old.preferred == new.preferred
}

fn pair_equivalent(
    old: Option<&str>,
    new: Option<&str>,
    relation: &RegistryRelation,
    registry_kinds: &[&str],
) -> bool {
    match (old, new) {
        (None, None) => true,
        (Some(old), Some(new)) => {
            old == new || relation.renamed_symbol(registry_kinds, old) == Some(new)
        }
        _ => false,
    }
}

fn compare_state(
    baseline: &BTreeMap<String, StateField>,
    current: &BTreeMap<String, StateField>,
    relation: &RegistryRelation,
    changes: &mut Vec<Change>,
) {
    const KINDS: &[&str] = &["state", "state-field"];
    for (name, old) in baseline {
        let matched = if let Some(tag) = old.tag.as_deref() {
            current
                .iter()
                .find(|(_, field)| field.tag.as_deref() == Some(tag))
                .map(|(current_name, field)| (current_name.as_str(), field))
        } else {
            resolve_current(name, current, relation, KINDS)
        };

        let Some((current_name, new)) = matched else {
            let subject = old.tag.as_deref().unwrap_or(name);
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("state.{subject}"),
                detail: "released persistent state tag/identity was removed or changed".to_owned(),
            });
            continue;
        };
        if current_name != name {
            changes.push(Change {
                class: Class::Compatible,
                subject: format!("state.{}", old.tag.as_deref().unwrap_or(name)),
                detail: format!(
                    "symbol renamed from `{name}` to `{current_name}` while preserving its persistent identity"
                ),
            });
        }
        if old.tag != new.tag {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("state.{name}.tag"),
                detail: format!("changed from {:?} to {:?}", old.tag, new.tag),
            });
        }
    }

    for (name, field) in current {
        let already_present = match field.tag.as_deref() {
            Some(tag) => baseline.values().any(|old| old.tag.as_deref() == Some(tag)),
            None => baseline.contains_key(name) || relation.is_rename_target(KINDS, name),
        };
        if already_present {
            continue;
        }
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("state.{}", field.tag.as_deref().unwrap_or(name)),
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
    for (id, version) in current {
        if baseline.contains_key(id) {
            continue;
        }
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
) -> Result<RegistryRelation, String> {
    let baseline = read_entries(&sibling_registry(baseline_manifest))?;
    let current = read_entries(&sibling_registry(current_manifest))?;
    let mut relation = RegistryRelation { renames: Vec::new(), current: current.clone() };

    let Some(baseline) = baseline else {
        if current.is_some() {
            changes.push(Change {
                class: Class::Compatible,
                subject: "clap-ids.registry".to_owned(),
                detail: "versioned persistent ID registry introduced".to_owned(),
            });
        }
        return Ok(relation);
    };
    let Some(current) = current else {
        changes.push(Change {
            class: Class::Forbidden,
            subject: "clap-ids.registry".to_owned(),
            detail: "released plugin.ids.kdl registry is missing from current baseline".to_owned(),
        });
        return Ok(relation);
    };

    compare_registry_symbols(&baseline, &current, changes);
    compare_released_registry_ids(&baseline, &current, &mut relation, changes);
    compare_new_registry_ids(&baseline, &current, changes);
    Ok(relation)
}

fn compare_registry_symbols(
    baseline: &[RegistryEntry],
    current: &[RegistryEntry],
    changes: &mut Vec<Change>,
) {
    for old in baseline {
        let Some(new) = current.iter().find(|new| new.kind == old.kind && new.key == old.key)
        else {
            continue;
        };
        if new.value != old.value {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("clap-id.{}:{}", old.kind, old.key),
                detail: format!("numeric ID changed from `{}` to `{}`", old.value, new.value),
            });
        }
    }
}

fn compare_released_registry_ids(
    baseline: &[RegistryEntry],
    current: &[RegistryEntry],
    relation: &mut RegistryRelation,
    changes: &mut Vec<Change>,
) {
    for old in baseline {
        let Some(new) = current.iter().find(|new| new.value == old.value) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("clap-id.{}", old.value),
                detail: format!("released numeric ID for `{}:{}` disappeared", old.kind, old.key),
            });
            continue;
        };
        compare_released_registry_id(old, new, relation, changes);
    }
}

fn compare_released_registry_id(
    old: &RegistryEntry,
    new: &RegistryEntry,
    relation: &mut RegistryRelation,
    changes: &mut Vec<Change>,
) {
    if old.kind != new.kind {
        changes.push(Change {
            class: Class::Forbidden,
            subject: format!("clap-id.{}", old.value),
            detail: format!("ID kind changed from `{}` to `{}`", old.kind, new.kind),
        });
        return;
    }
    if old.tombstone && !new.tombstone {
        changes.push(Change {
            class: Class::Forbidden,
            subject: format!("clap-id.{}", old.value),
            detail: "permanent tombstone was resurrected".to_owned(),
        });
    } else if !old.tombstone && new.tombstone {
        changes.push(Change {
            class: Class::Sensitive,
            subject: format!("clap-id.{}", old.value),
            detail: format!("released ID `{}:{}` was retired as a tombstone", old.kind, old.key),
        });
    } else if old.key != new.key {
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("clap-id.{}", old.value),
            detail: format!(
                "symbol renamed from `{}` to `{}` with numeric ID preserved",
                old.key, new.key
            ),
        });
        if !old.tombstone {
            relation.renames.push(RegistryRename {
                kind: old.kind.clone(),
                from: old.key.clone(),
                to: new.key.clone(),
            });
        }
    }
}

fn compare_new_registry_ids(
    baseline: &[RegistryEntry],
    current: &[RegistryEntry],
    changes: &mut Vec<Change>,
) {
    for new in current {
        let existing_value = baseline.iter().any(|old| old.value == new.value);
        let existing_symbol = baseline.iter().any(|old| old.kind == new.kind && old.key == new.key);
        if existing_value || existing_symbol {
            continue;
        }
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("clap-id.{}", new.value),
            detail: format!("new numeric ID allocated to `{}:{}`", new.kind, new.key),
        });
    }
}

fn validate_current_registry_coverage(
    current: &Snapshot,
    relation: &RegistryRelation,
    changes: &mut Vec<Change>,
) {
    let Some(entries) = relation.current.as_deref() else {
        return;
    };
    for id in current.parameters.keys() {
        require_active_id(entries, &["parameter"], id, "parameter", changes);
    }
    for id in current.audio_ports.keys() {
        require_active_id(entries, &["audio-port", "port"], id, "audio-port", changes);
    }
    for id in current.note_ports.keys() {
        require_active_id(entries, &["note-port", "port"], id, "note-port", changes);
    }
    for (name, field) in &current.state_fields {
        if field.tag.is_none() {
            require_active_id(entries, &["state", "state-field"], name, "state", changes);
        }
    }
}

fn require_active_id(
    entries: &[RegistryEntry],
    kinds: &[&str],
    key: &str,
    label: &str,
    changes: &mut Vec<Change>,
) {
    if entries
        .iter()
        .any(|entry| kinds.contains(&entry.kind.as_str()) && entry.key == key && !entry.tombstone)
    {
        return;
    }
    changes.push(Change {
        class: Class::Forbidden,
        subject: format!("clap-id.{label}:{key}"),
        detail: "host-visible persistent entity has no active numeric ID allocation".to_owned(),
    });
}

fn resolve_current<'a, T>(
    baseline_id: &str,
    current: &'a BTreeMap<String, T>,
    relation: &RegistryRelation,
    registry_kinds: &[&str],
) -> Option<(&'a str, &'a T)> {
    if let Some((key, value)) = current.get_key_value(baseline_id) {
        return Some((key.as_str(), value));
    }
    let renamed = relation.renamed_symbol(registry_kinds, baseline_id)?;
    current.get_key_value(renamed).map(|(key, value)| (key.as_str(), value))
}

fn sibling_registry(manifest: &Path) -> PathBuf {
    manifest.parent().unwrap_or_else(|| Path::new(".")).join("plugin.ids.kdl")
}

fn snapshot(ir: &CanonicalIr) -> Result<Snapshot, String> {
    let source = serialize_ir_kdl(ir);
    let document = KdlDocument::parse_v2(&source)
        .map_err(|error| format!("internal canonical IR parse failed: {error}"))?;
    let plugin =
        document.get("plugin").ok_or_else(|| "internal IR is missing plugin".to_owned())?;
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
        let mut persistent_tags = BTreeMap::new();
        for node in state.nodes().iter().filter(|node| node.name().value() == "field") {
            let name = first_string(node)
                .ok_or_else(|| "internal state field missing name".to_owned())?
                .to_owned();
            let tag = string_prop(node, "tag").map(str::to_owned);
            if let Some(tag) = tag.as_deref()
                && let Some(previous) = persistent_tags.insert(tag.to_owned(), name.clone())
            {
                return Err(format!(
                    "duplicate persistent state tag `{tag}` used by `{previous}` and `{name}`"
                ));
            }
            result.state_fields.insert(name, StateField { tag });
        }
    }
    if let Some(extensions) = document.get("extensions").and_then(KdlNode::children) {
        for node in extensions.nodes().iter().filter(|node| node.name().value() == "draft") {
            let id = first_string(node)
                .ok_or_else(|| "internal draft extension missing id".to_owned())?;
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
    for node in
        children.nodes().iter().filter(|node| matches!(node.name().value(), "input" | "output"))
    {
        let id = required_string(node, "id")?;
        output.insert(
            id,
            Port {
                direction: node.name().value().to_owned(),
                channels: (!note).then(|| value_prop(node, "channels")).transpose()?,
                media_type: if note { None } else { string_prop(node, "type").map(str::to_owned) },
                flags: if note {
                    String::new()
                } else {
                    string_prop(node, "flags").unwrap_or_default().to_owned()
                },
                in_place_pair: if note {
                    None
                } else {
                    string_prop(node, "in-place-pair").map(str::to_owned)
                },
                dialects: note
                    .then(|| string_prop(node, "dialects").unwrap_or_default().to_owned()),
                preferred: note
                    .then(|| string_prop(node, "preferred").unwrap_or_default().to_owned()),
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
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value <= '\u{1f}' => {
                write!(&mut escaped, "\\u{:04x}", u32::from(value))
                    .expect("String write cannot fail");
            }
            value => escaped.push(value),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::ir::build_ir;
    use crate::metadata::parse_metadata;

    use super::{Class, compare, json_string};

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

    fn write_registry(directory: &Path, source: &str) {
        fs::create_dir_all(directory).expect("registry directory");
        fs::write(directory.join("plugin.ids.kdl"), source).expect("registry write");
    }

    #[test]
    fn parameter_symbol_rename_uses_persistent_numeric_id() {
        let directory = temp_dir();
        let old_dir = directory.join("old");
        let new_dir = directory.join("new");
        write_registry(
            &old_dir,
            "ids version=1 next=2 { entry kind=\"parameter\" key=\"gain\" value=1 tombstone=#false }\n",
        );
        write_registry(
            &new_dir,
            "ids version=1 next=2 { entry kind=\"parameter\" key=\"level\" value=1 tombstone=#false }\n",
        );
        let baseline =
            ir(&manifest("param \"Gain\" id=\"gain\" min=0.0 max=1.0 default=0.5", "", "", ""));
        let current =
            ir(&manifest("param \"Level\" id=\"level\" min=0.0 max=1.0 default=0.5", "", "", ""));
        let report =
            compare(&baseline, &current, &old_dir.join("plugin.kdl"), &new_dir.join("plugin.kdl"))
                .expect("compare");
        assert!(!report.has_forbidden(), "{}", report.text());
        assert!(report.text().contains("numeric CLAP ID"), "{}", report.text());
        assert!(report.text().contains("gain"), "{}", report.text());
        assert!(report.text().contains("level"), "{}", report.text());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn audio_topology_includes_type_flags_and_in_place_pair() {
        let baseline = ir(&manifest(
            "",
            "input \"In\" id=\"in\" channels=2 type=\"stereo\"; output \"Out\" id=\"out\" channels=2 type=\"stereo\" flags=\"main\" in-place-pair=\"in\"",
            "",
            "",
        ));
        let current = ir(&manifest(
            "",
            "input \"In\" id=\"in\" channels=2 type=\"stereo\"; output \"Out\" id=\"out\" channels=2 type=\"stereo\" flags=\"main,supports-64bits\" in-place-pair=\"in\"",
            "",
            "",
        ));
        let report = compare(
            &baseline,
            &current,
            Path::new("baseline/plugin.kdl"),
            Path::new("current/plugin.kdl"),
        )
        .expect("compare");
        assert!(report.has_forbidden());
        assert!(report.text().contains("forbidden audio-port.out"), "{}", report.text());
    }

    #[test]
    fn state_symbol_rename_preserves_persistent_tag() {
        let baseline_source =
            manifest("", "", "field \"old-name\" type=\"string\" tag=\"state-1\"", "");
        let current_source =
            manifest("", "", "field \"new-name\" type=\"string\" tag=\"state-1\"", "");
        let report = compare(
            &ir(&baseline_source),
            &ir(&current_source),
            Path::new("baseline/plugin.kdl"),
            Path::new("current/plugin.kdl"),
        )
        .expect("compare");
        assert!(!report.has_forbidden(), "{}", report.text());
        assert!(report.text().contains("symbol renamed"), "{}", report.text());
    }

    #[test]
    fn changing_a_persistent_state_tag_is_forbidden() {
        let baseline = ir(&manifest("", "", "field \"value\" type=\"string\" tag=\"state-1\"", ""));
        let current = ir(&manifest("", "", "field \"value\" type=\"string\" tag=\"state-2\"", ""));
        let report = compare(
            &baseline,
            &current,
            Path::new("baseline/plugin.kdl"),
            Path::new("current/plugin.kdl"),
        )
        .expect("compare");
        assert!(report.has_forbidden());
        assert!(report.text().contains("state.state-1"), "{}", report.text());
    }

    #[test]
    fn numeric_registry_id_change_is_forbidden_but_symbol_rename_is_compatible() {
        let directory = temp_dir();
        let old_dir = directory.join("old");
        let new_dir = directory.join("new");
        write_registry(
            &old_dir,
            "ids version=1 next=2 { entry kind=\"parameter\" key=\"cutoff\" value=1 tombstone=#false }\n",
        );
        write_registry(
            &new_dir,
            "ids version=1 next=2 { entry kind=\"parameter\" key=\"filter-cutoff\" value=1 tombstone=#false }\n",
        );
        let baseline =
            ir(&manifest("param \"Cutoff\" id=\"cutoff\" min=0.0 max=1.0 default=0.5", "", "", ""));
        let current = ir(&manifest(
            "param \"Cutoff\" id=\"filter-cutoff\" min=0.0 max=1.0 default=0.5",
            "",
            "",
            "",
        ));
        let report =
            compare(&baseline, &current, &old_dir.join("plugin.kdl"), &new_dir.join("plugin.kdl"))
                .expect("compare");
        assert!(!report.has_forbidden(), "{}", report.text());
        assert!(report.text().contains("clap-id.1"), "{}", report.text());

        write_registry(
            &new_dir,
            "ids version=1 next=3 { entry kind=\"parameter\" key=\"cutoff\" value=2 tombstone=#false }\n",
        );
        let changed =
            ir(&manifest("param \"Cutoff\" id=\"cutoff\" min=0.0 max=1.0 default=0.5", "", "", ""));
        let report =
            compare(&baseline, &changed, &old_dir.join("plugin.kdl"), &new_dir.join("plugin.kdl"))
                .expect("compare changed");
        assert!(report.has_forbidden());
        assert!(report.text().contains("numeric ID changed"), "{}", report.text());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn compatibility_rejects_a_registry_with_numeric_collisions() {
        let directory = temp_dir();
        let old_dir = directory.join("old");
        let new_dir = directory.join("new");
        write_registry(
            &old_dir,
            "ids version=1 next=2 { entry kind=\"parameter\" key=\"a\" value=1 tombstone=#false }\n",
        );
        write_registry(
            &new_dir,
            "ids version=1 next=3 { entry kind=\"parameter\" key=\"a\" value=1 tombstone=#false; entry kind=\"port\" key=\"b\" value=1 tombstone=#false }\n",
        );
        let source = manifest("", "", "", "");
        let error = compare(
            &ir(&source),
            &ir(&source),
            &old_dir.join("plugin.kdl"),
            &new_dir.join("plugin.kdl"),
        )
        .expect_err("collision must reject compatibility report");
        assert!(error.contains("collision"), "{error}");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn registry_requires_allocations_for_host_visible_entities() {
        let directory = temp_dir();
        let old_dir = directory.join("old");
        let new_dir = directory.join("new");
        write_registry(
            &old_dir,
            "ids version=1 next=2 { entry kind=\"parameter\" key=\"gain\" value=1 tombstone=#false }\n",
        );
        write_registry(
            &new_dir,
            "ids version=1 next=2 { entry kind=\"parameter\" key=\"gain\" value=1 tombstone=#false }\n",
        );
        let baseline =
            ir(&manifest("param \"Gain\" id=\"gain\" min=0.0 max=1.0 default=0.5", "", "", ""));
        let current = ir(&manifest(
            "param \"Gain\" id=\"gain\" min=0.0 max=1.0 default=0.5; param \"Mix\" id=\"mix\" min=0.0 max=1.0 default=0.5",
            "",
            "",
            "",
        ));
        let report =
            compare(&baseline, &current, &old_dir.join("plugin.kdl"), &new_dir.join("plugin.kdl"))
                .expect("compare");
        assert!(report.has_forbidden());
        assert!(report.text().contains("clap-id.parameter:mix"), "{}", report.text());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn classifies_parameter_and_topology_changes_deterministically() {
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
        let report = compare(
            &ir(&baseline_source),
            &ir(&current_source),
            Path::new("baseline/plugin.kdl"),
            Path::new("current/plugin.kdl"),
        )
        .expect("compare");
        assert!(report.text().contains("sensitive parameter.gain.range"));
        assert!(report.text().contains("forbidden audio-port.out"));
        assert!(report.changes.iter().any(|change| change.class == Class::Forbidden));
        let first = report.json();
        let second = report.json();
        assert_eq!(first, second);
    }

    #[test]
    fn json_report_escapes_all_ascii_control_characters() {
        assert_eq!("\"a\\u0001b\\b\\f\\n\\r\\t\"", json_string("a\u{1}b\u{8}\u{c}\n\r\t"));
    }
}
