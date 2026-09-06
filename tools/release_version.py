#!/usr/bin/env python3
"""Read and validate clap-gen's canonical SemVer version from Cargo.toml."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "Cargo.toml"

SEMVER_RE = re.compile(
    r"^(?P<major>0|[1-9][0-9]*)\."
    r"(?P<minor>0|[1-9][0-9]*)\."
    r"(?P<patch>0|[1-9][0-9]*)"
    r"(?:-(?P<prerelease>"
    r"(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*"
    r"))?"
    r"(?:\+(?P<build>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)


@dataclass(frozen=True)
class SemVer:
    raw: str
    major: int
    minor: int
    patch: int
    prerelease: str | None
    build: str | None

    @property
    def core(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"


def workspace_version(manifest: Path = DEFAULT_MANIFEST) -> str:
    section: str | None = None
    version: str | None = None

    for raw_line in manifest.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        section_match = re.fullmatch(r"\[([^]]+)]", line)
        if section_match:
            section = section_match.group(1)
            continue
        if section != "workspace.package":
            continue

        version_match = re.fullmatch(r'version\s*=\s*"([^"]+)"\s*', line)
        if version_match:
            if version is not None:
                raise ValueError("multiple versions found in [workspace.package]")
            version = version_match.group(1)

    if version is None:
        raise ValueError("missing version in [workspace.package]")
    return version


def parse_semver(version: str) -> SemVer:
    match = SEMVER_RE.fullmatch(version)
    if not match:
        raise ValueError(f"invalid SemVer version: {version!r}")
    return SemVer(
        raw=version,
        major=int(match.group("major")),
        minor=int(match.group("minor")),
        patch=int(match.group("patch")),
        prerelease=match.group("prerelease"),
        build=match.group("build"),
    )


def validate_tag(version: SemVer, tag: str) -> None:
    expected = f"v{version.raw}"
    if tag != expected:
        raise ValueError(f"tag/version mismatch: expected {expected!r}, got {tag!r}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest",
        type=Path,
        default=DEFAULT_MANIFEST,
        help="Cargo.toml containing [workspace.package].version",
    )
    parser.add_argument("--tag", help="validate an exact vX.Y.Z tag against the manifest")
    parser.add_argument(
        "--cmake-core",
        action="store_true",
        help="print only the numeric X.Y.Z core accepted by CMake project(VERSION)",
    )
    args = parser.parse_args(argv)

    try:
        version = parse_semver(workspace_version(args.manifest))
        if args.tag is not None:
            validate_tag(version, args.tag)
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        return 1

    print(version.core if args.cmake_core else version.raw)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
