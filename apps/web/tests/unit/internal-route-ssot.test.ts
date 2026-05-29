// @vitest-environment node

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const WEB_ROOT = join(process.cwd());
const SCAN_ENTRIES = [
  join(WEB_ROOT, "app"),
  join(WEB_ROOT, "components"),
  join(WEB_ROOT, "lib"),
  join(WEB_ROOT, "proxy.ts"),
];

const INTERNAL_ROUTE_LITERAL =
  /^\/(?:listings|login|profile|forbidden|me\/notifications|api\/auth|api\/proxy|platform-core\/events)(?:\/|$|\?)/;
const ROUTE_SSOT_FILE = "lib/routes.ts";

function collectSourceFiles(path: string): string[] {
  const stat = statSync(path);
  if (stat.isFile()) return path.endsWith(".ts") || path.endsWith(".tsx") ? [path] : [];

  return readdirSync(path).flatMap((entry) => collectSourceFiles(join(path, entry)));
}

function normalizedRelative(path: string): string {
  return relative(WEB_ROOT, path).split(sep).join("/");
}

function isScannedFile(path: string): boolean {
  const normalized = normalizedRelative(path);
  if (normalized === ROUTE_SSOT_FILE) return false;
  if (normalized.endsWith(".test.ts") || normalized.endsWith(".test.tsx")) return false;
  return !normalized.startsWith("app/dev-x9-test/");
}

function formatViolation(
  path: string,
  source: ts.SourceFile,
  node: ts.Node,
  value: string,
): string {
  const { line, character } = source.getLineAndCharacterOfPosition(node.getStart(source));
  return `${normalizedRelative(path)}:${line + 1}:${character + 1} ${value}`;
}

function findRouteLiterals(path: string): string[] {
  const text = readFileSync(path, "utf8");
  const source = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const violations: string[] = [];

  function recordIfRoute(node: ts.Node, value: string): void {
    if (INTERNAL_ROUTE_LITERAL.test(value)) {
      violations.push(formatViolation(path, source, node, value));
    }
  }

  function visit(node: ts.Node): void {
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      recordIfRoute(node, node.text);
    } else if (ts.isTemplateExpression(node)) {
      recordIfRoute(node, node.head.text);
      for (const span of node.templateSpans) {
        recordIfRoute(node, span.literal.text);
      }
    }

    ts.forEachChild(node, visit);
  }

  visit(source);
  return violations;
}

describe("internal route SSOT", () => {
  it("keeps internal app and API routes behind lib/routes.ts", () => {
    const violations = SCAN_ENTRIES.flatMap(collectSourceFiles)
      .filter(isScannedFile)
      .flatMap(findRouteLiterals);

    expect(violations).toEqual([]);
  });
});
