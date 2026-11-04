// Files through VS Code's dialogs, for `vscode_files.rs`: each a request
// (`{ servalFile }`) answered with the same `id`. Bytes go as base64. Tests pass
// a fake `host`.

let files;

/** Asks through `host.parent`, as a request and an answer of each. */
export function connect(host = window) {
  let next = 1;
  const answers = new Map();
  host.addEventListener("message", ({ source, data }) => {
    if (source !== host.parent || !data?.servalFile) {
      return;
    }
    const { id, error, ...answer } = data.servalFile;
    const asked = answers.get(id);
    answers.delete(id);
    if (error) {
      asked?.reject(new Error(error));
    } else {
      asked?.resolve(answer);
    }
  });
  return {
    ask: (op, request) =>
      new Promise((resolve, reject) => {
        const id = next++;
        answers.set(id, { resolve, reject });
        host.parent.postMessage({ servalFile: { id, op, ...request } }, "*");
      }),
  };
}

function connected() {
  files ??= connect();
  return files;
}

/** Saves `text` where the user picks, suggested as `name`. */
export async function saveFile(name, text, { ask } = connected()) {
  await ask("save", { name, text });
}

/** A file the user picks, of `extensions` (any if none): null if they cancel. */
export async function openFile(extensions, { ask } = connected()) {
  const { file } = await ask("open", { extensions });
  return file ? { name: file.name, bytes: fromBase64(file.data) } : null;
}

export function fileName(file) {
  return file.name;
}

export function fileBytes(file) {
  return file.bytes;
}

/**
 * A record file the user picks, as a sink for `recording` (of
 * `record_file.js`): a new one suggested as `name`, or with `append`, one to
 * add to, with its last `tailBytes` bytes. Null if they cancel.
 */
export async function pickRecordFile(
  name,
  append,
  tailBytes,
  { ask } = connected(),
) {
  const { record } = await ask("record", { name, append, tailBytes });
  if (!record) {
    return null;
  }
  return {
    name: record.name,
    tail: fromBase64(record.tail),
    write: (bytes) =>
      ask("write", { record: record.id, data: toBase64(bytes) }),
    close: () => ask("close", { record: record.id }),
  };
}

export function toBase64(bytes) {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

export function fromBase64(text) {
  return Uint8Array.from(atob(text ?? ""), (char) => char.charCodeAt(0));
}
