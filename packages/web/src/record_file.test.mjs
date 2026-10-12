import assert from "node:assert/strict";
import { test } from "node:test";

import {
  closeRecordFile,
  isSupported,
  pickRecordFile,
  recordFileFailure,
  recordFileName,
  recordFileTail,
  writeRecordFile,
} from "./record_file.js";

/** A file whose contents change only as its writables close. */
function fakeFile(initial = "", { failWrite = false } = {}) {
  const file = { contents: initial, closes: 0, opened: [] };
  file.handle = {
    name: "old.log",
    getFile: async () => ({
      size: file.contents.length,
      slice: (start) => ({
        arrayBuffer: async () =>
          new TextEncoder().encode(file.contents.slice(start)).buffer,
      }),
    }),
    createWritable: async ({ keepExistingData }) => {
      file.opened.push(keepExistingData);
      let pending = keepExistingData ? file.contents : "";
      let at = 0;
      return {
        seek: async (position) => {
          at = position;
        },
        write: async (bytes) => {
          await new Promise((resolve) => setTimeout(resolve, 1));
          if (failWrite) {
            throw new Error("disk full");
          }
          const text = new TextDecoder().decode(bytes);
          pending = pending.slice(0, at) + text;
          at += text.length;
        },
        close: async () => {
          file.contents = pending;
          file.closes++;
        },
      };
    },
  };
  return file;
}

function fakeWindow(file, { cancel = false } = {}) {
  const picked = {};
  const pick = async (options) => {
    picked.options = options;
    if (cancel) {
      throw new DOMException("The user aborted a request.", "AbortError");
    }
    return file.handle;
  };
  return {
    picked,
    showSaveFilePicker: pick,
    showOpenFilePicker: async (options) => [await pick(options)],
  };
}

const bytes = (text) => new TextEncoder().encode(text);

test("a new record is named after the local time, and written in order", async () => {
  const file = fakeFile();
  const win = fakeWindow(file);
  const now = () => new Date(2026, 9, 8, 9, 5, 7).getTime();
  const record = await pickRecordFile("log", false, 0, { win, now });
  assert.equal(win.picked.options.suggestedName, "serval-20261008-090507.log");

  for (const line of ["a\n", "b\n", "c\n"]) {
    writeRecordFile(record, bytes(line));
  }
  await closeRecordFile(record);
  assert.equal(file.contents, "a\nb\nc\n");
});

test("an appended record adds to what the file has, and tells its end", async () => {
  const file = fakeFile("first\nold\n");
  const record = await pickRecordFile("log", true, 4, { win: fakeWindow(file) });
  assert.equal(recordFileName(record), "old.log");
  assert.equal(new TextDecoder().decode(recordFileTail(record)), "old\n");
  writeRecordFile(record, bytes("new\n"));
  await closeRecordFile(record);
  assert.equal(file.contents, "first\nold\nnew\n");
});

test("the file gets what was written so far as often as asked", async () => {
  const file = fakeFile();
  let time = 0;
  const record = await pickRecordFile("log", false, 0, {
    win: fakeWindow(file),
    now: () => time,
    commitMs: 1000,
  });
  writeRecordFile(record, bytes("a\n"));
  await record.chain;
  assert.equal(file.contents, "");

  time = 1000;
  writeRecordFile(record, bytes("b\n"));
  await record.chain;
  assert.equal(file.contents, "a\nb\n");
  // Opened again to add to it
  assert.deepEqual(file.opened, [false, true]);

  writeRecordFile(record, bytes("c\n"));
  await closeRecordFile(record);
  assert.equal(file.contents, "a\nb\nc\n");
});

test("a failed write is told at once", async () => {
  const file = fakeFile("", { failWrite: true });
  const record = await pickRecordFile("log", false, 0, { win: fakeWindow(file) });
  writeRecordFile(record, bytes("a\n"));
  assert.equal(await recordFileFailure(record), "disk full");
});

test("a record closed without failing tells no failure", async () => {
  const file = fakeFile();
  const record = await pickRecordFile("log", false, 0, { win: fakeWindow(file) });
  writeRecordFile(record, bytes("a\n"));
  await closeRecordFile(record);
  assert.equal(await recordFileFailure(record), null);
});

test("a failed write fails the close, and stops the writes after it", async () => {
  const file = fakeFile("", { failWrite: true });
  const record = await pickRecordFile("log", false, 0, { win: fakeWindow(file) });
  writeRecordFile(record, bytes("a\n"));
  writeRecordFile(record, bytes("b\n"));
  await assert.rejects(closeRecordFile(record), /disk full/);
});

test("cancelling the picker gives no record", async () => {
  const file = fakeFile();
  assert.equal(
    await pickRecordFile("log", false, 0, {
      win: fakeWindow(file, { cancel: true }),
    }),
    null,
  );
});

test("isSupported is whether the browser can save files", () => {
  assert.equal(isSupported({ showSaveFilePicker: () => {} }), true);
  assert.equal(isSupported({}), false);
});
