import fs from "node:fs";
const tag = process.argv[2];
if (!/^v\d+\.\d+\.\d+$/.test(tag || "")) throw Error("Expected a vX.Y.Z tag");
const expected = tag.slice(1);
for (const path of [
  "tauri/package.json",
  "tauri/package-lock.json",
  "tauri/src-tauri/tauri.conf.json",
  "next/package.json",
  "next/package-lock.json",
]) {
  if (JSON.parse(fs.readFileSync(path, "utf8")).version !== expected)
    throw Error(`Version mismatch in ${path}`);
}
const cargo = fs
  .readFileSync("tauri/src-tauri/Cargo.toml", "utf8")
  .match(/^version = "([^"]+)"/m)?.[1];
const lock = fs
  .readFileSync("tauri/src-tauri/Cargo.lock", "utf8")
  .match(/name = "internet-swap"\nversion = "([^"]+)"/)?.[1];
if (cargo !== expected || lock !== expected)
  throw Error("Cargo version mismatch");
if (!fs.existsSync(`release/${expected}.md`))
  throw Error("Release notes are missing");
console.log(`Release versions agree: ${expected}`);
