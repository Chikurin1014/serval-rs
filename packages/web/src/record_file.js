// Record files, for `files.rs`: through the File System Access API, or another
// way's sink (as VS Code's, of `vscode_files.js`), each written in order. Tests
// pass a fake `win` and clock.

// What a writable stream writes goes to the file only once it is closed: so it
// is closed (and opened again) this often, not to lose more on a crash
const COMMIT_MS = 10000;

export function isSupported(win = window) {
  return "showSaveFilePicker" in win;
}

/** A new record's name: `serval-<local time>.<extension>`. */
export function recordName(extension, now = Date.now) {
  return `serval-${localTime(new Date(now()))}.${extension}`;
}

/**
 * A record file the user picks: a new one named by `recordName`, or with
 * `append`, one to add to, with its last `tailBytes` bytes to tell its format
 * by. Null if they cancel.
 */
export async function pickRecordFile(
  extension,
  append,
  tailBytes,
  { win = window, now = Date.now, commitMs = COMMIT_MS } = {},
) {
  let handle;
  try {
    if (append) {
      [handle] = await win.showOpenFilePicker();
    } else {
      handle = await win.showSaveFilePicker({
        suggestedName: recordName(extension, now),
        types: [
          {
            description: "Record",
            accept: { "text/plain": [`.${extension}`] },
          },
        ],
      });
    }
  } catch (error) {
    if (error.name === "AbortError") {
      return null;
    }
    throw error;
  }
  let tail = new Uint8Array();
  if (append) {
    const file = await handle.getFile();
    tail = new Uint8Array(
      await file.slice(Math.max(0, file.size - tailBytes)).arrayBuffer(),
    );
  }
  let writable = await openAtEnd(handle, append);
  let committed = now();
  return recording({
    name: handle.name,
    tail,
    async write(bytes) {
      await writable.write(bytes);
      if (now() - committed >= commitMs) {
        await writable.close();
        writable = await openAtEnd(handle, true);
        committed = now();
      }
    },
    close: () => writable.close(),
  });
}

/**
 * A record of `sink` (its `name`, `tail`, and `write` and `close`, each a
 * promise): its writes in order.
 */
export function recording(sink) {
  const record = {
    sink,
    name: sink.name,
    tail: sink.tail,
    chain: Promise.resolve(),
    error: null,
  };
  record.failure = new Promise((resolve) => {
    record.settle = resolve;
  });
  return record;
}

export function recordFileName(record) {
  return record.name;
}

export function recordFileTail(record) {
  return record.tail;
}

/**
 * The message of the first failed write, as soon as it fails; null once the
 * record closes without one.
 */
export function recordFileFailure(record) {
  return record.failure;
}

function fail(record, error) {
  if (!record.error) {
    record.error = error;
    record.settle(error?.message ?? String(error));
  }
}

/** After the earlier writes, in order; the first failure stops those after. */
export function writeRecordFile(record, bytes) {
  record.chain = record.chain
    .then(async () => {
      if (!record.error) {
        await record.sink.write(bytes);
      }
    })
    .catch((error) => fail(record, error));
}

/** After the writes; fails with the first failure of any. */
export async function closeRecordFile(record) {
  await record.chain;
  try {
    await record.sink.close();
  } catch (error) {
    fail(record, error);
  }
  record.settle(null);
  if (record.error) {
    throw record.error;
  }
}

async function openAtEnd(handle, keep) {
  const writable = await handle.createWritable({ keepExistingData: keep });
  if (keep) {
    await writable.seek((await handle.getFile()).size);
  }
  return writable;
}

/** `YYYYMMDD-HHMMSS` in local time. */
function localTime(date) {
  const two = (n) => String(n).padStart(2, "0");
  return (
    `${date.getFullYear()}${two(date.getMonth() + 1)}${two(date.getDate())}-`
    + `${two(date.getHours())}${two(date.getMinutes())}${
      two(date.getSeconds())
    }`
  );
}
