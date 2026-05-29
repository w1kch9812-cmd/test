#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


INLINE_LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
REFERENCE_LINK_RE = re.compile(r"^\s*\[[^\]]+\]:\s*(\S+)", re.MULTILINE)
FENCED_CODE_RE = re.compile(r"```.*?```|~~~.*?~~~", re.DOTALL)
INLINE_CODE_RE = re.compile(r"`[^`\n]*`")
ROOT_FILES = ("README.md", "AGENTS.md", "TECH.md", "MEMORY.md")
SKIPPED_SCHEMES = {"http", "https", "mailto", "tel", "sms"}
EXCLUDED_DOC_PREFIXES = (
    Path("docs/superpowers/plans"),
    Path("docs/superpowers/specs"),
)


def iter_markdown_files(root: Path) -> list[Path]:
    files: list[Path] = []
    docs = root / "docs"
    if docs.is_dir():
        for file in sorted(docs.rglob("*.md")):
            relative = file.relative_to(root)
            if any(relative.is_relative_to(prefix) for prefix in EXCLUDED_DOC_PREFIXES):
                continue
            files.append(file)
    for name in ROOT_FILES:
        path = root / name
        if path.is_file():
            files.append(path)
    return files


def iter_link_targets(markdown: str) -> list[str]:
    markdown_without_code = FENCED_CODE_RE.sub("", markdown)
    markdown_without_code = INLINE_CODE_RE.sub("", markdown_without_code)
    targets = [match.group(1) for match in INLINE_LINK_RE.finditer(markdown_without_code)]
    targets.extend(match.group(1) for match in REFERENCE_LINK_RE.finditer(markdown_without_code))
    return targets


def local_path_target(raw_target: str) -> str | None:
    target = raw_target.strip().strip("<>")
    if not target or target.startswith("#"):
        return None

    parsed = urlsplit(target)
    if parsed.scheme.lower() in SKIPPED_SCHEMES:
        return None
    if parsed.scheme or parsed.netloc:
        return None

    path = unquote(parsed.path)
    if not path:
        return None
    return path


def resolve_target(root: Path, source: Path, target_path: str) -> Path:
    if target_path.startswith("/"):
        return (root / target_path.lstrip("/")).resolve()
    return (source.parent / target_path).resolve()


def is_inside_root(root: Path, path: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def main(argv: list[str]) -> int:
    root = Path(argv[1]).resolve() if len(argv) > 1 else Path.cwd().resolve()
    files = iter_markdown_files(root)
    checked_links = 0
    broken: list[tuple[Path, str, Path]] = []

    for file in files:
        markdown = file.read_text(encoding="utf-8")
        for raw_target in iter_link_targets(markdown):
            target_path = local_path_target(raw_target)
            if target_path is None:
                continue

            resolved = resolve_target(root, file, target_path)
            if not is_inside_root(root, resolved):
                continue

            checked_links += 1
            if not resolved.exists():
                broken.append((file, raw_target, resolved))

    for file, raw_target, resolved in broken:
        source = file.relative_to(root).as_posix()
        missing = resolved.relative_to(root).as_posix() if is_inside_root(root, resolved) else str(resolved)
        print(f"broken markdown link: {source} -> {raw_target} (resolved: {missing})", file=sys.stderr)

    if broken:
        print(f"markdown-links-failed files={len(files)} links={checked_links} broken={len(broken)}", file=sys.stderr)
        return 1

    print(f"markdown-links-ok files={len(files)} links={checked_links}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
