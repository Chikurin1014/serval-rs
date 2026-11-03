import assert from "node:assert/strict";
import { test } from "node:test";

import {
  closeRecordFile,
  connect,
  fromBase64,
  openFile,
  pickRecordFile,
  recordFileFailure,
  saveFile,
  toBase64,
  writeRecordFile,
} from "./vscode_files.js";

/** A frame whose parent (the panel's page) takes what it asks, and answers. */
function fakeHost() {
  const sent = [];
  const listeners = [];
  const parent = { postMessage: (message) => sent.push(message.servalFile) };
  const host = {
    parent,
    addEventListener: (_, listener) => listeners.push(listener),
  };
  return {
    sent,
    files: connect(host),
    answer: (servalFile, source = parent) =>
      listeners.forEach((listener) =>
        listener({ source, data: { servalFile } })
      ),
  };
}

const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

test("bytes go as base64 both ways", () => {
  const bytes = Uint8Array.from([0, 1, 254, 255]);
  assert.equal(toBase64(bytes), "AAH+/w==");
  assert.deepEqual(fromBase64("AAH+/w=="), bytes);
  assert.deepEqual(fromBase64(undefined), new Uint8Array());
});

test("each answer goes to its request, and an error fails it", async () => {
  const { sent, files, answer } = fakeHost();
  const saved = saveFile("layout.json", "{}", files);
  const opened = openFile(["json"], files);
  assert.deepEqual(sent, [
    { id: 1, op: "save", name: "layout.json", text: "{}" },
    { id: 2, op: "open", extensions: ["json"] },
  ]);

  // Of another page: passed over
  answer({ id: 2, file: null }, {});
  answer({ id: 2, file: { name: "layout.json", data: "e30=" } });
  assert.deepEqual(await opened, {
    name: "layout.json",
    bytes: new TextEncoder().encode("{}"),
  });
  answer({ id: 1, error: "Permission denied" });
  await assert.rejects(saved, /Permission denied/);
});

test("a file not picked is none", async () => {
  const { files, answer } = fakeHost();
  const opened = openFile([], files);
  answer({ id: 1, file: null });
  assert.equal(await opened, null);
  const record = pickRecordFile("log", false, 64, files);
  answer({ id: 2, record: null });
  assert.equal(await record, null);
});

test("a record's writes go in order, and the first failure stops those after", async () => {
  const { sent, files, answer } = fakeHost();
  const picking = pickRecordFile("log", true, 64, files);
  answer({ id: 1, record: { id: 7, name: "a.log", tail: "dGFpbA==" } });
  const record = await picking;
  assert.equal(record.name, "a.log");
  assert.deepEqual(record.tail, new TextEncoder().encode("tail"));

  writeRecordFile(record, Uint8Array.from([104, 105]));
  writeRecordFile(record, Uint8Array.from([33]));
  await settled();
  // The second waits for the first to be written
  assert.deepEqual(sent.slice(1), [{
    id: 2,
    op: "write",
    record: 7,
    data: "aGk=",
  }]);
  answer({ id: 2 });
  await settled();
  assert.deepEqual(sent.at(-1), {
    id: 3,
    op: "write",
    record: 7,
    data: "IQ==",
  });
  answer({ id: 3, error: "Disk full" });
  assert.equal(await recordFileFailure(record), "Disk full");

  writeRecordFile(record, Uint8Array.from([1]));
  const closing = closeRecordFile(record);
  await settled();
  // The write after the failure is not asked for
  assert.deepEqual(sent.at(-1), { id: 4, op: "close", record: 7 });
  answer({ id: 4 });
  await assert.rejects(closing, /Disk full/);
});
