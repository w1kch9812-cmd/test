#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from pathlib import Path


WORKSPACE_PATTERN_RE = re.compile(r"^\s*-\s*[\"']?([^\"'#]+)[\"']?\s*(?:#.*)?$")


def workspace_patterns(root: Path) -> list[str]:
    workspace_file = root / "pnpm-workspace.yaml"
    if not workspace_file.is_file():
        print(f"missing workspace file: {workspace_file}", file=sys.stderr)
        raise SystemExit(1)

    patterns: list[str] = []
    for line in workspace_file.read_text(encoding="utf-8").splitlines():
        match = WORKSPACE_PATTERN_RE.match(line)
        if match:
            patterns.append(match.group(1).strip())
    return patterns


def workspace_package_jsons(root: Path) -> list[Path]:
    packages: set[Path] = set()
    for pattern in workspace_patterns(root):
        for package_dir in root.glob(pattern):
            package_json = package_dir / "package.json"
            if package_json.is_file():
                packages.add(package_json)
    return sorted(packages)


def main(argv: list[str]) -> int:
    root = Path(argv[1]).resolve() if len(argv) > 1 else Path.cwd().resolve()
    package_jsons = workspace_package_jsons(root)
    missing: list[Path] = []

    for package_json in package_jsons:
        with package_json.open(encoding="utf-8") as file:
            manifest = json.load(file)
        scripts = manifest.get("scripts")
        if not isinstance(scripts, dict) or not scripts.get("typecheck"):
            missing.append(package_json)

    for package_json in missing:
        print(f"missing typecheck script: {package_json.relative_to(root).as_posix()}", file=sys.stderr)

    if missing:
        print(
            f"workspace-typecheck-coverage-failed packages={len(package_jsons)} missing={len(missing)}",
            file=sys.stderr,
        )
        return 1

    print(f"workspace-typecheck-coverage-ok packages={len(package_jsons)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
