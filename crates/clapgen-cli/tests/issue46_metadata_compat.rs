use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

#[test]
fn descriptor_utf8_encoding_does_not_change_existing_metadata_rendering() {
    let root = temporary_directory("issue46-metadata-compat");
    let source = root.join("source");
    let generated = root.join("generated");
    fs::create_dir_all(&source).expect("source directory");

    let manifest = source.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.metadata-compat\" name=\"Café\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"MetadataCompatProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    )
    .expect("manifest should be writable");

    let generation = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .args(["generate", "--metadata"])
        .arg(&manifest)
        .arg("--out")
        .arg(&generated)
        .output()
        .expect("clapgen generate should run");
    assert!(
        generation.status.success(),
        "generation failed: {}",
        String::from_utf8_lossy(&generation.stderr)
    );

    let metadata = fs::read_to_string(generated.join("clapgen_metadata.cpp"))
        .expect("generated metadata should be readable");
    let descriptors = fs::read_to_string(generated.join("clapgen_descriptors.hpp"))
        .expect("generated descriptors should be readable");

    assert!(
        metadata.contains("\"Café\""),
        "metadata renderer must preserve its pre-#46 UTF-8 representation: {metadata}"
    );
    assert!(
        descriptors.contains("\"Caf\\303\\251\""),
        "descriptor renderer must use explicit UTF-8 bytes: {descriptors}"
    );

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}
