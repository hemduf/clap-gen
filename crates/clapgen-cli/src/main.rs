mod metadata;

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use metadata::{DEFAULT_MANIFEST, format_metadata, parse_metadata};

const HELP: &str = "Usage: clapgen <COMMAND>\n\nCommands:\n  init [PATH]          Create canonical KDL 2.0 metadata\n  fmt [--check] PATH  Format and validate metadata\n  deps PATH            Print metadata import dependencies\n  doctor               Check the bootstrap toolchain contract\n  help                 Print this help\n\nOptions:\n  -h, --help       Print help\n  -V, --version    Print version";

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run(arguments: &[String]) -> Result<String, String> {
    match arguments {
        [argument] if argument == "-V" || argument == "--version" => {
            Ok(format!("clapgen {}", env!("CARGO_PKG_VERSION")))
        }
        [argument] if argument == "-h" || argument == "--help" || argument == "help" => {
            Ok(HELP.to_owned())
        }
        [command] if command == "doctor" => doctor(),
        [command] if command == "init" => init_manifest(Path::new("plugin.kdl")),
        [command, path] if command == "init" => init_manifest(Path::new(path)),
        [command, path] if command == "fmt" => format_file(Path::new(path), false),
        [command, flag, path] if command == "fmt" && flag == "--check" => {
            format_file(Path::new(path), true)
        }
        [command, path] if command == "deps" => metadata_dependencies(Path::new(path)),
        [] => Err(HELP.to_owned()),
        _ => Err(format!("unknown command or arguments\n\n{HELP}")),
    }
}

fn init_manifest(path: &Path) -> Result<String, String> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|error| {
            format!("failed to create metadata directory `{}`: {error}", parent.display())
        })?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create `{}`: {error}", path.display()))?;
    file.write_all(DEFAULT_MANIFEST.as_bytes())
        .map_err(|error| format!("failed to write `{}`: {error}", path.display()))?;

    Ok(format!("created {}", path.display()))
}

fn format_file(path: &Path, check: bool) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{}`: {error}", path.display()))?;
    let formatted = format_metadata(path, &source)?;

    if formatted == source {
        return Ok(format!("formatted: ok {}", path.display()));
    }

    if check {
        return Err(format!(
            "{} is not canonically formatted\nhint: run `clapgen fmt {}`",
            path.display(),
            path.display()
        ));
    }

    fs::write(path, formatted)
        .map_err(|error| format!("failed to write `{}`: {error}", path.display()))?;
    Ok(format!("formatted {}", path.display()))
}

fn metadata_dependencies(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{}`: {error}", path.display()))?;
    let parsed = parse_metadata(path, &source)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));

    let dependencies = parsed
        .imports
        .iter()
        .map(|import| base.join(import).display().to_string())
        .collect::<Vec<_>>();

    Ok(dependencies.join("\n"))
}

fn doctor() -> Result<String, String> {
    const KDL_V2_PROBE: &str = "/- kdl-version 2\nmetadata \"KDL 2.0\"\n";
    KDL_V2_PROBE
        .parse::<kdl::KdlDocument>()
        .map_err(|error| format!("KDL 2.0 parser check failed: {error}"))?;

    Ok(["clapgen doctor", "status: ok", "metadata: KDL 2.0", "runtime: C++20"].join("\n"))
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{HELP, run};

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn temporary_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        env::temp_dir().join(format!("clapgen-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn reports_the_package_version() {
        let output = run(&arguments(&["--version"])).expect("version should succeed");
        assert_eq!(format!("clapgen {}", env!("CARGO_PKG_VERSION")), output);
    }

    #[test]
    fn validates_the_bootstrap_contract() {
        let output = run(&arguments(&["doctor"])).expect("doctor should succeed");
        assert_eq!("clapgen doctor\nstatus: ok\nmetadata: KDL 2.0\nruntime: C++20", output);
    }

    #[test]
    fn init_fmt_and_deps_cover_the_metadata_file_lifecycle() {
        let directory = temporary_directory();
        let path = directory.join("plugin.kdl");
        let path_text = path.to_string_lossy().into_owned();

        let output = run(&arguments(&["init", &path_text])).expect("init should succeed");
        assert!(output.starts_with("created "));

        let check = run(&arguments(&["fmt", "--check", &path_text]))
            .expect("generated manifest must already be canonical");
        assert!(check.starts_with("formatted: ok "));

        let source = fs::read_to_string(&path).expect("manifest should be readable");
        let source = source.replace(
            "clapgen schema=\"1.0.0\"\n",
            "clapgen schema=\"1.0.0\"\nimport \"shared/common.kdl\"\n",
        );
        fs::write(&path, source).expect("manifest should be writable");

        let deps = run(&arguments(&["deps", &path_text])).expect("deps should succeed");
        assert!(deps.ends_with("shared/common.kdl"), "{deps}");

        fs::remove_dir_all(directory).expect("temporary directory should be removable");
    }

    #[test]
    fn rejects_unknown_commands() {
        let error = run(&arguments(&["unknown"])).expect_err("unknown command should fail");
        assert!(error.starts_with("unknown command"));
        assert!(error.contains(HELP));
    }
}
