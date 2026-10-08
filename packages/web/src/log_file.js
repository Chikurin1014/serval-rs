// Log files through the File System Access API, for `log_file.rs`; tests pass
// a fake `win` and clock.

// What a writable stream writes goes to the file only once it is closed: so it
// is closed (and opened again) this often, not to lose more on a crash
const COMMIT_MS = 10000;

export function isSupported(win = window) {
  return "showSaveFilePicker" in win;
}

/**
 * A log file the user picks: a new one named `serval-<local time>.<extension>`,
 * or with `append`, one to add to. Null if they cancel.
 */
export async function pickLogFile(
  extension,
  append,
  { win = window, now = Date.now, commitMs = COMMIT_MS } = {},
) {
  let handle;
  try {
    if (append) {
      [handle] = await win.showOpenFilePicker();
    } else {
      handle = await win.showSaveFilePicker({
        suggestedName: `serval-${localTime(new Date(now()))}.${extension}`,
        types: [
          { description: "Log", accept: { "text/plain": [`.${extension}`] } },
        ],
      });
    }
  } catch (error) {
    if (error.name === "AbortError") {
      return null;
    }
    throw error;
  }
  const log = {
    handle,
    writable: null,
    chain: Promise.resolve(),
    error: null,
    now,
    commitMs,
    committed: now(),
  };
  log.writable = await openAtEnd(handle, append);
  return log;
}

/** After the earlier writes, in order; the first failure is kept for `closeLogFile`. */
export function writeLogFile(log, bytes) {
  log.chain = log.chain
    .then(async () => {
      if (log.error) {
        return;
      }
      await log.writable.write(bytes);
      if (log.now() - log.committed >= log.commitMs) {
        await log.writable.close();
        log.writable = await openAtEnd(log.handle, true);
        log.committed = log.now();
      }
    })
    .catch((error) => {
      log.error ??= error;
    });
}

/** After the writes; fails with the first failure of any. */
export async function closeLogFile(log) {
  await log.chain;
  try {
    await log.writable.close();
  } catch (error) {
    log.error ??= error;
  }
  if (log.error) {
    throw log.error;
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
    `${date.getFullYear()}${two(date.getMonth() + 1)}${two(date.getDate())}-` +
    `${two(date.getHours())}${two(date.getMinutes())}${two(date.getSeconds())}`
  );
}
