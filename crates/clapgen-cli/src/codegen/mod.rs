#![allow(dead_code)]

mod dependency;
mod depfile;
mod manifest;
mod metadata_cpp;
mod outputs;
mod render;
mod resources_cpp;
mod source_map;

pub(crate) use outputs::{GeneratedFile, GenerationPlan, OUTPUT_NAMES};

pub(crate) fn render(ir: &crate::ir::CanonicalIr) -> GenerationPlan {
    render::render(ir)
}

#[cfg(test)]
mod issue39_tests;
#[cfg(test)]
mod tests;
