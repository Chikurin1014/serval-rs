// Web Serial calls for `serial.rs`, imported through wasm-bindgen.
//
// Functions taking `serial` default to `navigator.serial`; tests pass a fake.

/** Whether this browser has Web Serial (Chromium-based ones on desktop, on an
 * HTTPS page or `localhost`). */
export function isSupported(nav = navigator) {
  return "serial" in nav;
}

/** Ports this page was granted access to before. */
export async function getPorts(serial = navigator.serial) {
  return [...(await serial.getPorts())];
}

/** Asks the user to pick a port, granting access to it. */
export function requestPort(serial = navigator.serial) {
  return serial.requestPort();
}

/** Opens the port with every setting given, so none is left to a default. */
export function openPort(
  port,
  baudRate,
  dataBits,
  stopBits,
  parity,
  flowControl,
) {
  return port.open({ baudRate, dataBits, stopBits, parity, flowControl });
}

// The reader `readLoop` holds on each port, and when it is released
const reading = new WeakMap();

/**
 * Closes the port, stopping `readLoop` on it first: a port does not close
 * while its stream is locked to a reader.
 */
export async function closePort(port) {
  const active = reading.get(port);
  if (active) {
    await active.reader.cancel();
    await active.released;
  }
  await port.close();
}

/** USB vendor id of the port, or `undefined` if it has none (e.g. not USB). */
export function usbVendorId(port) {
  return u16(port.getInfo?.()?.usbVendorId);
}

/** USB product id of the port, or `undefined` if it has none (e.g. not USB). */
export function usbProductId(port) {
  return u16(port.getInfo?.()?.usbProductId);
}

function u16(value) {
  return Number.isInteger(value) && value >= 0 && value <= 0xffff
    ? value
    : undefined;
}

/**
 * Passes each chunk read from the port to `onChunk` until the stream ends
 * (e.g. the port is closed), then releases the reader.
 */
export async function readLoop(port, onChunk) {
  const reader = port.readable.getReader();
  let release;
  const released = new Promise((resolve) => (release = resolve));
  reading.set(port, { reader, released });
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) {
        break;
      }
      if (value) {
        onChunk(value);
      }
    }
  } finally {
    reader.releaseLock();
    reading.delete(port);
    release();
  }
}

/** Writes `bytes` (a `Uint8Array`) to the port, then releases the writer. */
export async function writePort(port, bytes) {
  const writer = port.writable.getWriter();
  try {
    await writer.write(bytes);
  } finally {
    writer.releaseLock();
  }
}
