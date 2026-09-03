#![allow(dead_code)]

mod dependency;
mod depfile;
mod manifest;
mod metadata_cpp;
mod outputs;
mod render;
mod resources_cpp;
mod source_map;
mod writer;

use std::path::Path;

pub(crate) use outputs::{GeneratedFile, GenerationPlan, OUTPUT_NAMES};

pub(crate) fn render(ir: &crate::ir::CanonicalIr) -> GenerationPlan {
    render::render(ir)
}

pub(crate) fn render_for_output(
    ir: &crate::ir::CanonicalIr,
    dependency_base: &Path,
    output_directory: &Path,
) -> GenerationPlan {
    render::render_for_output(ir, dependency_base, output_directory)
}

pub(crate) fn write(plan: &GenerationPlan, directory: &Path) -> Result<(), String> {
    writer::write(plan, directory)
}

#[cfg(test)]
mod issue39_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod writer_tests;
