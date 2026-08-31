use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn pinned_stable_compatibility_extension_id_remains_accepted() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let directory = env::temp_dir().join(format!(
        "clapgen-issue34-compat-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("temporary directory should be created");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.compat\" name=\"Compat\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"CompatProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.context-menu.draft/0\" }\n",
    )
    .expect("manifest should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .arg("validate")
        .arg(&manifest)
        .output()
        .expect("clapgen should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}
