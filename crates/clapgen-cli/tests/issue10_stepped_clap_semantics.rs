use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir()
        .join(format!("clapgen-issue10-stepped-semantics-{}-{nonce}", std::process::id()))
}

fn generate(metadata: &Path, out: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .args(["generate", "--metadata"])
        .arg(metadata)
        .arg("--out")
        .arg(out)
        .output()
        .expect("clapgen generate should run");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn stepped_runtime_uses_clap_truncation_semantics_instead_of_rounding_or_rejection() {
    let root = temporary_directory();
    fs::create_dir_all(&root).expect("temporary directory");

    let manifest = root.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.issue10-stepped\" name=\"Issue10 Stepped\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"Issue10SteppedProcessor\"\nparameters { param id=\"mode\" name=\"Mode\" min=0 max=3 default=0 flags=\"automatable,stepped,enum\" steps=4 }\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.params\" }\n",
    )
    .expect("manifest");
    fs::write(
        root.join("plugin.ids.kdl"),
        "ids version=1 next=2 {\n    entry kind=\"parameter\" key=\"mode\" value=1 tombstone=#false\n}\n",
    )
    .expect("registry");

    let out = root.join("generated");
    generate(&manifest, &out);

    let backend = fs::read_to_string(out.join("clapgen_instance_backend.hpp"))
        .expect("generated instance backend");

    assert!(
        backend.contains("std::trunc(value)"),
        "CLAP 1.2.10 defines stepped parameter conversion as integer cast/truncation; generated runtime must canonicalize fractional stepped values rather than reject or round them:\n{backend}"
    );
    assert!(
        !backend.contains("std::round(value)"),
        "round-to-nearest does not implement CLAP stepped parameter semantics:\n{backend}"
    );

    fs::remove_dir_all(root).expect("temporary directory cleanup");
}
