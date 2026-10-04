import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript, plain } from "../../js-test/load-script.mjs";

const { alignedData, applyMessage, withAlpha } = loadScript(
  new URL("./time_series_data.js", import.meta.url),
);

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
