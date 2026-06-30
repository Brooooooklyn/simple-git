import { execSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

import { Config, ConfigLevel, Repository } from "../index.js";

const __dirname = join(fileURLToPath(import.meta.url), "..");
const workDir = join(__dirname, "..");

// Spin up an isolated repo under os.tmpdir() so mutating cases never touch the
// project's own config. `identity` controls whether a local user.name/email is
// written. Returns the dir; caller cleans up.
function makeTempRepo({ identity = true } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-config-"));
  const run = (args) => execSync(`git ${args}`, { cwd: dir });
  run("init -q");
  if (identity) {
    run("config user.name tester");
    run("config user.email tester@example.com");
  }
  return dir;
}

// Read-only against the project repo: a snapshot can read an always-present key.
test("config snapshot reads a string value matching the git CLI", (t) => {
  const repo = new Repository(workDir);
  const value = repo.config().snapshot().getStringValue("core.bare");
  t.is(typeof value, "string");
  const expected = execSync("git config core.bare", { cwd: workDir })
    .toString()
    .trim();
  t.is(value, expected);
});

// Read-only: get_bool decodes a boolean config key.
test("config getBool reads a boolean value", (t) => {
  const repo = new Repository(workDir);
  const bare = repo.config().getBool("core.bare");
  t.is(typeof bare, "boolean");
  t.false(bare);
});

// Mutating (temp repo): set_str then read it back.
test("config setStr round-trips through getStringValue", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setStr("user.name", "Set By Api");
    t.is(config.getStringValue("user.name"), "Set By Api");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): the typed setters/getters round-trip.
test("config typed setters round-trip (bool/i32/i64)", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setBool("custom.flag", true);
    t.true(config.getBool("custom.flag"));
    config.setI32("custom.intval", 42);
    t.is(config.getI32("custom.intval"), 42);
    config.setI64("custom.bigval", 1234567890);
    t.is(config.getI64("custom.bigval"), 1234567890);
    // core.bare is written by `git init` as a real boolean.
    t.false(config.getBool("core.bare"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): remove_entry deletes a key.
test("config removeEntry deletes a key", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setStr("custom.key", "value");
    t.is(config.getStringValue("custom.key"), "value");
    config.removeEntry("custom.key");
    t.throws(() => config.getStringValue("custom.key"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): entries() filtered by glob includes the keys we set.
test("config entries can be filtered by glob", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setStr("user.name", "Glob Tester");
    const entries = config.entries("user.*");
    t.true(Array.isArray(entries));
    const names = entries.map((e) => e.name);
    t.true(names.includes("user.name"));
    t.true(names.includes("user.email"));
    // The local override we just wrote is present at Local level.
    const localName = entries.find(
      (e) => e.name === "user.name" && e.level === ConfigLevel.Local,
    );
    t.truthy(localName);
    t.is(localName.value, "Glob Tester");
    for (const entry of entries) {
      t.is(typeof entry.name, "string");
      t.is(typeof entry.value, "string");
      t.is(typeof entry.level, "number");
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Factory: open_default opens the global/XDG/system config and is iterable.
test("Config.openDefault returns a usable config", (t) => {
  const config = Config.openDefault();
  t.truthy(config);
  t.true(Array.isArray(config.entries()));
});

// Mutating (temp repo): signature() reads the identity from config.
test("signature reads identity from config", (t) => {
  const dir = makeTempRepo();
  try {
    const sig = new Repository(dir).signature();
    t.is(sig.name(), "tester");
    t.is(sig.email(), "tester@example.com");
    t.is(typeof sig.when(), "number");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// signature() throws iff no identity is resolvable. libgit2 falls back to
// global/system config, so we probe the effective identity the same way (a
// repo with no local identity), and assert the matching behaviour. This is
// deterministic in both CI (no global identity -> throws) and dev machines
// (global identity present -> succeeds via fallback).
test("signature behaviour matches the effective config identity", (t) => {
  const dir = makeTempRepo({ identity: false });
  try {
    let hasFallbackIdentity = true;
    try {
      // `git config <key>` with cwd inside the (identity-less) repo resolves
      // local -> global -> system, exactly like libgit2's signature() lookup.
      execSync("git config user.name", { cwd: dir, stdio: "pipe" });
      execSync("git config user.email", { cwd: dir, stdio: "pipe" });
    } catch {
      hasFallbackIdentity = false;
    }
    const repo = new Repository(dir);
    if (hasFallbackIdentity) {
      const sig = repo.signature();
      t.truthy(sig.name());
      t.truthy(sig.email());
    } else {
      t.throws(() => repo.signature());
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
