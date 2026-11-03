import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import files from "./files.js";

const { createFiles } = files;

/** VS Code's API, its dialogs picking `picks` in turn; what they were asked. */
function fakeVscode(picks) {
  const asked = [];
  const pick = (options) => {
    asked.push(options);
    return picks.shift();
  };
  const uri = (fsPath) => ({ fsPath, path: fsPath });
  return {
    asked,
    vscode: {
      window: {
        showSaveDialog: async (options) => pick(options),
        showOpenDialog: async (options) => {
          const picked = pick(options);
          return picked && [picked];
        },
      },
      workspace: {
        workspaceFolders: [{ uri: uri("/work") }],
        fs: {
          writeFile: async (to, bytes) => writeFileSync(to.fsPath, bytes),
          readFile: async (from) => readFileSync(from.fsPath),
        },
      },
      Uri: {
        file: uri,
        joinPath: (base, name) => uri(join(base.fsPath, name)),
      },
    },
  };
}

const folder = () => mkdtempSync(join(tmpdir(), "serval-files-"));

test("saves where the user picks, from the workspace's folder", async () => {
  const to = join(folder(), "layout.json");
  const { vscode, asked } = fakeVscode([{ fsPath: to, path: to }]);
  const answer = await createFiles(vscode).answer({
    id: 1,
    op: "save",
    name: "layout.json",
    text: "{}",
  });
  assert.deepEqual(answer, { id: 1 });
  assert.equal(readFileSync(to, "utf8"), "{}");
  assert.equal(asked[0].defaultUri.fsPath, "/work/layout.json");
  assert.deepEqual(asked[0].filters, { JSON: ["json"] });
});

test("opens a file of the extensions asked for, or none if cancelled", async () => {
  const from = join(folder(), "a.rn");
  writeFileSync(from, "pub fn main() {}");
  const { vscode, asked } = fakeVscode([
    { fsPath: from, path: from },
    undefined,
  ]);
  const opened = createFiles(vscode);
  const answer = await opened.answer({ id: 1, op: "open", extensions: ["rn"] });
  assert.equal(answer.file.name, "a.rn");
  assert.equal(
    Buffer.from(answer.file.data, "base64").toString(),
    "pub fn main() {}",
  );
  assert.deepEqual(asked[0].filters, { RN: ["rn"] });
  assert.deepEqual(await opened.answer({ id: 2, op: "open", extensions: [] }), {
    id: 2,
    file: null,
  });
  assert.equal(asked[1].filters, undefined);
});

test("records to a new file, named of the time, written as it comes", async () => {
  const dir = folder();
  const to = join(dir, "picked.log");
  const { vscode, asked } = fakeVscode([{ fsPath: to, path: to }]);
  const recording = createFiles(vscode, {
    now: () => new Date(2026, 10, 3, 9, 5, 7).getTime(),
  });
  const { record } = await recording.answer({
    id: 1,
    op: "record",
    extension: "log",
    append: false,
  });
  assert.equal(asked[0].defaultUri.fsPath, "/work/serval-20261103-090507.log");
  assert.deepEqual(record, { id: 1, name: "picked.log", tail: "" });

  await recording.answer({
    id: 2,
    op: "write",
    record: 1,
    data: Buffer.from("hi ").toString("base64"),
  });
  await recording.answer({
    id: 3,
    op: "write",
    record: 1,
    data: Buffer.from("there").toString("base64"),
  });
  assert.deepEqual(await recording.answer({ id: 4, op: "close", record: 1 }), {
    id: 4,
  });
  assert.equal(readFileSync(to, "utf8"), "hi there");

  const closed = await recording.answer({
    id: 5,
    op: "write",
    record: 1,
    data: "",
  });
  assert.equal(closed.error, "The record file is not open");
});

test("adds to an existing record, given its tail", async () => {
  const to = join(folder(), "old.log");
  writeFileSync(to, "first line\nsecond line\n");
  const { vscode } = fakeVscode([{ fsPath: to, path: to }]);
  const recording = createFiles(vscode);
  const { record } = await recording.answer({
    id: 1,
    op: "record",
    extension: "log",
    append: true,
    tailBytes: 12,
  });
  assert.equal(Buffer.from(record.tail, "base64").toString(), "second line\n");
  await recording.answer({
    id: 2,
    op: "write",
    record: record.id,
    data: Buffer.from("third\n").toString("base64"),
  });
  await recording.closeAll();
  assert.equal(readFileSync(to, "utf8"), "first line\nsecond line\nthird\n");
});

test("an unknown request is answered with why", async () => {
  const { vscode } = fakeVscode([]);
  assert.deepEqual(await createFiles(vscode).answer({ id: 9, op: "delete" }), {
    id: 9,
    error: "Unknown request: delete",
  });
});
