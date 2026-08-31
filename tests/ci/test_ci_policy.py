from __future__ import annotations

import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


class CiPolicyTest(unittest.TestCase):
    def test_ci_workflow_is_read_only_and_fork_safe(self) -> None:
        workflow = read(".github/workflows/ci.yml")
        self.assertIn("permissions:\n  contents: read", workflow)
        self.assertNotIn("pull-requests: write", workflow)
        self.assertNotIn("contents: write", workflow)
        self.assertNotIn("secrets.", workflow)
        self.assertIn("pull_request:", workflow)
        self.assertNotIn("pull_request_target:", workflow)

    def test_ci_covers_cross_platform_rust_cpp_and_sanitizers(self) -> None:
        workflow = read(".github/workflows/ci.yml")
        for runner in ("ubuntu-latest", "macos-latest", "windows-latest"):
            self.assertIn(runner, workflow)
        for command in (
            "cargo fmt --all --check",
            "cargo clippy --workspace --all-targets --locked -- -D warnings",
            "cargo test --workspace --locked",
            "cppcheck",
            "Debug",
            "Release",
            "-fsanitize=address,undefined",
        ):
            self.assertIn(command, workflow)
        self.assertIn(r"C:\Program Files\Cppcheck\cppcheck.exe", workflow)

    def test_sanitizers_run_only_on_main_pushes(self) -> None:
        workflow = read(".github/workflows/ci.yml")
        self.assertIn(
            "if: github.event_name == 'push' && github.ref == 'refs/heads/main'",
            workflow,
        )
        self.assertIn("EVENT_NAME: ${{ github.event_name }}", workflow)
        self.assertIn('test "$SANITIZERS_RESULT" = success', workflow)
        self.assertIn('test "$SANITIZERS_RESULT" = skipped', workflow)

    def test_ci_has_hygiene_cache_and_failure_artifact_retention(self) -> None:
        workflow = read(".github/workflows/ci.yml")
        self.assertIn("concurrency:", workflow)
        self.assertIn("cancel-in-progress: true", workflow)
        self.assertIn("actions/cache@v4", workflow)
        self.assertIn("hashFiles('**/Cargo.lock')", workflow)
        self.assertIn("actionlint", workflow)
        self.assertIn("retention-days: 7", workflow)

    def test_required_gate_is_single_stable_branch_protection_context(self) -> None:
        workflow = read(".github/workflows/ci.yml")
        self.assertIn("name: Required CI gate", workflow)
        self.assertIn("needs: [policy, rust, cpp, sanitizers]", workflow)
        self.assertIn("if: always()", workflow)
        for dependency in ("policy", "rust", "cpp", "sanitizers"):
            self.assertIn(f"needs.{dependency}.result", workflow)

        policy = json.loads(read(".github/branch-protection.json"))
        self.assertEqual(
            ["Required CI gate"], policy["required_status_checks"]["contexts"]
        )
        self.assertTrue(policy["required_status_checks"]["strict"])

    def test_review_policy_supports_solo_maintainer_authorization(self) -> None:
        policy = json.loads(read(".github/branch-protection.json"))
        reviews = policy["required_pull_request_reviews"]
        self.assertEqual(0, reviews["required_approving_review_count"])
        self.assertFalse(reviews["dismiss_stale_reviews"])
        self.assertFalse(reviews["require_code_owner_reviews"])
        self.assertFalse(reviews["require_last_push_approval"])
        self.assertTrue(policy["required_conversation_resolution"])
        self.assertTrue(policy["enforce_admins"])
        self.assertFalse(policy["allow_force_pushes"])
        self.assertFalse(policy["allow_deletions"])

        codeowners = read(".github/CODEOWNERS")
        self.assertIn("* @hemduf", codeowners)
        review_policy = read("docs/ci-and-review-policy.md")
        normalized_policy = " ".join(review_policy.lower().split())
        self.assertIn("solo maintainer", normalized_policy)
        self.assertIn("no github approval is required", normalized_policy)
        self.assertIn("/automerge", review_policy)
        self.assertIn("Required CI gate", review_policy)
        self.assertIn("Allow auto-merge", review_policy)

    def test_auto_merge_requires_explicit_repository_owner_command(self) -> None:
        workflow = read(".github/workflows/auto-merge.yml")
        self.assertIn("issue_comment:", workflow)
        self.assertIn("types: [created]", workflow)
        self.assertIn("github.event.issue.pull_request", workflow)
        self.assertIn("github.event.comment.body == '/automerge'", workflow)
        self.assertIn(
            "github.event.comment.user.login == github.repository_owner", workflow
        )
        self.assertIn("checks: read", workflow)
        self.assertIn("contents: write", workflow)
        self.assertIn("pull-requests: write", workflow)
        self.assertIn("gh pr checks", workflow)
        self.assertIn("Required CI gate", workflow)
        self.assertIn("SUCCESS", workflow)
        self.assertIn("Required CI gate must pass before auto-merge", workflow)
        self.assertIn("gh pr merge --auto --squash", workflow)
        self.assertIn(".allow_auto_merge", workflow)
        self.assertIn("head.repo.full_name", workflow)
        self.assertIn("base.ref", workflow)
        self.assertNotIn("actions/checkout", workflow)
        self.assertNotIn("secrets.", workflow)


if __name__ == "__main__":
    unittest.main()
