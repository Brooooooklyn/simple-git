import { join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

import { Repository } from "../index.js";

const __dirname = join(fileURLToPath(import.meta.url), "..");
const workDir = join(__dirname, "..");

// Blame is read-only, so we can run it against the project's own repo (like
// modification.spec.mjs). `Cargo.toml` has existed for the whole history, so it
// always produces at least one hunk.
test.beforeEach((t) => {
  t.context.repo = new Repository(workDir);
});

function assertHunkShape(t, hunk) {
  t.is(typeof hunk.linesInHunk, "number");
  t.true(hunk.linesInHunk > 0, "linesInHunk is positive");
  t.regex(hunk.finalCommitId, /^[0-9a-f]{40}$/, "finalCommitId is 40-char hex");
  t.is(typeof hunk.finalStartLine, "number");
  t.true(hunk.finalStartLine >= 1, "finalStartLine is 1-based");
  t.is(typeof hunk.finalTime, "number");
  t.regex(hunk.origCommitId, /^[0-9a-f]{40}$/, "origCommitId is 40-char hex");
  t.is(typeof hunk.origStartLine, "number");
  t.is(typeof hunk.isBoundary, "boolean");
}

test("blameFile returns a non-empty array of well-formed hunks", (t) => {
  const { repo } = t.context;
  const hunks = repo.blameFile("Cargo.toml");
  t.true(Array.isArray(hunks));
  t.true(hunks.length > 0, "at least one hunk");
  for (const hunk of hunks) {
    assertHunkShape(t, hunk);
  }
});

test("blameFile hunks are contiguous and cover at least one line", (t) => {
  const { repo } = t.context;
  const hunks = repo.blameFile("Cargo.toml");
  const total = hunks.reduce((sum, h) => sum + h.linesInHunk, 0);
  t.true(total >= 1, "sum of linesInHunk is >= 1");

  // Hunks come out ordered by final start line; each should start exactly where
  // the previous one ended (1-based, contiguous coverage of the file).
  let expectedNext = hunks[0].finalStartLine;
  for (const hunk of hunks) {
    t.is(hunk.finalStartLine, expectedNext, "hunk starts where the prior ended");
    expectedNext += hunk.linesInHunk;
  }
});

test("blameLine returns the hunk covering the requested line", (t) => {
  const { repo } = t.context;
  const hunk = repo.blameLine("Cargo.toml", 1);
  t.truthy(hunk);
  assertHunkShape(t, hunk);
  // Line 1 must fall inside [finalStartLine, finalStartLine + linesInHunk).
  t.true(hunk.finalStartLine <= 1, "hunk begins at or before line 1");
  t.true(1 < hunk.finalStartLine + hunk.linesInHunk, "hunk extends past line 1");
});

test("blameFileAsync matches the sync call length", async (t) => {
  const { repo } = t.context;
  const sync = repo.blameFile("Cargo.toml");
  const async = await repo.blameFileAsync("Cargo.toml");
  t.true(Array.isArray(async));
  t.is(async.length, sync.length);
  // First hunk should be byte-identical between the two paths.
  t.is(async[0].finalCommitId, sync[0].finalCommitId);
  t.is(async[0].finalStartLine, sync[0].finalStartLine);
});
