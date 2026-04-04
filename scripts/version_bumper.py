#!/usr/bin/env python3
"""
Bump the project release version across Flutter pubspec.yaml files and Rust workspace crates.

Usage:
  ./scripts/version_bumper.py 1.2.3
  ./scripts/version_bumper.py 1.0.0-beta.2 --target rust
  ./scripts/version_bumper.py 2.0.0 --target flutter --include-cargokit
  ./scripts/version_bumper.py 1.0.0 --dry-run
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# Repo root = parent of scripts/
REPO_ROOT = Path(__file__).resolve().parent.parent

PUBSPEC_DEFAULT = [
    REPO_ROOT / "pubspec.yaml",
    REPO_ROOT / "rust_builder" / "pubspec.yaml",
]

PUBSPEC_CARGOKIT_BUILD_TOOL = REPO_ROOT / "rust_builder" / "cargokit" / "build_tool" / "pubspec.yaml"

# Workspace member crates (rust/Cargo.toml members); root workspace Cargo.toml has no [package].version
RUST_CRATE_MANIFESTS = sorted(REPO_ROOT.glob("rust/karbeat-*/Cargo.toml"))

PUBSPEC_VERSION_LINE = re.compile(r"^version:\s*.+$", re.MULTILINE)
RUST_PACKAGE_VERSION_LINE = re.compile(r"^version\s*=\s*\"[^\"]+\"\s*$", re.MULTILINE)


def bump_pubspec(text: str, new_version: str) -> str:
    if not PUBSPEC_VERSION_LINE.search(text):
        raise ValueError("no top-level version: line found")
    return PUBSPEC_VERSION_LINE.sub(f"version: {new_version}", text, count=1)


def bump_rust_package_version(text: str, new_version: str) -> str:
    if not RUST_PACKAGE_VERSION_LINE.search(text):
        raise ValueError('no package version = "..." line found')
    return RUST_PACKAGE_VERSION_LINE.sub(f'version = "{new_version}"', text, count=1)


def process_file(path: Path, new_version: str, kind: str, dry_run: bool) -> bool:
    text = path.read_text(encoding="utf-8")
    if kind == "pubspec":
        new_text = bump_pubspec(text, new_version)
    elif kind == "rust":
        new_text = bump_rust_package_version(text, new_version)
    else:
        raise ValueError(kind)

    if new_text == text:
        return False
    rel = path.relative_to(REPO_ROOT)
    if dry_run:
        print(f"[dry-run] would update {rel}")
    else:
        path.write_text(new_text, encoding="utf-8", newline="\n")
        print(f"updated {rel}")
    return True


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Set the same release version on pubspec.yaml files and Rust crate manifests.",
    )
    parser.add_argument(
        "version",
        help='Release version (e.g. "1.2.3" or "1.0.0-alpha.2").',
    )
    parser.add_argument(
        "--target",
        "-t",
        choices=("all", "flutter", "rust"),
        default="all",
        help="Only bump Flutter pubspec files, only Rust crates, or both (default: all).",
    )
    parser.add_argument(
        "--include-cargokit",
        action="store_true",
        help=f"Also bump {PUBSPEC_CARGOKIT_BUILD_TOOL.relative_to(REPO_ROOT)} (vendored Cargokit build_tool).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print paths that would change without writing files.",
    )
    args = parser.parse_args()
    new_v = args.version.strip()
    if not new_v or any(c in new_v for c in "\n\r\t"):
        print("error: version must be a non-empty single-line string", file=sys.stderr)
        return 2

    pubspec_paths = list(PUBSPEC_DEFAULT)
    if args.include_cargokit:
        pubspec_paths.append(PUBSPEC_CARGOKIT_BUILD_TOOL)

    changed = 0
    errors: list[str] = []

    if args.target in ("all", "flutter"):
        for p in pubspec_paths:
            if not p.is_file():
                errors.append(f"missing file: {p.relative_to(REPO_ROOT)}")
                continue
            try:
                if process_file(p, new_v, "pubspec", args.dry_run):
                    changed += 1
            except ValueError as e:
                errors.append(f"{p.relative_to(REPO_ROOT)}: {e}")

    if args.target in ("all", "rust"):
        if not RUST_CRATE_MANIFESTS:
            errors.append("no Rust manifests matched rust/karbeat-*/Cargo.toml")
        for p in RUST_CRATE_MANIFESTS:
            try:
                if process_file(p, new_v, "rust", args.dry_run):
                    changed += 1
            except ValueError as e:
                errors.append(f"{p.relative_to(REPO_ROOT)}: {e}")

    for msg in errors:
        print(f"error: {msg}", file=sys.stderr)

    if errors:
        return 1
    if changed == 0 and not args.dry_run:
        print("nothing to change (versions already match)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
