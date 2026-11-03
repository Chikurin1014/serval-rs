// The `serial-bridge` program, as `extension.js` runs it: apart from VS Code,
// so it is tested with node alone.

const { spawn } = require("node:child_process");
const readline = require("node:readline");

/**
 * Starts the bridge at `program`: `onLine` gets each line it writes. Once it
 * stops (or cannot start), `onLost` is told why, and each request after is
 * answered that it is lost, so none waits on it.
 */
function startBridge(program, onLine, onLost) {
  const bridge = spawn(program, { stdio: ["pipe", "pipe", "ignore"] });
  let lost;
  const stop = (why) => {
    if (lost) {
      return;
    }
    lost = `The serial bridge stopped: ${why}`;
    onLine(JSON.stringify({ lost }));
    onLost(lost);
  };
  bridge.on("error", (error) => stop(error.message));
  bridge.on("exit", (code, signal) => stop(signal ?? `exit code ${code}`));
  bridge.stdin.on("error", (error) => stop(error.message));
  readline.createInterface({ input: bridge.stdout }).on("line", onLine);
  return {
    send(line) {
      if (lost) {
        onLine(JSON.stringify({ lost }));
      } else {
        bridge.stdin.write(`${line}\n`);
      }
    },
    /** Its input ended, it ends and lets the ports go: not told as lost. */
    end() {
      lost ??= "The panel was closed";
      bridge.stdin.end();
    },
  };
}

module.exports = { startBridge };
