#!/usr/bin/env bun
// @ts-nocheck -- runs under bun; main tsconfig (include: ["src"]) lacks bun/node types
/**
 * scripts/seed-screenshots.ts
 *
 * Populate a *throwaway* Hearth SQLite database with believable demo data so
 * we can capture App Store screenshots without exposing real user content.
 *
 * Spec: docs/superpowers/specs/2026-04-26-app-store-submission-design.md (§5.1)
 *
 * SAFETY MODEL — read before editing
 * ----------------------------------
 *  - The user's live DB lives at:
 *        ~/Library/Application Support/com.codewithgenie.hearth/data.db
 *    A backup of that DB exists at ~/.hearth-backup-pre-screenshots-20260426/
 *    BUT THIS SCRIPT MUST NEVER WRITE TO THE LIVE PATH.
 *  - The script aborts loudly if --target resolves to that live path (or a
 *    realpath-equivalent). No "are you sure?" override. Re-run with a
 *    different --target if you really meant something else.
 *  - --target is required. The only sugar is --locale=en|ko which expands to
 *    ~/.hearth-screenshot-seed-en.db / ~/.hearth-screenshot-seed-ko.db.
 *  - --dry-run prints what would be inserted without touching the filesystem.
 *
 * Run:
 *     bun scripts/seed-screenshots.ts --locale=en
 *     bun scripts/seed-screenshots.ts --locale=ko
 *     bun scripts/seed-screenshots.ts --target=/tmp/foo.db --locale=en
 *     bun scripts/seed-screenshots.ts --locale=en --dry-run
 *
 * After seeding, point a Hearth dev build at the seeded DB by symlinking or
 * copying it into ~/Library/Application Support/com.codewithgenie.hearth/data.db
 * AFTER confirming the live DB is backed up. (That swap is a manual, deliberate
 * step — this script never performs it.)
 */

import { Database } from "bun:sqlite";
import { existsSync, mkdirSync, realpathSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Locale = "en" | "ko";

interface SeedCategory {
  name: string;
  color: string;
}
interface SeedProject {
  priority: string;
  number: number;
  name: string;
  category: string;
  evaluation: string;
}
interface SeedMemo {
  project: number | null; // 1-based index into projects[]; null = inbox
  color: string;
  content: string;
}
interface SeedSchedule {
  date: string;
  time: string | null;
  description: string;
  location: string | null;
  notes: string | null;
}
interface SeedFile {
  locale: Locale;
  today: string;
  categories: SeedCategory[];
  projects: SeedProject[];
  memos: SeedMemo[];
  schedules: SeedSchedule[];
}

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

interface Args {
  target?: string;
  locale?: Locale;
  dryRun: boolean;
  help: boolean;
}

function parseArgs(argv: string[]): Args {
  const out: Args = { dryRun: false, help: false };
  for (const raw of argv) {
    if (raw === "--help" || raw === "-h") {
      out.help = true;
      continue;
    }
    if (raw === "--dry-run") {
      out.dryRun = true;
      continue;
    }
    const eq = raw.indexOf("=");
    if (eq === -1) {
      throw new Error(
        `unrecognized argument: ${raw}. Use --target=<path> --locale=en|ko [--dry-run]`,
      );
    }
    const key = raw.slice(0, eq);
    const val = raw.slice(eq + 1);
    if (key === "--target") {
      out.target = val;
    } else if (key === "--locale") {
      if (val !== "en" && val !== "ko") {
        throw new Error(`--locale must be 'en' or 'ko' (got ${val})`);
      }
      out.locale = val;
    } else {
      throw new Error(`unknown flag: ${key}`);
    }
  }
  return out;
}

function printUsage() {
  console.error(`Usage:
  bun scripts/seed-screenshots.ts --locale=en|ko [--target=<path>] [--dry-run]

  --locale     en | ko  (REQUIRED — picks which seed-data/*.json to load)
  --target     Path to the SQLite DB to write. Default when --locale is set:
                 en -> ~/.hearth-screenshot-seed-en.db
                 ko -> ~/.hearth-screenshot-seed-ko.db
               Must NOT resolve to the live Hearth DB
               (~/Library/Application Support/com.codewithgenie.hearth/data.db).
  --dry-run    Print what would be inserted; do not write any files.
  -h, --help   Show this message.
`);
}

// ---------------------------------------------------------------------------
// Safety guard
// ---------------------------------------------------------------------------

function expandHome(p: string): string {
  if (p === "~") return homedir();
  if (p.startsWith("~/")) return join(homedir(), p.slice(2));
  return p;
}

function liveDbCandidates(): string[] {
  const home = homedir();
  // Production path used by tauri's app_data_dir() on macOS:
  //   ~/Library/Application Support/com.codewithgenie.hearth/data.db
  // (Confirmed in src-tauri/cli/src/db.rs and src-tauri/app/src/lib.rs.)
  const macPath = join(
    home,
    "Library",
    "Application Support",
    "com.codewithgenie.hearth",
    "data.db",
  );
  return [macPath];
}

function assertNotLiveDb(target: string): void {
  const resolvedTarget = resolve(target);
  const candidates = liveDbCandidates();
  for (const live of candidates) {
    if (resolvedTarget === resolve(live)) {
      throw new Error(refuseLiveMsg(resolvedTarget, live));
    }
    // Compare realpaths if both exist (catches symlinks / hardlinks).
    try {
      const targetReal = existsSync(resolvedTarget)
        ? realpathSync(resolvedTarget)
        : resolvedTarget;
      const liveReal = existsSync(live) ? realpathSync(live) : live;
      if (targetReal === liveReal) {
        throw new Error(refuseLiveMsg(targetReal, liveReal));
      }
    } catch (e) {
      // realpath can fail on missing parents; fallthrough is fine — the
      // string compare above already covered the common case.
      if (e instanceof Error && e.message.startsWith("REFUSING")) throw e;
    }
  }
}

function refuseLiveMsg(target: string, live: string): string {
  return `REFUSING TO SEED INTO LIVE HEARTH DB.
  --target resolved to: ${target}
  Live DB path:         ${live}

The screenshot seeder MUST NOT touch the user's real database.
Pick a different --target (e.g. ~/.hearth-screenshot-seed-en.db) and try again.`;
}

// ---------------------------------------------------------------------------
// Schema (mirrors src-tauri/core/src/db.rs run_migrations)
// ---------------------------------------------------------------------------

const SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    priority TEXT NOT NULL DEFAULT 'P4',
    number INTEGER,
    name TEXT NOT NULL,
    category TEXT,
    path TEXT,
    evaluation TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS schedules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    time TEXT,
    location TEXT,
    description TEXT,
    notes TEXT,
    remind_before_5min INTEGER NOT NULL DEFAULT 0,
    remind_at_start INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS memos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL DEFAULT '',
    color TEXT NOT NULL DEFAULT 'yellow',
    project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS clients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    company_name TEXT,
    ceo TEXT,
    phone TEXT,
    fax TEXT,
    email TEXT,
    offices TEXT,
    project_desc TEXT,
    status TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    color      TEXT    NOT NULL DEFAULT '#6b7280',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          TEXT    NOT NULL DEFAULT (datetime('now')),
    source      TEXT    NOT NULL,
    op          TEXT    NOT NULL,
    table_name  TEXT    NOT NULL,
    row_id      INTEGER,
    before_json TEXT,
    after_json  TEXT,
    undone      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_audit_ts     ON audit_log(ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_undone ON audit_log(undone, ts DESC);

CREATE VIRTUAL TABLE IF NOT EXISTS projects_fts USING fts5(name, category, evaluation);
CREATE TRIGGER IF NOT EXISTS projects_ai AFTER INSERT ON projects BEGIN
    INSERT INTO projects_fts(rowid, name, category, evaluation)
    VALUES (new.id, new.name, COALESCE(new.category,''), COALESCE(new.evaluation,''));
END;
CREATE TRIGGER IF NOT EXISTS projects_ad AFTER DELETE ON projects BEGIN
    DELETE FROM projects_fts WHERE rowid = old.id;
END;
CREATE TRIGGER IF NOT EXISTS projects_au AFTER UPDATE ON projects BEGIN
    DELETE FROM projects_fts WHERE rowid = old.id;
    INSERT INTO projects_fts(rowid, name, category, evaluation)
    VALUES (new.id, new.name, COALESCE(new.category,''), COALESCE(new.evaluation,''));
END;

CREATE VIRTUAL TABLE IF NOT EXISTS memos_fts USING fts5(content);
CREATE TRIGGER IF NOT EXISTS memos_ai AFTER INSERT ON memos BEGIN
    INSERT INTO memos_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE TRIGGER IF NOT EXISTS memos_ad AFTER DELETE ON memos BEGIN
    DELETE FROM memos_fts WHERE rowid = old.id;
END;
CREATE TRIGGER IF NOT EXISTS memos_au AFTER UPDATE ON memos BEGIN
    DELETE FROM memos_fts WHERE rowid = old.id;
    INSERT INTO memos_fts(rowid, content) VALUES (new.id, new.content);
END;

CREATE VIRTUAL TABLE IF NOT EXISTS schedules_fts USING fts5(description, location, notes);
CREATE TRIGGER IF NOT EXISTS schedules_ai AFTER INSERT ON schedules BEGIN
    INSERT INTO schedules_fts(rowid, description, location, notes)
    VALUES (new.id, COALESCE(new.description,''), COALESCE(new.location,''), COALESCE(new.notes,''));
END;
CREATE TRIGGER IF NOT EXISTS schedules_ad AFTER DELETE ON schedules BEGIN
    DELETE FROM schedules_fts WHERE rowid = old.id;
END;
CREATE TRIGGER IF NOT EXISTS schedules_au AFTER UPDATE ON schedules BEGIN
    DELETE FROM schedules_fts WHERE rowid = old.id;
    INSERT INTO schedules_fts(rowid, description, location, notes)
    VALUES (new.id, COALESCE(new.description,''), COALESCE(new.location,''), COALESCE(new.notes,''));
END;
`;

// ---------------------------------------------------------------------------
// Seeder
// ---------------------------------------------------------------------------

function loadSeed(locale: Locale): SeedFile {
  const here = dirname(fileURLToPath(import.meta.url));
  const path = join(here, "seed-data", `${locale}.json`);
  const file = Bun.file(path);
  if (!file.size) throw new Error(`seed file not found: ${path}`);
  const json = JSON.parse(require("node:fs").readFileSync(path, "utf8")) as SeedFile;
  // Sanity checks tied to the spec.
  if (json.projects.length !== 5) {
    throw new Error(`${path}: expected 5 projects, got ${json.projects.length}`);
  }
  if (json.memos.length !== 12) {
    throw new Error(`${path}: expected 12 memos, got ${json.memos.length}`);
  }
  if (json.schedules.length !== 8) {
    throw new Error(`${path}: expected 8 schedules, got ${json.schedules.length}`);
  }
  return json;
}

function applySeed(db: Database, seed: SeedFile): void {
  db.exec("BEGIN");
  try {
    // Wipe tables (this is a fresh-or-overwritten temp DB by definition).
    db.exec(
      "DELETE FROM memos; DELETE FROM schedules; DELETE FROM projects; DELETE FROM categories;",
    );

    // Categories
    const insertCat = db.prepare(
      "INSERT INTO categories (name, color, sort_order) VALUES (?, ?, ?)",
    );
    seed.categories.forEach((c, i) => insertCat.run(c.name, c.color, i));

    // Projects
    const insertProj = db.prepare(
      `INSERT INTO projects (priority, number, name, category, evaluation, sort_order)
       VALUES (?, ?, ?, ?, ?, ?)`,
    );
    const projectIds: number[] = [];
    seed.projects.forEach((p, i) => {
      const info = insertProj.run(
        p.priority,
        p.number,
        p.name,
        p.category,
        p.evaluation,
        i,
      );
      projectIds.push(Number(info.lastInsertRowid));
    });

    // Memos
    const insertMemo = db.prepare(
      `INSERT INTO memos (content, color, project_id, sort_order) VALUES (?, ?, ?, ?)`,
    );
    seed.memos.forEach((m, i) => {
      const projId =
        m.project === null ? null : projectIds[m.project - 1] ?? null;
      insertMemo.run(m.content, m.color, projId, i);
    });

    // Schedules
    const insertSched = db.prepare(
      `INSERT INTO schedules (date, time, location, description, notes,
                              remind_before_5min, remind_at_start)
       VALUES (?, ?, ?, ?, ?, 0, 0)`,
    );
    seed.schedules.forEach((s) =>
      insertSched.run(s.date, s.time, s.location, s.description, s.notes),
    );

    db.exec("COMMIT");
  } catch (e) {
    db.exec("ROLLBACK");
    throw e;
  }
}

function summarize(seed: SeedFile, target: string): string {
  const lines: string[] = [];
  lines.push(`locale: ${seed.locale}`);
  lines.push(`target: ${target}`);
  lines.push(`today (anchor): ${seed.today}`);
  lines.push(`categories (${seed.categories.length}):`);
  for (const c of seed.categories) lines.push(`  - ${c.name} (${c.color})`);
  lines.push(`projects (${seed.projects.length}):`);
  for (const p of seed.projects)
    lines.push(`  [${p.priority}] #${p.number} ${p.name}  <${p.category}>`);
  lines.push(`memos (${seed.memos.length}):`);
  for (const m of seed.memos) {
    const proj = m.project === null ? "inbox" : `proj#${m.project}`;
    const trim = m.content.length > 70 ? m.content.slice(0, 67) + "..." : m.content;
    lines.push(`  [${m.color.padEnd(6)}] ${proj.padEnd(7)} ${trim}`);
  }
  lines.push(`schedules (${seed.schedules.length}):`);
  for (const s of seed.schedules)
    lines.push(`  ${s.date} ${s.time ?? "--:--"}  ${s.description}  @${s.location ?? "-"}`);
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

function main() {
  let args: Args;
  try {
    args = parseArgs(process.argv.slice(2));
  } catch (e) {
    console.error(String(e));
    printUsage();
    process.exit(2);
  }

  if (args.help) {
    printUsage();
    process.exit(0);
  }

  if (!args.locale) {
    console.error("ERROR: --locale=en|ko is required.");
    printUsage();
    process.exit(2);
  }

  let target = args.target;
  if (!target) {
    target =
      args.locale === "en"
        ? "~/.hearth-screenshot-seed-en.db"
        : "~/.hearth-screenshot-seed-ko.db";
  }
  target = expandHome(target);
  if (!isAbsolute(target)) target = resolve(target);

  // Hard refusal — never touch the live DB.
  assertNotLiveDb(target);

  const seed = loadSeed(args.locale);

  if (args.dryRun) {
    console.log("=== DRY RUN — nothing will be written ===");
    console.log(summarize(seed, target));
    console.log("=== end dry run ===");
    return;
  }

  // Make sure parent dir exists.
  mkdirSync(dirname(target), { recursive: true });
  // Always start from a clean DB so the screenshots are deterministic.
  if (existsSync(target)) {
    rmSync(target);
  }

  const db = new Database(target, { create: true });
  try {
    db.exec("PRAGMA journal_mode = WAL");
    db.exec("PRAGMA foreign_keys = ON");
    db.exec(SCHEMA_SQL);
    applySeed(db, seed);
  } finally {
    db.close();
  }

  console.log(`Seeded ${args.locale} screenshot DB at: ${target}`);
  console.log(
    `  ${seed.projects.length} projects, ${seed.memos.length} memos, ${seed.schedules.length} schedules.`,
  );
  console.log(
    "Live DB at ~/Library/Application Support/com.codewithgenie.hearth/data.db was NOT touched.",
  );
}

main();
