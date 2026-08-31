# CI, review, and merge policy

`main` is protected by one stable required check named `Required CI gate`. That
job succeeds only when policy linting, every Rust runner, every Debug/Release
C++ runner, and every supported sanitizer runner succeed. A deliberately
failing test therefore makes the aggregate gate fail and blocks merge.

The canonical branch-protection payload is `.github/branch-protection.json`.
Repository administrators must keep the live `main` protection equivalent to
that file: strict required checks, one approving CODEOWNER review, stale-review
dismissal, last-push approval, resolved review conversations, no force pushes,
and no branch deletion. The policy applies to administrators as well.

## Independent review

A pull request must never self-approve. The repository uses `@hemduf` as the
current CODEOWNER. For automated development, the intended author identity is
`clap-gen-dev[bot]` and the independent automated reviewer identity is
`clap-gen-reviewer[bot]`. Those identities represent separate GitHub App
installations/credentials. They must not share credentials, installation
ownership, or approval state. A human CODEOWNER approval may substitute for the
reviewer bot.

Bot-authored pull requests are not eligible for merge until an approval comes
from a different GitHub identity than the pull-request author. A new push must
invalidate stale approval through branch protection.

## Pull requests from forks

The `CI` workflow runs on `pull_request` with only `contents: read`; it neither
references repository secrets nor grants write permissions. Untrusted fork code
therefore runs only with the read-only token GitHub provides to pull-request
workflows.

The conditional auto-merge workflow never checks out pull-request code. Its
write-capable job is skipped unless the pull request head repository is exactly
the base repository, so fork pull requests never execute a write-capable job.

## Auto-merge

After a same-repository pull request receives an independent approval, the
`Conditional auto-merge` workflow asks GitHub to enable native squash
auto-merge. It does not merge by bypassing repository rules. GitHub keeps the
pull request pending until `Required CI gate`, CODEOWNER approval, stale-review
rules, and conversation-resolution requirements are all satisfied.

This workflow intentionally reacts to approval rather than CI completion:
approval before CI enables auto-merge and waits for CI; approval after CI
enables auto-merge once all branch requirements are already green.

## Dependency caches and artifacts

Cargo registry, git, and target caches are keyed by operating system and the
locked dependency graph. Restored cache contents are an optimization only;
`cargo ... --locked` remains authoritative. CTest and sanitizer logs are
uploaded only on failure and retained for seven days.

## Applying branch protection

The JSON file is an auditable source of truth for repository settings, but it
is not itself enforcement. Apply it with repository-administration credentials
using the GitHub branch protection API for `main`, then verify the live settings
match the file before enabling unattended merge automation.
