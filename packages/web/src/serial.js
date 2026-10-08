// Web Serial calls for `serial.rs`; tests pass a fake `serial`.

export function isSupported(nav = navigator) {
  return "serial" in nav;
}

export async function getPorts(serial = navigator.serial) {
  return [...(await serial.getPorts())];
}

export function requestPort(serial = navigator.serial) {
  return serial.requestPort();
}

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

const reading = new WeakMap();

/** Stops `readLoop` first: a port does not close while its stream is locked. */
export async function closePort(port) {
  const active = reading.get(port);
  if (active) {
    await active.reader.cancel();
    await active.released;
  }
  await port.close();
}

export function usbVendorId(port) {
  return u16(port.getInfo?.()?.usbVendorId);
}

export function usbProductId(port) {
  return u16(port.getInfo?.()?.usbProductId);
}

function u16(value) {
  return Number.isInteger(value) && value >= 0 && value <= 0xffff
    ? value
    : undefined;
}

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

const writing = new WeakMap();

/** After the port's earlier writes: a second writer would find the stream locked. */
export function writePort(port, bytes) {
  const write = (writing.get(port) ?? Promise.resolve())
    .catch(() => {})
    .then(async () => {
      const writer = port.writable.getWriter();
      try {
        await writer.write(bytes);
      } finally {
        writer.releaseLock();
      }
    });
  writing.set(port, write);
  return write;
}
