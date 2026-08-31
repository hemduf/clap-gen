from __future__ import annotations

import os
from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]
CARGO = os.environ.get("CARGO", "cargo")
CMAKE = os.environ.get("CMAKE", "cmake")
CTEST = os.environ.get("CTEST", "ctest")


def run(*command: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


class RepositoryBootstrapTest(unittest.TestCase):
    def test_required_engineering_files_exist(self) -> None:
        expected = {
            ".clang-format",
            ".gitignore",
            "CONTRIBUTING.md",
            "Cargo.lock",
            "Cargo.toml",
            "CMakeLists.txt",
            "CMakePresets.json",
            "DEPENDENCIES.md",
            "README.md",
            "deny.toml",
            "docs/generated-source-policy.md",
            "docs/realtime-coding-rules.md",
            "rust-toolchain.toml",
            "rustfmt.toml",
        }

        missing = sorted(path for path in expected if not (ROOT / path).is_file())
        self.assertEqual([], missing, f"missing engineering files: {missing}")

    def test_dependencies_are_pinned(self) -> None:
        cargo_manifest = (ROOT / "crates/clapgen-cli/Cargo.toml").read_text()
        clap_dependency = (ROOT / "cmake/dependencies/Clap.cmake").read_text()
        rust_toolchain = (ROOT / "rust-toolchain.toml").read_text()

        self.assertIn('kdl = "=6.7.1"', cargo_manifest)
        self.assertIn("a47f6badb49d948fd009998f28309cdab78979c9", clap_dependency)
        self.assertIn('channel = "1.98.0"', rust_toolchain)

    def test_generated_outputs_are_build_tree_only(self) -> None:
        policy = (ROOT / "docs/generated-source-policy.md").read_text()
        gitignore = (ROOT / ".gitignore").read_text()

        self.assertIn("build tree", policy.lower())
        self.assertIn("user-owned", policy.lower())
        self.assertIn("/build/", gitignore)
        self.assertIn("/target/", gitignore)

        tracked = run("git", "ls-files").stdout.splitlines()
        generated = [
            path
            for path in tracked
            if path.startswith(("build/", "target/"))
            or "/__pycache__/" in path
            or path.endswith((".pyc", ".pyo"))
            or path.endswith((".generated.cpp", ".generated.hpp"))
        ]
        self.assertEqual([], generated, f"generated outputs are tracked: {generated}")

    def test_cli_smoke_contract(self) -> None:
        version = run(CARGO, "run", "--quiet", "-p", "clapgen-cli", "--", "--version")
        self.assertEqual(0, version.returncode, version.stderr)
        self.assertRegex(version.stdout.strip(), r"^clapgen \d+\.\d+\.\d+$")

        doctor = run(CARGO, "run", "--quiet", "-p", "clapgen-cli", "--", "doctor")
        self.assertEqual(0, doctor.returncode, doctor.stderr)
        self.assertEqual(
            [
                "clapgen doctor",
                "status: ok",
                "metadata: KDL 2.0",
                "runtime: C++20",
            ],
            doctor.stdout.splitlines(),
        )

    def test_cmake_configure_build_and_test_contract(self) -> None:
        configure = run(CMAKE, "--preset", "dev")
        self.assertEqual(0, configure.returncode, configure.stderr)

        build = run(CMAKE, "--build", "--preset", "dev")
        self.assertEqual(0, build.returncode, build.stderr)

        test = run(CTEST, "--preset", "dev")
        self.assertEqual(0, test.returncode, test.stderr)


if __name__ == "__main__":
    unittest.main()
