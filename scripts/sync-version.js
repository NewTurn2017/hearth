#!/usr/bin/env node
// Sync the canonical version (package.json) into every other manifest the
// MAS build pipeline reads — the Tauri config and the 3 Cargo crates.
// Idempotent: if everything is already in sync, this is a no-op + clean exit.
//
// Source of truth: package.json (.version)
// Targets:
//   - src-tauri/app/tauri.conf.json (.version)
//   - src-tauri/app/Cargo.toml      ([package] version)
//   - src-tauri/core/Cargo.toml     ([package] version)
//
// hearth-cli has an independent release cadence (currently at 1.0.0 GA while
// the app is at 0.9.x) and is intentionally NOT synced here. Use
// scripts/bump-version.sh if you ever need to bump everything together.

import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pkgPath = resolve(ROOT, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const VERSION = pkg.version;

if (!/^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$/.test(VERSION)) {
  console.error(`sync-version: package.json version is not semver: ${VERSION}`);
  process.exit(1);
}

let touched = 0;

function patchJson(rel, mutate) {
  const p = resolve(ROOT, rel);
  const obj = JSON.parse(readFileSync(p, "utf8"));
  const before = JSON.stringify(obj);
  mutate(obj);
  const after = JSON.stringify(obj, null, 2) + "\n";
  if (JSON.stringify(JSON.parse(after)) === before) return;
  writeFileSync(p, after);
  console.log(`sync-version: updated ${rel} → ${VERSION}`);
  touched++;
}

function patchCargoToml(rel) {
  const p = resolve(ROOT, rel);
  const text = readFileSync(p, "utf8");
  const re = /(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/;
  const m = text.match(re);
  if (!m) {
    console.error(`sync-version: no [package] version in ${rel}`);
    process.exit(1);
  }
  if (m[2] === VERSION) return;
  const next = text.replace(re, `$1${VERSION}$3`);
  writeFileSync(p, next);
  console.log(`sync-version: updated ${rel} → ${VERSION}`);
  touched++;
}

patchJson("src-tauri/app/tauri.conf.json", (o) => {
  o.version = VERSION;
});

patchCargoToml("src-tauri/app/Cargo.toml");
patchCargoToml("src-tauri/core/Cargo.toml");

if (touched === 0) {
  console.log(`sync-version: all manifests already at ${VERSION}`);
}
