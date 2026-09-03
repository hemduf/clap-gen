use std::collections::BTreeSet;

use crate::ir::CanonicalIr;

pub(crate) fn collect(ir: &CanonicalIr) -> Vec<String> {
    let mut dependencies =
        ir.dependencies().iter().map(|path| normalize_path(path)).collect::<BTreeSet<_>>();
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
                if parts.last().is_some_and(|part| *part != "..") {
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
        (unc, true) if unc.starts_with("//") => unc.to_owned(),
        (unc, false) if unc.starts_with("//") => format!("{unc}/{body}"),
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

pub(crate) fn resolve_from_base(base: &str, dependency: &str) -> String {
    if is_absolute_like(dependency) {
        return normalize_path(dependency);
    }
    let base = normalize_path(base);
    if base.is_empty() {
        normalize_path(dependency)
    } else {
        normalize_path(&format!("{base}/{dependency}"))
    }
}

pub(crate) fn relative_path(from: &str, to: &str) -> Option<String> {
    let (from_root, from_parts) = path_parts(from);
    let (to_root, to_parts) = path_parts(to);
    let case_insensitive =
        is_case_insensitive_root(&from_root) || is_case_insensitive_root(&to_root);
    let roots_match = if case_insensitive {
        from_root.eq_ignore_ascii_case(&to_root)
    } else {
        from_root == to_root
    };
    if !roots_match {
        return None;
    }

    let common =
        from_parts
            .iter()
            .zip(&to_parts)
            .take_while(|(left, right)| {
                if case_insensitive { left.eq_ignore_ascii_case(right) } else { left == right }
            })
            .count();

    let mut parts = vec!["..".to_owned(); from_parts.len() - common];
    parts.extend(to_parts.into_iter().skip(common));
    Some(if parts.is_empty() { ".".to_owned() } else { parts.join("/") })
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

fn path_parts(value: &str) -> (String, Vec<String>) {
    let normalized = normalize_path(value);
    if let Some(remainder) = normalized.strip_prefix("//") {
        let components = remainder
            .split('/')
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if components.len() >= 2 {
            return (
                format!("//{}/{}", components[0], components[1]),
                components.into_iter().skip(2).collect(),
            );
        }
        return ("//".to_owned(), components);
    }
    if let Some(remainder) = normalized.strip_prefix('/') {
        return (
            "/".to_owned(),
            remainder.split('/').filter(|part| !part.is_empty()).map(ToOwned::to_owned).collect(),
        );
    }
    let bytes = normalized.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/' {
        return (
            normalized[..2].to_owned(),
            normalized[3..]
                .split('/')
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        );
    }
    (
        String::new(),
        normalized.split('/').filter(|part| !part.is_empty()).map(ToOwned::to_owned).collect(),
    )
}

fn is_case_insensitive_root(root: &str) -> bool {
    root.ends_with(':') || root.starts_with("//")
}

fn split_prefix(value: &str) -> (String, &str) {
    if let Some(remainder) = value.strip_prefix("//") {
        let mut components = remainder.split('/');
        let server = components.next().unwrap_or_default();
        let share = components.next().unwrap_or_default();
        if !server.is_empty() && !share.is_empty() {
            let prefix_len = 2 + server.len() + 1 + share.len();
            let tail = value[prefix_len..].strip_prefix('/').unwrap_or(&value[prefix_len..]);
            return (value[..prefix_len].to_owned(), tail);
        }
        return ("//".to_owned(), remainder);
    }
    if let Some(remainder) = value.strip_prefix('/') {
        return ("/".to_owned(), remainder);
    }
    let bytes = value.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/' {
        return (value[..2].to_owned(), &value[3..]);
    }
    (String::new(), value)
}

fn is_absolute_like(value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    let bytes = normalized.as_bytes();
    normalized.starts_with('/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'/')
}
