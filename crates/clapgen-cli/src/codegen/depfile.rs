use std::path::Path;

use crate::ir::CanonicalIr;

use super::dependency;

const TARGET: &str = "clapgen.manifest.kdl";

pub(crate) fn render(ir: &CanonicalIr) -> String {
    let dependencies = dependency::collect(ir);
    render_dependencies(TARGET, &dependencies)
}

pub(crate) fn render_for_output(
    ir: &CanonicalIr,
    dependency_base: &Path,
    output_directory: &Path,
) -> String {
    let base = dependency::normalize_path(&dependency_base.to_string_lossy());
    let output = dependency::normalize_path(&output_directory.to_string_lossy());
    let dependencies = dependency::collect(ir)
        .into_iter()
        .map(|path| {
            let physical = dependency::resolve_from_base(&base, &path);
            dependency::relative_path(&output, &physical).unwrap_or(physical)
        })
        .collect::<Vec<_>>();
    render_dependencies(TARGET, &dependencies)
}

pub(crate) fn render_for_depfile_base(
    ir: &CanonicalIr,
    dependency_base: &Path,
    output_directory: &Path,
    depfile_base: &Path,
) -> String {
    let base = dependency::normalize_path(&dependency_base.to_string_lossy());
    let output = dependency::normalize_path(&output_directory.to_string_lossy());
    let depfile_base = dependency::normalize_path(&depfile_base.to_string_lossy());
    let physical_target = dependency::resolve_from_base(&output, TARGET);
    let target =
        dependency::relative_path(&depfile_base, &physical_target).unwrap_or(physical_target);
    let dependencies = dependency::collect(ir)
        .into_iter()
        .map(|path| {
            let physical = dependency::resolve_from_base(&base, &path);
            dependency::relative_path(&depfile_base, &physical).unwrap_or(physical)
        })
        .collect::<Vec<_>>();
    render_dependencies(&target, &dependencies)
}

fn render_dependencies(target: &str, dependencies: &[String]) -> String {
    let dependencies = dependencies
        .iter()
        .map(|path| dependency::depfile_escape(path))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{}: {dependencies}\n", dependency::depfile_escape(target))
}
