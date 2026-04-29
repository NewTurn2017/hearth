#!/usr/bin/env node
// Increment build-number.json's build counter by 1.
//
// CFBundleVersion (Apple's monotonically-increasing build number) is distinct
// from CFBundleShortVersionString (the user-visible "1.0.0"). The App Store
// rejects re-uploads that don't bump CFBundleVersion, even when the marketing
// version is unchanged. This script owns that counter; build-mas.sh patches
// the value into Info.plist via PlistBuddy after `tauri build` and before
// re-signing.
//
// build-number.json is checked in so bumps survive across machines.

import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const path = resolve(ROOT, "build-number.json");

const obj = JSON.parse(readFileSync(path, "utf8"));
if (typeof obj.build !== "number" || !Number.isInteger(obj.build) || obj.build < 1) {
  console.error(`bump-build-number: build-number.json has invalid .build: ${obj.build}`);
  process.exit(1);
}

obj.build += 1;
writeFileSync(path, JSON.stringify(obj, null, 2) + "\n");
console.log(`bump-build-number: ${obj.build}`);
