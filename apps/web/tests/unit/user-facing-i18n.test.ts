// @vitest-environment node

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const WEB_ROOT = join(process.cwd());
const REPO_ROOT = join(WEB_ROOT, "..", "..");
const SCAN_ROOTS = [
  join(WEB_ROOT, "app"),
  join(WEB_ROOT, "components"),
  join(WEB_ROOT, "lib", "panel"),
  join(REPO_ROOT, "packages", "ui", "primitives"),
];
const HANGUL = /[가-힣]/;

function collectTsxFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) return collectTsxFiles(path);
    return path.endsWith(".tsx") ? [path] : [];
  });
}

function isProductionSurfaceFile(path: string): boolean {
  const normalized = relative(WEB_ROOT, path).split(sep).join("/");
  return !normalized.startsWith("app/dev-x9-test/");
}

function formatViolation(
  path: string,
  source: ts.SourceFile,
  node: ts.Node,
  value: string,
): string {
  const { line, character } = source.getLineAndCharacterOfPosition(node.getStart(source));
  const normalized = relative(REPO_ROOT, path).split(sep).join("/");
  const excerpt = value.trim().replace(/\s+/g, " ");
  return `${normalized}:${line + 1}:${character + 1} ${excerpt}`;
}

function findHardcodedKorean(path: string): string[] {
  const text = readFileSync(path, "utf8");
  const source = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const violations: string[] = [];

  function visit(node: ts.Node): void {
    if (ts.isJsxText(node) && HANGUL.test(node.getText(source))) {
      violations.push(formatViolation(path, source, node, node.getText(source)));
    }

    if (
      (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) &&
      HANGUL.test(node.text)
    ) {
      violations.push(formatViolation(path, source, node, node.text));
    }

    ts.forEachChild(node, visit);
  }

  visit(source);
  return violations;
}

describe("user-facing i18n contract", () => {
  it("keeps Korean TSX text in typed i18n messages", () => {
    const violations = SCAN_ROOTS.flatMap(collectTsxFiles)
      .filter(isProductionSurfaceFile)
      .flatMap(findHardcodedKorean);

    expect(violations).toEqual([]);
  });
});
