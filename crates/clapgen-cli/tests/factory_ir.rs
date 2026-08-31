use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn factory_metadata_reaches_the_canonical_ir() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let directory = env::temp_dir().join(format!("clapgen-factory-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&directory).expect("temporary directory should be created");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.factory\" name=\"Factory\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"FactoryProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories { factory \"clap.plugin-factory\" kind=\"plugin\" }\nextensions {}\n",
    )
    .expect("manifest should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .args(["inspect", "--format", "kdl"])
        .arg(&manifest)
        .output()
        .expect("clapgen should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("factory \"clap.plugin-factory\" kind=\"plugin\""), "{stdout}");

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}
