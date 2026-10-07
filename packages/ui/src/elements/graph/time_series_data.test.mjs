import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript, plain } from "../../../js-test/load-script.mjs";

const { alignedData, applyMessage, forLogScale, timeOfDay, timeRange, withAlpha } =
  loadScript(new URL("./time_series_data.js", import.meta.url));

function asObject(series) {
  return Object.fromEntries([...series].map(([label, s]) => [label, plain(s)]));
}

test("applyMessage appends new points, in seconds", () => {
  const series = new Map();
  applyMessage(
    series,
    [
      ["temp"],
      [
        [
          "temp",
          true,
          [
            [1000, 20],
            [2000, 21],
          ],
        ],
      ],
    ],
    100,
  );
  applyMessage(series, [["temp"], [["temp", false, [[3000, 22]]]]], 100);
  assert.deepEqual(asObject(series), {
    temp: { t: [1, 2, 3], v: [20, 21, 22] },
  });
});

test("applyMessage replaces a label's points on reset", () => {
  const series = new Map();
  applyMessage(
    series,
    [
      ["temp"],
      [
        [
          "temp",
          true,
          [
            [1000, 20],
            [2000, 21],
          ],
        ],
      ],
    ],
    100,
  );
  applyMessage(series, [["temp"], [["temp", true, [[9000, 5]]]]], 100);
  assert.deepEqual(asObject(series), { temp: { t: [9], v: [5] } });
});

test("applyMessage drops labels no longer sent and adds empty ones", () => {
  const series = new Map();
  applyMessage(series, [["temp", "volt"], [["temp", true, [[1000, 20]]]]], 100);
  assert.deepEqual(asObject(series), {
    temp: { t: [1], v: [20] },
    volt: { t: [], v: [] },
  });

  applyMessage(series, [["volt"], []], 100);
  assert.deepEqual([...series.keys()], ["volt"]);
});

test("applyMessage keeps only the latest points", () => {
  const series = new Map();
  const points = [1, 2, 3, 4, 5].map((s) => [s * 1000, s]);
  applyMessage(series, [["temp"], [["temp", true, points]]], 3);
  assert.deepEqual(asObject(series), { temp: { t: [3, 4, 5], v: [3, 4, 5] } });
});

test("alignedData puts every label on one sorted time axis, null where missing", () => {
  const series = new Map([
    ["temp", { t: [1, 3], v: [20, 22] }],
    ["volt", { t: [2, 3], v: [3.3, 3.4] }],
  ]);
  assert.deepEqual(plain(alignedData(series, ["temp", "volt"])), [
    [1, 2, 3],
    [20, null, 22],
    [null, 3.3, 3.4],
  ]);
  assert.deepEqual(plain(alignedData(new Map(), [])), [[]]);
});

test("withAlpha turns a computed rgb colour translucent", () => {
  assert.equal(
    withAlpha("rgb(176, 176, 176)", 0.35),
    "rgba(176, 176, 176, 0.35)",
  );
  // Colours that already have an alpha are left alone
  assert.equal(withAlpha("rgba(0, 0, 0, 0.5)", 0.35), "rgba(0, 0, 0, 0.5)");
});

test("forLogScale leaves out values a log scale cannot show", () => {
  assert.deepEqual(
    plain(
      forLogScale([
        [1, 2, 3, 4],
        [10, 0, -1, null],
        [0.5, 1, 2, 3],
      ]),
    ),
    [
      [1, 2, 3, 4],
      [10, null, null, null],
      [0.5, 1, 2, 3],
    ],
  );
});

test("timeOfDay is HH:MM:SS.SSS in the local time zone", () => {
  const ms = new Date(2026, 9, 6, 9, 5, 3, 7).getTime();
  assert.equal(timeOfDay(ms), "09:05:03.007");
  assert.equal(
    timeOfDay(new Date(2026, 9, 6, 23, 59, 59, 999).getTime()),
    "23:59:59.999",
  );
});

test("timeRange shows the last window, or fits shorter data", () => {
  // Always as wide as the window
  assert.deepEqual(plain(timeRange(100, 103, 60, false)), [43, 103]);
  assert.deepEqual(plain(timeRange(100, 200, 60, false)), [140, 200]);
  // Fitting: the data itself, until it is longer than the window
  assert.deepEqual(plain(timeRange(100, 103, 10, true)), [100, 103]);
  assert.deepEqual(plain(timeRange(100, 200, 10, true)), [190, 200]);
  // No data yet: left to uPlot
  assert.deepEqual(plain(timeRange(null, null, 10, true)), [null, null]);
});
