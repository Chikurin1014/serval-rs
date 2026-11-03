// The serial ports of the `serial-bridge` program, for `extension.rs`: where a page
// can use no Web Serial (as VS Code's), what shows it passes on lines of JSON.
// Tests pass a fake `host`.

let bridge;

/** Whether the page is in the VS Code extension, which has a bridge. */
export function isActive(host = window) {
  return host.servalExtension === true;
}

/** Talks through `host.parent`, as a line in and a line out of each message. */
export function connect(host = window) {
  let next = 1;
  const answers = new Map();
  const reads = new Map();

  const finish = (
    read,
    error,
  ) => (error ? read.reject(new Error(error)) : read.resolve());

  host.addEventListener("message", ({ source, data }) => {
    if (source !== host.parent || typeof data !== "string") {
      return;
    }
    const line = JSON.parse(data);
    if (line.lost) {
      for (const answer of answers.values()) {
        answer.reject(new Error(line.lost));
      }
      answers.clear();
      for (const read of reads.values()) {
        finish(read, line.lost);
      }
      reads.clear();
    } else if (line.id !== undefined) {
      const answer = answers.get(line.id);
      answers.delete(line.id);
      if (line.error) {
        answer?.reject(new Error(line.error));
      } else {
        answer?.resolve(line);
      }
    } else if (line.data !== undefined) {
      reads.get(line.path)?.onChunk(fromHex(line.data));
    } else if (line.ended) {
      const read = reads.get(line.path);
      reads.delete(line.path);
      if (read) {
        finish(read, line.error);
      }
    }
  });

  const ask = (op, request) =>
    new Promise((resolve, reject) => {
      const id = next++;
      answers.set(id, { resolve, reject });
      host.parent.postMessage(JSON.stringify({ id, op, ...request }), "*");
    });

  return { ask, reads };
}

function connected() {
  bridge ??= connect();
  return bridge;
}

export async function listPorts({ ask } = connected()) {
  return (await ask("list")).ports;
}

export function portPath(port) {
  return port.path;
}

export function usbDevice(port) {
  return Number.isInteger(port.vendorId) && Number.isInteger(port.productId)
    ? [port.vendorId, port.productId, port.manufacturer, port.product]
    : null;
}

export async function openPort(
  path,
  baudRate,
  dataBits,
  stopBits,
  parity,
  flowControl,
  { ask } = connected(),
) {
  await ask("open", {
    path,
    baudRate,
    dataBits,
    stopBits,
    parity,
    flowControl,
  });
}

/** Ends as the port is closed, or with why it is lost. */
export function readLoop(path, onChunk, { ask, reads } = connected()) {
  return new Promise((resolve, reject) => {
    reads.set(path, { onChunk, resolve, reject });
    ask("read", { path }).catch((error) => {
      reads.delete(path);
      reject(error);
    });
  });
}

export async function writePort(path, bytes, { ask } = connected()) {
  await ask("write", { path, data: toHex(bytes) });
}

export async function setSignals(path, dtr, rts, { ask } = connected()) {
  await ask("signals", { path, dtr, rts });
}

export async function setBreak(path, on, { ask } = connected()) {
  await ask("break", { path, on });
}

export async function closePort(path, { ask } = connected()) {
  await ask("close", { path });
}

export function toHex(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}

export function fromHex(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index++) {
    bytes[index] = parseInt(hex.substr(index * 2, 2), 16);
  }
  return bytes;
}
