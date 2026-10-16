import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript } from "../../../js-test/load-script.mjs";

const { withAlpha } = loadScript(new URL("./plot_common.js", import.meta.url));

test("withAlpha turns a computed rgb colour translucent", () => {
  assert.equal(
    withAlpha("rgb(176, 176, 176)", 0.35),
    "rgba(176, 176, 176, 0.35)",
  );
  assert.equal(withAlpha("rgba(0, 0, 0, 0.5)", 0.35), "rgba(0, 0, 0, 0.5)");
});
