import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript } from "../../../js-test/load-script.mjs";

const { withAlpha, placeTooltip, segmentDistance, nearestArrow } = loadScript(
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

test("segmentDistance is to the nearest point of the segment", () => {
  // Beside it, past its end, and of a segment of no length
  assert.equal(segmentDistance([5, 3], [0, 0], [10, 0]), 3);
  assert.equal(segmentDistance([13, 4], [0, 0], [10, 0]), 5);
  assert.equal(segmentDistance([3, 4], [0, 0], [0, 0]), 5);
});

test("nearestArrow is the arrow nearest anywhere along it", () => {
  const origin = [0, 0];
  const arrows = [
    [0, origin, [100, 0]],
    [1, origin, [0, 100]],
  ];
  // By the middle of an arrow, not only its tip
  assert.equal(nearestArrow(arrows, [50, 6], 10), 0);
  assert.equal(nearestArrow(arrows, [8, 60], 10), 1);
  // Too far from both
  assert.equal(nearestArrow(arrows, [50, 50], 10), null);
  // Behind the origin both are as near: that of the nearer tip
  const shorter = [[0, origin, [30, 0]], [1, origin, [0, 100]]];
  assert.equal(nearestArrow(shorter, [-2, -2], 10), 0);
  const longer = [[0, origin, [100, 0]], [1, origin, [0, 30]]];
  assert.equal(nearestArrow(longer, [-2, -2], 10), 1);
});
