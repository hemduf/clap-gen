pub(crate) const OUTPUT_NAMES: &[&str] = &[
    "clapgen.d",
    "clapgen.manifest.kdl",
    "clapgen.sources.kdl",
    "clapgen_metadata.cpp",
    "clapgen_metadata.hpp",
    "clapgen_resources.hpp",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedFile {
    pub(crate) path: &'static str,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationPlan {
    pub(crate) files: Vec<GeneratedFile>,
}
