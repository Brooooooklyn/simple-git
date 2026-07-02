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
  const value = repo.config().snapshot().getString("core.bare");
  t.is(typeof value, "string");
  const expected = execSync("git config core.bare", { cwd: workDir })
    .toString()
    .trim();
  t.is(value, expected);
});

// Read-only: get_boolean decodes a boolean config key.
test("config getBoolean reads a boolean value", (t) => {
  const repo = new Repository(workDir);
  const bare = repo.config().getBoolean("core.bare");
  t.is(typeof bare, "boolean");
  t.false(bare);
});

// Mutating (temp repo): setString then read it back via getString.
test("config setString round-trips through getString", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setString("user.name", "Set By Api");
    t.is(config.getString("user.name"), "Set By Api");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): the typed setters/getters round-trip.
test("config typed setters round-trip (bool/number/bigint)", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setBoolean("custom.flag", true);
    t.true(config.getBoolean("custom.flag"));
    config.setNumber("custom.intval", 42);
    t.is(config.getNumber("custom.intval"), 42);
    config.setBigInt("custom.bigval", 1234567890n);
    t.is(config.getBigInt("custom.bigval"), 1234567890n);
    // core.bare is written by `git init` as a real boolean.
    t.false(config.getBoolean("core.bare"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): a value above 2^31 but below 2^53 round-trips through
// setNumber/getNumber exactly. Under the old i32-backed path setNumber(3e9)
// silently ToInt32-wrapped to -1294967296; now it survives losslessly.
test("config setNumber/getNumber round-trips a value beyond i32 range", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    const big = 3000000000; // > 2^31 - 1, < Number.MAX_SAFE_INTEGER
    t.true(big > 2 ** 31 - 1);
    config.setNumber("x.big", big);
    t.is(config.getNumber("x.big"), big);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): small integers (including negatives and zero) still
// round-trip unchanged after the i32 -> f64/i64 switch.
test("config setNumber/getNumber round-trips small integers", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    for (const n of [42, -7, 0]) {
      config.setNumber("x.small", n);
      t.is(config.getNumber("x.small"), n);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): setNumber rejects a non-integer instead of truncating.
test("config setNumber rejects a non-integer value", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    t.throws(() => config.setNumber("x.f", 1.5), { code: "InvalidArg" });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): setNumber rejects a magnitude beyond the JS
// safe-integer range instead of silently losing precision.
test("config setNumber rejects an out-of-safe-range value", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    t.throws(() => config.setNumber("x.huge", 1e18), { code: "InvalidArg" });
    t.throws(() => config.setNumber("x.huge", Number.MAX_SAFE_INTEGER + 2), {
      code: "InvalidArg",
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): a value stored (via setBigInt) beyond the safe-integer
// range makes getNumber throw InvalidArg rather than returning a lossy number.
test("config getNumber throws InvalidArg on a value beyond safe range", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setBigInt("x.i64", 9007199254740993n); // Number.MAX_SAFE_INTEGER + 2
    t.throws(() => config.getNumber("x.i64"), { code: "InvalidArg" });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): the exact Number.MAX_SAFE_INTEGER boundary is accepted
// by setNumber and read back exactly by getNumber.
test("config setNumber/getNumber round-trips Number.MAX_SAFE_INTEGER", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setNumber("x.max", Number.MAX_SAFE_INTEGER);
    t.is(config.getNumber("x.max"), Number.MAX_SAFE_INTEGER);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): a bigint above Number.MAX_SAFE_INTEGER survives a
// setBigInt/getBigInt round-trip exactly. Under the old `number`-based getI64
// this value would have been silently truncated.
test("config setBigInt/getBigInt is lossless above Number.MAX_SAFE_INTEGER", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    const huge = 9007199254740993n; // Number.MAX_SAFE_INTEGER + 2
    t.true(huge > BigInt(Number.MAX_SAFE_INTEGER));
    config.setBigInt("custom.huge", huge);
    const read = config.getBigInt("custom.huge");
    t.is(typeof read, "bigint");
    t.is(read, huge);
    // Proof of why bigint matters: Number() truncation would not be exact.
    t.not(BigInt(Number(huge)), huge);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): negative bigint round-trips with the sign preserved.
test("config setBigInt/getBigInt preserves negative values", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setBigInt("custom.neg", -9007199254740993n);
    t.is(config.getBigInt("custom.neg"), -9007199254740993n);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): remove_entry deletes a key.
test("config removeEntry deletes a key", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setString("custom.key", "value");
    t.is(config.getString("custom.key"), "value");
    config.removeEntry("custom.key");
    t.throws(() => config.getString("custom.key"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Mutating (temp repo): entries() filtered by glob includes the keys we set.
test("config entries can be filtered by glob", (t) => {
  const dir = makeTempRepo();
  try {
    const config = new Repository(dir).config();
    config.setString("user.name", "Glob Tester");
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
    t.true(sig.when() instanceof Date);
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
