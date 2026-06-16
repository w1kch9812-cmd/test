const args = process.argv.slice(2);

if (
  (args[0] === "build" || args[0] === "dev") &&
  !args.includes("--webpack") &&
  !args.includes("--turbopack")
) {
  process.argv = [process.argv[0], process.argv[1], args[0], "--webpack", ...args.slice(1)];
}

await import("next/dist/bin/next");
