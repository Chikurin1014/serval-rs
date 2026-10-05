// Loads a script meant for `document::eval` (not a module) and returns its
// top-level functions, for testing them in Node.

import { readFileSync } from "node:fs";
import vm from "node:vm";

export function loadScript(url) {
  const context = vm.createContext({});
  vm.runInContext(readFileSync(url, "utf8"), context);
  return context;
}

/** Copies a value made in the script's context into this one, for `deepEqual`. */
export function plain(value) {
  return JSON.parse(JSON.stringify(value));
}
