// Files through VS Code's dialogs, for the app in the panel: its requests
// (`{ servalFile }`, as `vscode_files.js` asks) answered with the same `id`.
// Bytes go as base64. A record file is written as it comes, through Node's
// `fs`. VS Code's API is passed in, so it is tested with node alone.

const fs = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");

/** The panel's files: `vscode` is VS Code's API (`window`, `workspace`, `Uri`). */
function createFiles(vscode) {
  const { window, workspace, Uri } = vscode;
  const records = new Map();
  let next = 1;

  /** Where a dialog starts: the workspace's folder, else the home folder. */
  const near = (name) =>
    Uri.joinPath(
      workspace.workspaceFolders?.[0]?.uri ?? Uri.file(os.homedir()),
      name,
    );

  /** Of `extensions` (any if none), as a dialog filters. */
  const filters = (extensions) =>
    extensions?.length
      ? {
        [extensions.map((extension) => extension.toUpperCase()).join(", ")]:
          extensions,
      }
      : undefined;

  const ops = {
    async save({ name, text }) {
      const extension = path.extname(name).slice(1);
      const uri = await window.showSaveDialog({
        defaultUri: near(name),
        filters: filters(extension ? [extension] : []),
      });
      if (uri) {
        await workspace.fs.writeFile(uri, Buffer.from(text));
      }
      return {};
    },

    async open({ extensions }) {
      const [uri] = (await window.showOpenDialog({
        canSelectMany: false,
        filters: filters(extensions),
      })) ?? [];
      if (!uri) {
        return { file: null };
      }
      const data = await workspace.fs.readFile(uri);
      return {
        file: {
          name: path.posix.basename(uri.path),
          data: Buffer.from(data).toString("base64"),
        },
      };
    },

    async record({ name, append, tailBytes }) {
      const uri = append
        ? (await window.showOpenDialog({ canSelectMany: false }))?.[0]
        : await window.showSaveDialog({
          defaultUri: near(name),
          filters: { Record: [path.extname(name).slice(1)] },
        });
      if (!uri) {
        return { record: null };
      }
      const tail = append
        ? await readTail(uri.fsPath, tailBytes)
        : Buffer.alloc(0);
      const id = next++;
      records.set(id, await fs.open(uri.fsPath, append ? "a" : "w"));
      return {
        record: {
          id,
          name: path.basename(uri.fsPath),
          tail: tail.toString("base64"),
        },
      };
    },

    async write({ record, data }) {
      await opened(record).write(Buffer.from(data, "base64"));
      return {};
    },

    async close({ record }) {
      const file = opened(record);
      records.delete(record);
      await file.close();
      return {};
    },
  };

  function opened(record) {
    const file = records.get(record);
    if (!file) {
      throw new Error("The record file is not open");
    }
    return file;
  }

  return {
    /** The answer to `request`, of its `id`: with `error` if it failed. */
    async answer({ id, op, ...request }) {
      try {
        const run = ops[op];
        if (!run) {
          throw new Error(`Unknown request: ${op}`);
        }
        return { id, ...(await run(request)) };
      } catch (error) {
        return { id, error: error?.message ?? String(error) };
      }
    },

    /** Closes the records still open, as the panel closes. */
    async closeAll() {
      const files = [...records.values()];
      records.clear();
      await Promise.allSettled(files.map((file) => file.close()));
    },
  };
}

/** The last `bytes` bytes of the file at `file`. */
async function readTail(file, bytes) {
  const handle = await fs.open(file, "r");
  try {
    const { size } = await handle.stat();
    const length = Math.min(size, bytes);
    const tail = Buffer.alloc(length);
    await handle.read(tail, 0, length, size - length);
    return tail;
  } finally {
    await handle.close();
  }
}

module.exports = { createFiles };
