import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import bridge from "./bridge.js";

const { startBridge } = bridge;

/** Its lines and why it was lost, once `until` of the lines holds. */
function run(program, until) {
  const lines = [];
  const losses = [];
  let done;
  const finished = new Promise((resolve) => (done = resolve));
  const started = startBridge(
    program,
    (line) => {
      lines.push(JSON.parse(line));
      if (until(lines)) done();
    },
    (why) => losses.push(why),
  );
  return { started, lines, losses, finished };
}

/** A program of the shell script `body`. */
function script(body) {
  const program = join(mkdtempSync(join(tmpdir(), "serval-bridge-")), "bridge");
  writeFileSync(program, `#!/bin/sh\n${body}\n`);
  chmodSync(program, 0o755);
  return program;
}

test("a program that cannot start is told once, and each request after is answered lost", async () => {
  const { started, lines, losses, finished } = run(
    "/no/such/serial-bridge",
    (lines) => lines.length === 3,
  );
  await new Promise((resolve) => setTimeout(resolve, 100));
  started.send(JSON.stringify({ id: 1, op: "list" }));
  started.send(JSON.stringify({ id: 2, op: "list" }));
  await finished;
  assert.equal(losses.length, 1);
  assert.match(losses[0], /^The serial bridge stopped: .*ENOENT/);
  assert.deepEqual(lines, [{ lost: losses[0] }, { lost: losses[0] }, {
    lost: losses[0],
  }]);
});

test("a program that ends is told with its exit code", {
  skip: process.platform === "win32",
}, async () => {
  const { losses, finished } = run(
    script("exit 3"),
    (lines) => lines.length === 1,
  );
  await finished;
  assert.deepEqual(losses, ["The serial bridge stopped: exit code 3"]);
});

test("its lines are passed on, and its end as the panel closes is not told", {
  skip: process.platform === "win32",
}, async () => {
  // Answers each request with itself, till its input ends
  const echo = script("while read -r line; do echo \"$line\"; done");
  const { started, lines, losses, finished } = run(
    echo,
    (lines) => lines.length === 1,
  );
  started.send(JSON.stringify({ id: 1, op: "list" }));
  await finished;
  assert.deepEqual(lines, [{ id: 1, op: "list" }]);
  started.end();
  await new Promise((resolve) => setTimeout(resolve, 100));
  assert.deepEqual(losses, []);
});
