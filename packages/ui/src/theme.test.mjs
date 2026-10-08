import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript } from "../js-test/load-script.mjs";

const { restoreTheme, storedTheme, toggleTheme } = loadScript(
  new URL("./theme.js", import.meta.url),
);

function fakeStorage(entries = {}) {
  const items = new Map(Object.entries(entries));
  return {
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => items.set(key, value),
    items,
  };
}

const blockedStorage = {
  getItem: () => {
    throw new Error("SecurityError");
  },
  setItem: () => {
    throw new Error("QuotaExceededError");
  },
};

test("restoreTheme applies a stored theme, and only a known one", () => {
  const root = { dataset: {} };
  restoreTheme(root, fakeStorage({ theme: "dark" }));
  assert.equal(root.dataset.theme, "dark");

  for (const storage of [
    fakeStorage(),
    fakeStorage({ theme: "blue" }),
    blockedStorage,
    null,
  ]) {
    const untouched = { dataset: {} };
    restoreTheme(untouched, storage);
    assert.equal(untouched.dataset.theme, undefined);
  }
  assert.equal(storedTheme(blockedStorage), null);
});

test("toggleTheme switches away from the system preference until one is chosen", () => {
  const root = { dataset: {} };
  const storage = fakeStorage();
  assert.equal(toggleTheme(root, storage, true), "light");
  assert.equal(root.dataset.theme, "light");
  assert.equal(storage.items.get("theme"), "light");

  assert.equal(toggleTheme(root, storage, true), "dark");
  assert.equal(toggleTheme({ dataset: {} }, fakeStorage(), false), "dark");
});

test("toggleTheme still switches when the choice cannot be stored", () => {
  const root = { dataset: { theme: "dark" } };
  assert.equal(toggleTheme(root, blockedStorage, false), "light");
  assert.equal(toggleTheme(root, null, false), "dark");
});
