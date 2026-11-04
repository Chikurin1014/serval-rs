import assert from "node:assert/strict";
import { test } from "node:test";

import {
  connect,
  fromBase64,
  openFile,
  pickRecordFile,
  saveFile,
  toBase64,
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

test("a record picked is a sink of its writes and close", async () => {
  const { sent, files, answer } = fakeHost();
  const picking = pickRecordFile("serval.log", true, 64, files);
  assert.deepEqual(sent[0], {
    id: 1,
    op: "record",
    name: "serval.log",
    append: true,
    tailBytes: 64,
  });
  answer({ id: 1, record: { id: 7, name: "a.log", tail: "dGFpbA==" } });
  const sink = await picking;
  assert.equal(sink.name, "a.log");
  assert.deepEqual(sink.tail, new TextEncoder().encode("tail"));

  const written = sink.write(Uint8Array.from([104, 105]));
  assert.deepEqual(sent[1], { id: 2, op: "write", record: 7, data: "aGk=" });
  answer({ id: 2, error: "Disk full" });
  await assert.rejects(written, /Disk full/);

  const closed = sink.close();
  assert.deepEqual(sent[2], { id: 3, op: "close", record: 7 });
  answer({ id: 3 });
  await closed;
});
