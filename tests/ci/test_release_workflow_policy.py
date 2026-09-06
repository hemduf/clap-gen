from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


class ReleaseWorkflowPolicyTest(unittest.TestCase):
    def test_current_intel_runner_false_positive_is_narrowly_ignored(self) -> None:
        release = read(".github/workflows/release.yml")
        actionlint = read(".github/actionlint.yaml")

        self.assertIn("- os: macos-15-intel", release)
        self.assertIn(".github/workflows/release.yml:", actionlint)
        self.assertIn('label "macos-15-intel" is unknown', actionlint)

    def test_checksum_glob_cannot_be_parsed_as_an_option(self) -> None:
        release = read(".github/workflows/release.yml")
        self.assertIn("sha256sum ./* > SHA256SUMS", release)
        self.assertNotIn("sha256sum * > SHA256SUMS", release)


if __name__ == "__main__":
    unittest.main()
