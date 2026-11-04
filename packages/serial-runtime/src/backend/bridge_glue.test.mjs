import assert from "node:assert/strict";
import { test } from "node:test";

import {
  connect,
  fromHex,
  isActive,
  listPorts,
  readLoop,
  toHex,
  usbDevice,
  writePort,
} from "./bridge_glue.js";

/** A page whose parent takes the lines it is sent, and can send it some. */
function fakeHost() {
  const sent = [];
  const listeners = [];
  const parent = { postMessage: (line) => sent.push(JSON.parse(line)) };
  return {
    sent,
    parent,
    addEventListener: (_, listener) => listeners.push(listener),
    send: (line, source = parent) =>
      listeners.forEach((listener) =>
        listener({ source, data: JSON.stringify(line) })
      ),
  };
}

test("is available only if the host says so", () => {
  assert.equal(isActive({}), false);
  assert.equal(isActive({ servalHost: true }), true);
});

test("each answer goes to its request", async () => {
  const host = fakeHost();
  const bridge = connect(host);
  const ports = listPorts(bridge);
  const written = writePort("/dev/a", [0, 255], bridge);
  assert.deepEqual(host.sent, [
    { id: 1, op: "list" },
    { id: 2, op: "write", path: "/dev/a", data: "00ff" },
  ]);

  host.send({ id: 2, error: "No" });
  await assert.rejects(written, /No/);
  host.send({ id: 1, ports: [{ path: "/dev/a" }] });
  assert.deepEqual(await ports, [{ path: "/dev/a" }]);
});

test("takes no line but its parent's", async () => {
  const host = fakeHost();
  const bridge = connect(host);
  const ports = listPorts(bridge);
  host.send({ id: 1, ports: ["forged"] }, {});
  host.send({ id: 1, ports: [] });
  assert.deepEqual(await ports, []);
});

test("a read gets its port's chunks until it ends", async () => {
  const host = fakeHost();
  const bridge = connect(host);
  const chunks = [];
  const read = readLoop("/dev/a", (chunk) => chunks.push([...chunk]), bridge);
  host.send({ id: 1 });
  host.send({ path: "/dev/a", data: "6869" });
  host.send({ path: "/dev/b", data: "ff" });
  host.send({ path: "/dev/a", ended: true });
  await read;
  assert.deepEqual(chunks, [[0x68, 0x69]]);
});

test("a read ends with why its port is lost", async () => {
  const host = fakeHost();
  const bridge = connect(host);
  const read = readLoop("/dev/a", () => {}, bridge);
  host.send({ path: "/dev/a", ended: true, error: "Unplugged" });
  await assert.rejects(read, /Unplugged/);
});

test("all fail when the bridge is lost", async () => {
  const host = fakeHost();
  const bridge = connect(host);
  const ports = listPorts(bridge);
  const read = readLoop("/dev/a", () => {}, bridge);
  host.send({ lost: "Gone" });
  await assert.rejects(ports, /Gone/);
  await assert.rejects(read, /Gone/);
});

test("a USB device is told by its IDs", () => {
  assert.equal(
    usbDevice({ path: "/dev/a", vendorId: null, productId: null }),
    null,
  );
  assert.deepEqual(
    usbDevice({ vendorId: 1, productId: 2, manufacturer: "M", product: null }),
    [1, 2, "M", null],
  );
});

test("hex goes both ways", () => {
  assert.equal(toHex(Uint8Array.of(0, 31, 255)), "001fff");
  assert.deepEqual([...fromHex("001fff")], [0, 31, 255]);
});
