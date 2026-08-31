use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn canonical_ir_v1_matches_the_reviewed_golden_snapshot() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let directory =
        env::temp_dir().join(format!("clapgen-ir-snapshot-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&directory).expect("temporary directory should be created");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.snapshot\" name=\"Snapshot\" vendor=\"Example\" version=\"1.0.0\" { feature \"synthesizer\"; feature \"instrument\" }\nprocessor class=\"SnapshotProcessor\"\nparameters { param \"Gain\" id=\"gain\" min=0 max=1 default=0.5 flags=\"automatable\" unit=\"dB\" }\naudio-ports { input \"Aux In\" id=\"aux-in\" channels=2; input \"Main In\" id=\"main-in\" channels=2 flags=\"main\"; output \"Main Out\" id=\"main-out\" channels=2 flags=\"main\" }\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.preset-load/2\" version=\"2\" }\n",
    )
    .expect("manifest should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .args(["inspect", "--format", "kdl"])
        .arg(&manifest)
        .output()
        .expect("clapgen should run");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let cli_stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let actual = cli_stdout.strip_suffix('\n').expect("CLI should append one trailing newline");
    let expected = include_str!("golden/issue5-ir-v1.kdl").replace("\r\n", "\n");
    assert_eq!(expected, actual);
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}
