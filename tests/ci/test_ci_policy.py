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

    def test_review_policy_requires_independent_approval(self) -> None:
        policy = json.loads(read(".github/branch-protection.json"))
        reviews = policy["required_pull_request_reviews"]
        self.assertEqual(1, reviews["required_approving_review_count"])
        self.assertTrue(reviews["dismiss_stale_reviews"])
        self.assertTrue(reviews["require_code_owner_reviews"])
        self.assertTrue(reviews["require_last_push_approval"])
        self.assertTrue(policy["required_conversation_resolution"])
        self.assertTrue(policy["enforce_admins"])
        self.assertFalse(policy["allow_force_pushes"])
        self.assertFalse(policy["allow_deletions"])

        codeowners = read(".github/CODEOWNERS")
        self.assertIn("* @hemduf", codeowners)
        review_policy = read("docs/ci-and-review-policy.md")
        self.assertIn("clap-gen-dev[bot]", review_policy)
        self.assertIn("clap-gen-reviewer[bot]", review_policy)
        self.assertIn("must never self-approve", review_policy.lower())
        self.assertIn("Required CI gate", review_policy)

    def test_auto_merge_only_enables_after_independent_same_repo_approval(self) -> None:
        workflow = read(".github/workflows/auto-merge.yml")
        self.assertIn("pull_request_review:", workflow)
        self.assertIn("types: [submitted]", workflow)
        self.assertIn("github.event.review.state == 'approved'", workflow)
        self.assertIn(
            "github.event.review.user.login != github.event.pull_request.user.login",
            workflow,
        )
        self.assertIn(
            "github.event.pull_request.head.repo.full_name == github.repository", workflow
        )
        self.assertIn("contents: write", workflow)
        self.assertIn("pull-requests: write", workflow)
        self.assertIn("gh pr merge --auto --squash", workflow)
        self.assertNotIn("actions/checkout", workflow)
        self.assertNotIn("secrets.", workflow)


if __name__ == "__main__":
    unittest.main()
