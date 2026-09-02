use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn run(command: &mut Command, context: &str) {
    let output = command.output().unwrap_or_else(|error| panic!("failed to run {context}: {error}"));
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_manifest(path: &Path) {
    fs::write(
        path,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.compile\" name=\"Compile\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"CompileProcessor\"\nparameters { param \"Gain\" id=\"gain\" min=0 max=1 default=0.5 }\naudio-ports { input \"Main In\" id=\"in\" channels=2; output \"Main Out\" id=\"out\" channels=2 }\nnote-ports {}\nstate {}\ngui { resource \"panel.svg\" mime=\"image/svg+xml\" }\npresets {}\nfactories {}\nextensions {}\n",
    )
    .expect("manifest should be writable");
}

#[test]
fn generated_metadata_and_resource_cpp_compile_as_cxx20() {
    let root = temporary_directory("issue42-cpp-compile");
    let source = root.join("source");
    let generated = root.join("generated");
    let build = root.join("cmake-build");
    fs::create_dir_all(&source).expect("source directory");
    let manifest = source.join("plugin.kdl");
    write_manifest(&manifest);
    fs::write(source.join("panel.svg"), b"<svg/>\n").expect("resource");

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

    fs::write(
        root.join("smoke.cpp"),
        "#include \"clapgen_metadata.hpp\"\n#include \"clapgen_resources.hpp\"\nint main() {\n    return clapgen::generated::plugin.id == nullptr || clapgen::generated::resources.size() != 1;\n}\n",
    )
    .expect("smoke source");
    fs::write(
        root.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.20)\nproject(clapgen_generated_smoke LANGUAGES CXX)\nadd_executable(generated_smoke smoke.cpp generated/clapgen_metadata.cpp)\ntarget_include_directories(generated_smoke PRIVATE generated)\ntarget_compile_features(generated_smoke PRIVATE cxx_std_20)\n",
    )
    .expect("CMake project");

    run(
        Command::new("cmake").arg("-S").arg(&root).arg("-B").arg(&build),
        "CMake configure for generated C++",
    );
    run(
        Command::new("cmake").arg("--build").arg(&build).arg("--config").arg("Release"),
        "CMake build for generated C++",
    );

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}
