#!/usr/bin/env python3
"""Download a pinned clap-validator release and validate a built CLAP plugin."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

VERSION = "0.4.1"
BASE_URL = f"https://github.com/free-audio/clap-validator/releases/download/{VERSION}"
ASSETS = {
    "Linux": (
        "clap-validator-0.4.1-127-g152b982-ubuntu-22.04.zip",
        "49edadcfb407ea0dd946ce418300e853fbd2660fa4b0d00e4f19ff8eef24ad90",
    ),
    "Darwin": (
        "clap-validator-0.4.1-127-g152b982-macos-universal.zip",
        "bbec8cd7d18274e549d5d8c12ece3cec54be966129388dd2e742b9957f2ba9f1",
    ),
    "Windows": (
        "clap-validator-0.4.1-127-g152b982-windows.zip",
        "d935c3af0a45c3911ea2e900f4aa5d6709dac82bb485f0c4ce28648ab2cd0c10",
    ),
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def find_plugin(search_root: Path, name: str) -> Path:
    candidates = sorted(search_root.rglob(name), key=lambda path: (len(path.parts), str(path)))
    if not candidates:
        raise SystemExit(f"could not find {name!r} below {search_root}")
    return candidates[0].resolve()


def find_validator(root: Path) -> Path:
    executable = "clap-validator.exe" if os.name == "nt" else "clap-validator"
    candidates = [path for path in root.rglob(executable) if path.is_file()]
    if not candidates:
        raise SystemExit(f"downloaded archive does not contain {executable}")
    path = candidates[0]
    if os.name != "nt":
        path.chmod(path.stat().st_mode | 0o111)
    return path


def downloaded_validator(cache: Path) -> Path:
    system = platform.system()
    if system not in ASSETS:
        raise SystemExit(f"unsupported platform: {system}")
    asset, expected_digest = ASSETS[system]
    root = cache / f"clap-validator-{VERSION}-{system.lower()}"
    marker = root / ".verified"
    if marker.exists():
        return find_validator(root)

    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="clap-validator-download-") as temporary:
        archive = Path(temporary) / asset
        urllib.request.urlretrieve(f"{BASE_URL}/{asset}", archive)
        actual_digest = sha256(archive)
        if actual_digest != expected_digest:
            raise SystemExit(
                f"clap-validator SHA256 mismatch: expected {expected_digest}, got {actual_digest}"
            )
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(root)
    marker.write_text(expected_digest + "\n", encoding="utf-8")
    return find_validator(root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--plugin", type=Path)
    parser.add_argument("--search-root", type=Path)
    parser.add_argument("--plugin-name", default="clapgen_issue55_validation.clap")
    parser.add_argument("--validator", type=Path)
    parser.add_argument("--cache", type=Path, default=Path(".cache/clap-validator"))
    arguments = parser.parse_args()

    if arguments.plugin is None and arguments.search_root is None:
        parser.error("provide --plugin or --search-root")
    plugin = (
        arguments.plugin.resolve()
        if arguments.plugin is not None
        else find_plugin(arguments.search_root.resolve(), arguments.plugin_name)
    )
    if not plugin.exists():
        raise SystemExit(f"plugin does not exist: {plugin}")

    validator = (
        arguments.validator.resolve()
        if arguments.validator is not None
        else downloaded_validator(arguments.cache.resolve())
    )
    command = [str(validator), "validate", str(plugin), "--only-failed"]
    print("+", " ".join(command), flush=True)
    return subprocess.run(command, check=False).returncode


if __name__ == "__main__":
    sys.exit(main())
