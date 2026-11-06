import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript } from "../../../js-test/load-script.mjs";

const { withAlpha, placeTooltip } = loadScript(
  new URL("./plot_common.js", import.meta.url),
);

test("withAlpha turns a computed rgb colour translucent", () => {
  assert.equal(
    withAlpha("rgb(176, 176, 176)", 0.35),
    "rgba(176, 176, 176, 0.35)",
  );
  assert.equal(withAlpha("rgba(0, 0, 0, 0.5)", 0.35), "rgba(0, 0, 0, 0.5)");
});

test("placeTooltip puts a tooltip below right of a point, or where it fits", () => {
  const tooltip = { offsetWidth: 100, offsetHeight: 40, style: {} };
  placeTooltip(tooltip, [50, 30], [400, 300]);
  assert.deepEqual(tooltip.style, { left: "62px", top: "42px" });
  // Near the right and the bottom: on the left and above
  placeTooltip(tooltip, [350, 280], [400, 300]);
  assert.deepEqual(tooltip.style, { left: "238px", top: "228px" });
  // Never out past the top left
  placeTooltip(tooltip, [60, 20], [120, 50]);
  assert.deepEqual(tooltip.style, { left: "0px", top: "0px" });
});
