import assert from "node:assert/strict";
import { test } from "node:test";

import {
  closePort,
  getPorts,
  isSupported,
  openPort,
  readLoop,
  requestPort,
  usbProductId,
  usbVendorId,
  writePort,
} from "./serial.js";

function fakePort({ chunks = [], error, info } = {}) {
  const port = { released: { reader: false, writer: false }, written: [] };
  port.readable = {
    getReader: () => ({
      read: async () => {
        if (chunks.length) {
          return { value: chunks.shift(), done: false };
        }
        if (error) {
          throw error;
        }
        return { value: undefined, done: true };
      },
      releaseLock: () => {
        port.released.reader = true;
      },
    }),
  };
  port.writable = {
    getWriter: () => ({
      write: async (bytes) => {
        if (error) {
          throw error;
        }
        port.written.push(bytes);
      },
      releaseLock: () => {
        port.released.writer = true;
      },
    }),
  };
  if (info) {
    port.getInfo = () => info;
  }
  return port;
}

test("readLoop passes every chunk on, then releases the reader", async () => {
  const port = fakePort({ chunks: [Uint8Array.of(1, 2), Uint8Array.of(3)] });
  const received = [];
  await readLoop(port, (chunk) => received.push([...chunk]));
  assert.deepEqual(received, [[1, 2], [3]]);
  assert.equal(port.released.reader, true);
});

test("readLoop releases the reader when reading fails", async () => {
  const port = fakePort({
    chunks: [Uint8Array.of(1)],
    error: new Error("device lost"),
  });
  await assert.rejects(
    readLoop(port, () => {}),
    /device lost/,
  );
  assert.equal(port.released.reader, true);
});

test("writePort writes the bytes, then releases the writer, even on failure", async () => {
  const port = fakePort();
  await writePort(port, Uint8Array.of(104, 105));
  assert.deepEqual(
    port.written.map((bytes) => [...bytes]),
    [[104, 105]],
  );
  assert.equal(port.released.writer, true);

  const failing = fakePort({ error: new Error("disconnected") });
  await assert.rejects(writePort(failing, Uint8Array.of(1)), /disconnected/);
  assert.equal(failing.released.writer, true);
});

test("writePort writes one after another, in order", async () => {
  const port = fakePort();
  let locked = false;
  const getWriter = port.writable.getWriter;
  port.writable.getWriter = () => {
    assert.equal(locked, false, "the stream is locked");
    locked = true;
    const writer = getWriter();
    return {
      write: async (bytes) => {
        await new Promise((resolve) => setTimeout(resolve, 5));
        await writer.write(bytes);
      },
      releaseLock: () => {
        locked = false;
        writer.releaseLock();
      },
    };
  };
  await Promise.all(
    [1, 2, 3].map((byte) => writePort(port, Uint8Array.of(byte))),
  );
  assert.deepEqual(
    port.written.map((bytes) => [...bytes]),
    [[1], [2], [3]],
  );
});

test("USB ids are read from the port info, if valid", () => {
  const usb = fakePort({ info: { usbVendorId: 0x2341, usbProductId: 0x0043 } });
  assert.equal(usbVendorId(usb), 0x2341);
  assert.equal(usbProductId(usb), 0x0043);

  // Bluetooth ports have no USB ids; very old browsers have no getInfo
  assert.equal(usbVendorId(fakePort({ info: {} })), undefined);
  assert.equal(usbVendorId(fakePort()), undefined);
  assert.equal(
    usbVendorId(fakePort({ info: { usbVendorId: 0x10000 } })),
    undefined,
  );
  assert.equal(
    usbVendorId(fakePort({ info: { usbVendorId: 1.5 } })),
    undefined,
  );
});

test("port selection and opening go through the given serial API", async () => {
  const port = {
    opened: null,
    open: async (options) => (port.opened = options),
  };
  const serial = {
    getPorts: async () => [port],
    requestPort: async () => port,
  };

  assert.deepEqual(await getPorts(serial), [port]);
  assert.equal(await requestPort(serial), port);
  await openPort(port, 115200, 8, 1, "none", "none");
  assert.deepEqual(port.opened, {
    baudRate: 115200,
    dataBits: 8,
    stopBits: 1,
    parity: "none",
    flowControl: "none",
  });
});

/** A port that, like a real one, fails to close while its stream is locked. */
function streamingPort() {
  const port = { locked: false, closed: false };
  let endRead;
  port.readable = {
    getReader: () => {
      port.locked = true;
      return {
        read: () => new Promise((resolve) => (endRead = resolve)),
        cancel: async () => endRead({ value: undefined, done: true }),
        releaseLock: () => (port.locked = false),
      };
    },
  };
  port.close = async () => {
    if (port.locked) {
      throw new TypeError("The port's stream is locked");
    }
    port.closed = true;
  };
  return port;
}

test("closePort stops readLoop, then closes the port", async () => {
  const port = streamingPort();
  const reading = readLoop(port, () => {});

  await closePort(port);
  await reading;
  assert.equal(port.locked, false);
  assert.equal(port.closed, true);
});

test("closePort closes a port that is not being read", async () => {
  const port = streamingPort();
  await closePort(port);
  assert.equal(port.closed, true);
});

test("isSupported is whether the browser has Web Serial", () => {
  assert.equal(isSupported({ serial: {} }), true);
  assert.equal(isSupported({}), false);
});
