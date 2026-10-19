// Loads a script meant for `document::eval` and returns its functions.

import { readFileSync } from "node:fs";
import vm from "node:vm";

/** With `globals` as the page's, e.g. a fake `requestAnimationFrame`. */
export function loadScript(url, globals = {}) {
  const context = vm.createContext({ ...globals });
  vm.runInContext(readFileSync(url, "utf8"), context);
  return context;
}

/** Copies a value from the script's context, for `deepEqual`. */
export function plain(value) {
  return JSON.parse(JSON.stringify(value));
}
