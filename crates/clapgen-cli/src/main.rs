mod metadata;

use std::env;
use std::process::ExitCode;

const HELP: &str = "Usage: clapgen <COMMAND>\n\nCommands:\n  doctor     Check the bootstrap toolchain contract\n  help       Print this help\n\nOptions:\n  -h, --help       Print help\n  -V, --version    Print version";

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
        [] => Err(HELP.to_owned()),
        _ => Err(format!("unknown command\n\n{HELP}")),
    }
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
    use super::{HELP, run};

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
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
    fn rejects_unknown_commands() {
        let error = run(&arguments(&["unknown"])).expect_err("unknown command should fail");
        assert!(error.starts_with("unknown command"));
        assert!(error.contains(HELP));
    }
}
