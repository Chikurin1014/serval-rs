import assert from "node:assert/strict";
import { test } from "node:test";

import {
    getPorts,
    openPort,
    readLoop,
    requestPort,
    usbProductId,
    usbVendorId,
    writePort,
} from "./serial.js";

/** A port whose reader yields `chunks`, then ends (or throws `error`). */
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
    const port = fakePort({ chunks: [Uint8Array.of(1)], error: new Error("device lost") });
    await assert.rejects(readLoop(port, () => {}), /device lost/);
    assert.equal(port.released.reader, true);
});

test("writePort writes the bytes, then releases the writer, even on failure", async () => {
    const port = fakePort();
    await writePort(port, Uint8Array.of(104, 105));
    assert.deepEqual(port.written.map((bytes) => [...bytes]), [[104, 105]]);
    assert.equal(port.released.writer, true);

    const failing = fakePort({ error: new Error("disconnected") });
    await assert.rejects(writePort(failing, Uint8Array.of(1)), /disconnected/);
    assert.equal(failing.released.writer, true);
});

test("USB ids are read from the port info, if valid", () => {
    const usb = fakePort({ info: { usbVendorId: 0x2341, usbProductId: 0x0043 } });
    assert.equal(usbVendorId(usb), 0x2341);
    assert.equal(usbProductId(usb), 0x0043);

    // Bluetooth ports have no USB ids; very old browsers have no getInfo
    assert.equal(usbVendorId(fakePort({ info: {} })), undefined);
    assert.equal(usbVendorId(fakePort()), undefined);
    assert.equal(usbVendorId(fakePort({ info: { usbVendorId: 0x10000 } })), undefined);
    assert.equal(usbVendorId(fakePort({ info: { usbVendorId: 1.5 } })), undefined);
});

test("port selection and opening go through the given serial API", async () => {
    const port = { opened: null, open: async (options) => (port.opened = options) };
    const serial = { getPorts: async () => [port], requestPort: async () => port };

    assert.deepEqual(await getPorts(serial), [port]);
    assert.equal(await requestPort(serial), port);
    await openPort(port, 115200);
    assert.deepEqual(port.opened, { baudRate: 115200 });
});
