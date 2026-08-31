# CI, review, and merge policy

`main` is protected by one stable required check named `Required CI gate`. That
job succeeds only when policy linting, every Rust runner, every Debug/Release
C++ runner, and every supported sanitizer runner succeed. A deliberately
failing test therefore makes the aggregate gate fail and blocks merge.

The canonical branch-protection payload is `.github/branch-protection.json`.
Repository administrators must keep the live `main` protection equivalent to
that file: strict required checks, pull requests required, resolved review
conversations, no force pushes, no branch deletion, and linear history. The
policy applies to administrators as well.

## Solo maintainer authorization

This repository currently has a solo maintainer. GitHub does not allow the
author of a pull request to approve that same pull request, so no GitHub
approval is required by branch protection. CODEOWNERS documents ownership, but
CODEOWNER review is not a merge gate.

Maintainer validation is explicit: after reviewing a pull request, the repository
owner comments exactly `/automerge` on that pull request. The conditional
auto-merge workflow accepts that command only when the comment author is the
GitHub repository owner.

The command does not bypass CI. It only asks GitHub to enable native squash
auto-merge; GitHub still waits until `Required CI gate` passes, the branch is up
to date, and required review conversations are resolved.

If this repository later gains additional maintainers, this policy should be
revisited and can move back to required independent approvals.

## Pull requests from forks

The `CI` workflow runs on `pull_request` with only `contents: read`; it neither
references repository secrets nor grants write permissions. Untrusted fork code
therefore runs only with the read-only token GitHub provides to pull-request
workflows.

The conditional auto-merge workflow runs from a trusted issue-comment event and
never checks out pull-request code. Before enabling auto-merge it fetches the PR
metadata and requires the head repository to be exactly this repository and the
base branch to be `main`.

## Auto-merge

GitHub repository setting **Allow auto-merge** must be enabled before this
workflow can request native auto-merge. The workflow checks the live repository
setting first and fails with an explicit diagnostic instead of silently falling
back to a direct merge.

The explicit `/automerge` owner command can be issued before or after CI
finishes. If issued before CI, GitHub waits for `Required CI gate`; if issued
after CI, GitHub merges as soon as all live branch-protection requirements are
satisfied.

## Dependency caches and artifacts

Cargo registry, git, and target caches are keyed by operating system and the
locked dependency graph. Restored cache contents are an optimization only;
`cargo ... --locked` remains authoritative. CTest and sanitizer logs are
uploaded only on failure and retained for seven days.

## Applying branch protection

The JSON file is an auditable source of truth for repository settings, but it
is not itself enforcement. The live `main` rule should require pull requests,
`Required CI gate`, up-to-date branches, conversation resolution, linear
history, and should reject force pushes and branch deletion. No approving
review is required while the project has a single maintainer.
