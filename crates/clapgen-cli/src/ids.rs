use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlValue};

const REGISTRY_VERSION: i128 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryEntry {
    pub(crate) kind: String,
    pub(crate) key: String,
    pub(crate) value: u32,
    pub(crate) tombstone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Registry {
    version: i128,
    next: u32,
    entries: Vec<RegistryEntry>,
}

impl Default for Registry {
    fn default() -> Self {
        Self { version: REGISTRY_VERSION, next: 1, entries: Vec::new() }
    }
}

struct RegistryLock {
    path: PathBuf,
}

impl Drop for RegistryLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn allocate(path: &Path, kind: &str, key: &str) -> Result<u32, String> {
    validate_symbol(kind, "kind")?;
    validate_symbol(key, "key")?;
    let _lock = lock_registry(path)?;
    let mut registry = load_registry(path)?;

    if let Some(entry) =
        registry.entries.iter().find(|entry| entry.kind == kind && entry.key == key)
    {
        if entry.tombstone {
            return Err(format!(
                "{}: `{kind}:{key}` is tombstoned and cannot be allocated again",
                path.display()
            ));
        }
        return Ok(entry.value);
    }

    let value = registry.next;
    registry.next = registry
        .next
        .checked_add(1)
        .ok_or_else(|| format!("{}: numeric CLAP ID space exhausted", path.display()))?;
    registry.entries.push(RegistryEntry {
        kind: kind.to_owned(),
        key: key.to_owned(),
        value,
        tombstone: false,
    });
    canonicalize(&mut registry);
    write_registry_atomic(path, &registry)?;
    Ok(value)
}

pub(crate) fn rename(path: &Path, kind: &str, old_key: &str, new_key: &str) -> Result<u32, String> {
    validate_symbol(kind, "kind")?;
    validate_symbol(old_key, "old key")?;
    validate_symbol(new_key, "new key")?;
    let _lock = lock_registry(path)?;
    let mut registry = load_registry(path)?;

    if old_key == new_key {
        return registry
            .entries
            .iter()
            .find(|entry| entry.kind == kind && entry.key == old_key && !entry.tombstone)
            .map(|entry| entry.value)
            .ok_or_else(|| {
                format!("{}: active ID `{kind}:{old_key}` was not found", path.display())
            });
    }

    if registry.entries.iter().any(|entry| entry.kind == kind && entry.key == new_key) {
        return Err(format!("{}: target `{kind}:{new_key}` already exists", path.display()));
    }

    let entry = registry
        .entries
        .iter_mut()
        .find(|entry| entry.kind == kind && entry.key == old_key && !entry.tombstone)
        .ok_or_else(|| format!("{}: active ID `{kind}:{old_key}` was not found", path.display()))?;
    entry.key = new_key.to_owned();
    let value = entry.value;
    canonicalize(&mut registry);
    write_registry_atomic(path, &registry)?;
    Ok(value)
}

pub(crate) fn tombstone(path: &Path, kind: &str, key: &str) -> Result<u32, String> {
    validate_symbol(kind, "kind")?;
    validate_symbol(key, "key")?;
    let _lock = lock_registry(path)?;
    let mut registry = load_registry(path)?;
    let entry = registry
        .entries
        .iter_mut()
        .find(|entry| entry.kind == kind && entry.key == key)
        .ok_or_else(|| format!("{}: ID `{kind}:{key}` was not found", path.display()))?;
    entry.tombstone = true;
    let value = entry.value;
    write_registry_atomic(path, &registry)?;
    Ok(value)
}

pub(crate) fn read_entries(path: &Path) -> Result<Option<Vec<RegistryEntry>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(load_registry(path)?.entries))
}

fn validate_symbol(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
        return Err(format!("invalid {label} `{value}`: use a non-empty stable symbolic token"));
    }
    Ok(())
}

fn lock_registry(path: &Path) -> Result<RegistryLock, String> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create `{}`: {error}", parent.display()))?;
    }
    let lock_path = suffixed_path(path, OsStr::new(".lock"));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|error| {
            format!(
                "failed to lock `{}`: {error}\nhint: another `clapgen ids` update may be in progress; retry after it finishes",
                path.display()
            )
        })?;
    Ok(RegistryLock { path: lock_path })
}

fn load_registry(path: &Path) -> Result<Registry, String> {
    if !path.exists() {
        return Ok(Registry::default());
    }
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{}`: {error}", path.display()))?;
    parse_registry(path, &source)
}

fn parse_registry(path: &Path, source: &str) -> Result<Registry, String> {
    let document = KdlDocument::parse_v2(source)
        .map_err(|error| format!("{}: invalid KDL 2.0 ID registry: {error}", path.display()))?;
    let root = document
        .get("ids")
        .ok_or_else(|| format!("{}: missing `ids` registry root", path.display()))?;
    let version = integer_prop(root, "version")
        .ok_or_else(|| format!("{}: `ids` requires integer `version`", path.display()))?;
    if version != REGISTRY_VERSION {
        return Err(format!(
            "{}: unsupported ID registry version `{version}`; expected `{REGISTRY_VERSION}`",
            path.display()
        ));
    }
    let next = integer_prop(root, "next")
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{}: `ids` requires positive integer `next`", path.display()))?;

    let mut entries = Vec::new();
    if let Some(children) = root.children() {
        for node in children.nodes().iter().filter(|node| node.name().value() == "entry") {
            let kind = string_prop(node, "kind").ok_or_else(|| {
                format!("{}: registry entry requires string `kind`", path.display())
            })?;
            let key = string_prop(node, "key").ok_or_else(|| {
                format!("{}: registry entry requires string `key`", path.display())
            })?;
            validate_symbol(kind, "registry kind")?;
            validate_symbol(key, "registry key")?;
            let value = integer_prop(node, "value")
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| {
                    format!("{}: registry entry requires u32 `value`", path.display())
                })?;
            let tombstone = optional_bool_prop(node, "tombstone")?.unwrap_or(false);
            entries.push(RegistryEntry {
                kind: kind.to_owned(),
                key: key.to_owned(),
                value,
                tombstone,
            });
        }
    }

    let mut registry = Registry { version, next, entries };
    canonicalize(&mut registry);
    validate_registry(path, &registry)?;
    Ok(registry)
}

fn validate_registry(path: &Path, registry: &Registry) -> Result<(), String> {
    let mut symbols = BTreeSet::new();
    let mut values = BTreeMap::new();
    for entry in &registry.entries {
        let symbol = (entry.kind.as_str(), entry.key.as_str());
        if !symbols.insert(symbol) {
            return Err(format!(
                "{}: duplicate registry symbol `{}:{}`",
                path.display(),
                entry.kind,
                entry.key
            ));
        }
        if let Some(previous) = values.insert(entry.value, symbol) {
            return Err(format!(
                "{}: numeric ID collision `{}` between `{}:{}` and `{}:{}`",
                path.display(),
                entry.value,
                previous.0,
                previous.1,
                entry.kind,
                entry.key
            ));
        }
    }
    if registry.entries.iter().any(|entry| entry.value >= registry.next) {
        return Err(format!(
            "{}: registry `next={}` must be greater than every allocated/tombstoned ID",
            path.display(),
            registry.next
        ));
    }
    Ok(())
}

fn canonicalize(registry: &mut Registry) {
    registry.entries.sort_by(|a, b| (a.value, &a.kind, &a.key).cmp(&(b.value, &b.kind, &b.key)));
}

fn write_registry_atomic(path: &Path, registry: &Registry) -> Result<(), String> {
    let source = serialize_registry(registry);
    let temp_suffix = format!(".tmp-{}", std::process::id());
    let temp = suffixed_path(path, OsStr::new(&temp_suffix));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("failed to create `{}`: {error}", temp.display()))?;

    if let Err(error) = file.write_all(source.as_bytes()).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(format!("failed to persist `{}`: {error}", temp.display()));
    }
    drop(file);

    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("failed to atomically update `{}`: {error}", path.display())
    })
}

fn serialize_registry(registry: &Registry) -> String {
    let mut out = format!("ids version={} next={} {{\n", registry.version, registry.next);
    for entry in &registry.entries {
        out.push_str(&format!(
            "    entry kind={} key={} value={} tombstone={}\n",
            quote(&entry.kind),
            quote(&entry.key),
            entry.value,
            if entry.tombstone { "#true" } else { "#false" }
        ));
    }
    out.push_str("}\n");
    out
}

fn suffixed_path(path: &Path, suffix: &OsStr) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn prop<'a>(node: &'a kdl::KdlNode, key: &str) -> Option<&'a KdlValue> {
    node.entries().iter().rev().find_map(|entry| {
        let name = entry.name()?;
        (name.value() == key).then_some(entry.value())
    })
}

fn string_prop<'a>(node: &'a kdl::KdlNode, key: &str) -> Option<&'a str> {
    match prop(node, key)? {
        KdlValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn integer_prop(node: &kdl::KdlNode, key: &str) -> Option<i128> {
    match prop(node, key)? {
        KdlValue::Integer(value) => Some(*value),
        _ => None,
    }
}

fn optional_bool_prop(node: &kdl::KdlNode, key: &str) -> Result<Option<bool>, String> {
    match prop(node, key) {
        None => Ok(None),
        Some(KdlValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("registry property `{key}` must be a KDL boolean")),
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{allocate, parse_registry, read_entries, rename, serialize_registry, tombstone};

    fn temporary_path() -> PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        env::temp_dir().join(format!("clapgen-ids-{}-{nonce}.kdl", std::process::id()))
    }

    #[test]
    fn allocation_is_stable_and_rename_preserves_numeric_id() {
        let path = temporary_path();
        assert_eq!(1, allocate(&path, "parameter", "cutoff").expect("allocate"));
        assert_eq!(1, allocate(&path, "parameter", "cutoff").expect("idempotent allocate"));
        assert_eq!(1, rename(&path, "parameter", "cutoff", "filter-cutoff").expect("rename"));
        assert_eq!(1, allocate(&path, "parameter", "filter-cutoff").expect("renamed lookup"));
        assert_eq!(
            1,
            rename(&path, "parameter", "filter-cutoff", "filter-cutoff")
                .expect("idempotent rename")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tombstones_are_permanent_and_values_are_never_reused() {
        let path = temporary_path();
        assert_eq!(1, allocate(&path, "parameter", "old").expect("allocate old"));
        assert_eq!(1, tombstone(&path, "parameter", "old").expect("tombstone"));
        let error = allocate(&path, "parameter", "old").expect_err("tombstone must reject reuse");
        assert!(error.contains("tombstoned"), "{error}");
        assert_eq!(2, allocate(&path, "parameter", "new").expect("allocate new"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn existing_registry_is_replaced_by_rename_and_tombstone() {
        let path = temporary_path();
        allocate(&path, "parameter", "old").expect("initial allocation");
        rename(&path, "parameter", "old", "new").expect("replace registry during rename");
        tombstone(&path, "parameter", "new").expect("replace registry during tombstone");
        let entries = read_entries(&path).expect("registry read").expect("registry exists");
        assert_eq!(1, entries.len());
        assert_eq!(1, entries[0].value);
        assert!(entries[0].tombstone);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn collisions_are_rejected_with_actionable_diagnostic() {
        let path = temporary_path();
        let source = "ids version=1 next=3 {\n    entry kind=\"parameter\" key=\"a\" value=1\n    entry kind=\"port\" key=\"b\" value=1\n}\n";
        let error = parse_registry(&path, source).expect_err("collision must fail");
        assert!(error.contains("collision"), "{error}");
        assert!(error.contains("parameter:a"), "{error}");
        assert!(error.contains("port:b"), "{error}");
    }

    #[test]
    fn malformed_tombstone_type_is_rejected() {
        let path = temporary_path();
        let source = "ids version=1 next=2 { entry kind=\"parameter\" key=\"a\" value=1 tombstone=\"false\" }\n";
        let error = parse_registry(&path, source).expect_err("non-boolean tombstone must fail");
        assert!(error.contains("tombstone"), "{error}");
        assert!(error.contains("boolean"), "{error}");
    }

    #[test]
    fn concurrent_allocation_never_silently_duplicates_numeric_ids() {
        let path = temporary_path();
        let barrier = Arc::new(Barrier::new(2));
        let handles = ["a", "b"].map(|key| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                allocate(&path, "parameter", key)
            })
        });
        let first =
            handles.into_iter().map(|handle| handle.join().expect("thread")).collect::<Vec<_>>();

        let mut values = Vec::new();
        for (key, result) in ["a", "b"].into_iter().zip(first) {
            match result {
                Ok(value) => values.push(value),
                Err(error) => {
                    assert!(error.contains("another `clapgen ids` update"), "{error}");
                    values
                        .push(allocate(&path, "parameter", key).expect("retry after lock release"));
                }
            }
        }
        values.sort_unstable();
        assert_eq!(vec![1, 2], values);

        let source = fs::read_to_string(&path).expect("registry must remain readable");
        parse_registry(&path, &source).expect("registry must remain valid after concurrent update");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn serialization_is_deterministic() {
        let path = temporary_path();
        allocate(&path, "port", "out").expect("allocate out");
        allocate(&path, "parameter", "gain").expect("allocate gain");
        let source = fs::read_to_string(&path).expect("read registry");
        let registry = parse_registry(&path, &source).expect("parse registry");
        assert_eq!(source, serialize_registry(&registry));
        let _ = fs::remove_file(path);
    }
}
