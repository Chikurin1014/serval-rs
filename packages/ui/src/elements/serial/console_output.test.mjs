import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript } from "../../../js-test/load-script.mjs";

const { appendOutput, cutPoint } = loadScript(
  new URL("./console_output.js", import.meta.url),
);

/** Like a DOM `Text`, for what `appendOutput` uses of it. */
function fakeText(data = "") {
  return {
    data,
    get length() {
      return this.data.length;
    },
    appendData(text) {
      this.data += text;
    },
    deleteData(offset, count) {
      this.deletes = (this.deletes ?? 0) + 1;
      this.data = this.data.slice(0, offset) + this.data.slice(offset + count);
    },
  };
}

test("appendOutput appends within the limit", () => {
  const node = fakeText("one\n");
  appendOutput(node, "two\n", 100);
  assert.equal(node.data, "one\ntwo\n");
});

test("appendOutput drops whole lines from the front past the limit", () => {
  const node = fakeText("one\ntwo\n");
  appendOutput(node, "three\n", 9);
  assert.equal(node.data, "three\n");
});

test("appendOutput drops a tenth at once, not a little on every chunk", () => {
  const node = fakeText();
  for (let i = 0; i < 100; i++) {
    appendOutput(node, "123456789\n", 1000);
    assert.ok(node.length <= 1000);
  }
  // 1000 characters in all: the limit is reached, not passed
  assert.equal(node.deletes, undefined);

  for (let i = 0; i < 100; i++) {
    appendOutput(node, "123456789\n", 1000);
  }
  // Each drop leaves 900, room for 10 more lines before the next
  assert.equal(node.deletes, 10);
  assert.match(node.data, /^(123456789\n)+$/);
});

test("cutPoint keeps a line that already starts at the cut", () => {
  assert.equal(cutPoint("one\ntwo\n", 4), 4);
});

test("cutPoint without line breaks cuts right after the excess", () => {
  assert.equal(cutPoint("abcdef", 2), 2);
  // Not between the halves of a surrogate pair ("😀" is two units)
  assert.equal(cutPoint("a😀b", 2), 3);
});
