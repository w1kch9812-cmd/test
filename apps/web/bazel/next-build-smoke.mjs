import { existsSync, readFileSync } from "node:fs";

const requiredFiles = [
  ".next/BUILD_ID",
  ".next/build-manifest.json",
  ".next/required-server-files.json",
  ".next/server/app-paths-manifest.json",
  ".next/static",
];

const missingFiles = requiredFiles.filter((path) => !existsSync(path));

if (missingFiles.length > 0) {
  console.error(`next-build-smoke: missing Bazel Next build output: ${missingFiles.join(", ")}`);
  process.exit(1);
}

const buildId = readFileSync(".next/BUILD_ID", "utf8").trim();
if (buildId.length === 0) {
  console.error("next-build-smoke: .next/BUILD_ID is empty");
  process.exit(1);
}
