from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / "tools" / "release_version.py"

spec = importlib.util.spec_from_file_location("release_version", TOOL)
assert spec is not None and spec.loader is not None
release_version = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = release_version
spec.loader.exec_module(release_version)


class ReleaseVersionTest(unittest.TestCase):
    def test_repository_workspace_version_is_valid_semver(self) -> None:
        version = release_version.workspace_version(ROOT / "Cargo.toml")
        parsed = release_version.parse_semver(version)
        self.assertEqual(version, parsed.raw)
        self.assertRegex(parsed.core, r"^[0-9]+\.[0-9]+\.[0-9]+$")

    def test_semver_accepts_prerelease_and_build_metadata(self) -> None:
        parsed = release_version.parse_semver("1.2.3-rc.1+build.7")
        self.assertEqual("1.2.3", parsed.core)
        self.assertEqual("rc.1", parsed.prerelease)
        self.assertEqual("build.7", parsed.build)

    def test_semver_rejects_invalid_leading_zeroes(self) -> None:
        for version in ("01.2.3", "1.02.3", "1.2.03", "1.2.3-01"):
            with self.subTest(version=version):
                with self.assertRaises(ValueError):
                    release_version.parse_semver(version)

    def test_tag_must_match_manifest_version_exactly(self) -> None:
        parsed = release_version.parse_semver("0.4.0-beta.2")
        release_version.validate_tag(parsed, "v0.4.0-beta.2")
        with self.assertRaises(ValueError):
            release_version.validate_tag(parsed, "v0.4.0")

    def test_cli_validates_current_repository_tag(self) -> None:
        version = release_version.workspace_version(ROOT / "Cargo.toml")
        result = subprocess.run(
            [sys.executable, str(TOOL), "--tag", f"v{version}"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual(version, result.stdout.strip())

    def test_workspace_version_is_read_only_from_workspace_package(self) -> None:
        manifest = """\
[workspace]
members = ["crate"]

[workspace.package]
version = "2.3.4"

[dependencies]
other = "9.9.9"
"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "Cargo.toml"
            path.write_text(manifest, encoding="utf-8")
            self.assertEqual("2.3.4", release_version.workspace_version(path))


if __name__ == "__main__":
    unittest.main()
