import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const required = [
  "package.json", "vite.config.ts", "src/App.svelte", "src-tauri/Cargo.toml",
  "src-tauri/tauri.conf.json", "src-tauri/src/lib.rs", "src-tauri/src/market/mod.rs",
  "src-tauri/src/portfolio/mod.rs", "src-tauri/src/alerts/mod.rs", "src-tauri/src/tray/mod.rs",
  "src-tauri/migrations/0001_initial.sql", "docs/ARCHITECTURE.md", "docs/TEST_STRATEGY.md",
  "scripts/BUILD_WINDOWS.ps1", "scripts/TEST_WINDOWS.ps1"
];

const errors = [];
for (const rel of required) {
  if (!fs.existsSync(path.join(root, rel))) errors.push(`missing required file: ${rel}`);
}

for (const rel of ["package.json", "src-tauri/tauri.conf.json", "src-tauri/capabilities/default.json", "tsconfig.json"]) {
  try { JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")); }
  catch (e) { errors.push(`invalid JSON ${rel}: ${e.message}`); }
}

const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const deps = { ...pkg.dependencies, ...pkg.devDependencies };
for (const forbidden of ["electron", "express", "fastify"]) {
  if (deps[forbidden]) errors.push(`forbidden dependency present: ${forbidden}`);
}
if (!deps["@tauri-apps/api"]) errors.push("Tauri JS API dependency missing");
if (!deps.svelte) errors.push("Svelte dependency missing");

const cargo = fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8");
if (!/tauri\s*=.*tray-icon/.test(cargo)) errors.push("Tauri tray-icon feature is not enabled");
if (!/rusqlite/.test(cargo)) errors.push("rusqlite dependency missing");

const strategy = fs.readFileSync(path.join(root, "docs/TEST_STRATEGY.md"), "utf8");
if (!strategy.includes("Phase 8: Windows qualification")) errors.push("Windows-only qualification deferral missing");

if (errors.length) {
  console.error("Phase 0 validation FAILED");
  for (const e of errors) console.error(` - ${e}`);
  process.exit(1);
}
console.log(`Phase 0 validation PASS (${required.length} required artifacts checked)`);
