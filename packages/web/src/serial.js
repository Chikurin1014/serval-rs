// Web Serial calls for `serial.rs`, imported through wasm-bindgen.
//
// Functions taking `serial` default to `navigator.serial`; tests pass a fake.

/** Ports this page was granted access to before. */
export async function getPorts(serial = navigator.serial) {
  return [...(await serial.getPorts())];
}

/** Asks the user to pick a port, granting access to it. */
export function requestPort(serial = navigator.serial) {
  return serial.requestPort();
}

export function openPort(port, baudRate) {
  return port.open({ baudRate });
}

export function closePort(port) {
  return port.close();
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
