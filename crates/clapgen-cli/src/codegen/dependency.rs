use std::collections::BTreeSet;

use crate::ir::CanonicalIr;

pub(crate) fn collect(ir: &CanonicalIr) -> Vec<String> {
    let mut dependencies = ir.dependencies().iter().map(|path| normalize_path(path)).collect::<BTreeSet<_>>();
    for source in ir.sources() {
        let Some(resource) = source.key.strip_prefix("resource:") else {
            continue;
        };
        dependencies.insert(resolve_from_owner(&source.path, resource));
    }
    dependencies.into_iter().collect()
}

pub(crate) fn normalize_path(value: &str) -> String {
    let value = value.replace('\\', "/");
    let (prefix, remainder) = split_prefix(&value);
    let mut parts = Vec::new();
    for part in remainder.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|part| part != "..") {
                    parts.pop();
                } else if prefix.is_empty() {
                    parts.push("..");
                }
            }
            part => parts.push(part),
        }
    }

    let body = parts.join("/");
    match (prefix.as_str(), body.is_empty()) {
        ("/", true) => "/".to_owned(),
        ("/", false) => format!("/{body}"),
        ("//", true) => "//".to_owned(),
        ("//", false) => format!("//{body}"),
        (drive, true) if drive.ends_with(':') => format!("{drive}/"),
        (drive, false) if drive.ends_with(':') => format!("{drive}/{body}"),
        (_, _) => body,
    }
}

pub(crate) fn depfile_escape(value: &str) -> String {
    let normalized = normalize_path(value);
    let mut output = String::with_capacity(normalized.len());
    for character in normalized.chars() {
        match character {
            ' ' | '\t' | '#' | ':' => {
                output.push('\\');
                output.push(character);
            }
            '$' => output.push_str("$$"),
            '\\' => output.push_str("\\\\"),
            character => output.push(character),
        }
    }
    output
}

fn resolve_from_owner(owner: &str, dependency: &str) -> String {
    if is_absolute_like(dependency) {
        return normalize_path(dependency);
    }
    let owner = normalize_path(owner);
    let parent = owner.rsplit_once('/').map_or("", |(parent, _)| parent);
    if parent.is_empty() {
        normalize_path(dependency)
    } else {
        normalize_path(&format!("{parent}/{dependency}"))
    }
}

fn split_prefix(value: &str) -> (String, &str) {
    if let Some(remainder) = value.strip_prefix("//") {
        return ("//".to_owned(), remainder);
    }
    if let Some(remainder) = value.strip_prefix('/') {
        return ("/".to_owned(), remainder);
    }
    let bytes = value.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let prefix = value[..2].to_owned();
        let remainder = value[2..].strip_prefix('/').unwrap_or(&value[2..]);
        return (prefix, remainder);
    }
    (String::new(), value)
}

fn is_absolute_like(value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    normalized.starts_with('/')
        || normalized.starts_with("//")
        || normalized.as_bytes().get(1).is_some_and(|value| *value == b':')
}
