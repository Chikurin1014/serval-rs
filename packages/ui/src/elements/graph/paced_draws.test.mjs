import assert from "node:assert/strict";
import { test } from "node:test";

import { loadScript } from "../../../js-test/load-script.mjs";

const { nextPace } = loadScript(new URL("./paced_draws.js", import.meta.url));

// A window of frames each `ms` apart, `late` of them `lateMs`
function window(ms, late = 0, lateMs = ms * 3) {
  return [...Array(30 - late).fill(ms), ...Array(late).fill(lateMs)];
}

// As `[step, onTime]`, its frames' shortest apart aside
function pace(...args) {
  const [step, onTime] = nextPace(...args);
  return [step, onTime];
}

test("frames late lower the redraws at once, a step a window", () => {
  assert.deepEqual(pace(0, 0, window(16.7, 10)), [1, 0]);
  assert.deepEqual(pace(1, 0, window(16.7, 10)), [2, 0]);
  // No lower than 10 a second
  assert.deepEqual(pace(3, 0, window(16.7, 10)), [3, 0]);
});

test("all of a window late, by those before", () => {
  const [, , shortest] = nextPace(0, 0, window(16.7));
  assert.equal(shortest, 16.7);
  assert.deepEqual(pace(0, 0, window(40), shortest), [1, 0]);
});

test("a few late frames keep them as they are", () => {
  assert.deepEqual(pace(2, 3, window(16.7, 3)), [2, 0]);
});

test("frames on time raise them again, slowly", () => {
  let [step, onTime] = [2, 0];
  for (let windows = 0; windows < 3; windows++) {
    [step, onTime] = nextPace(step, onTime, window(16.7));
    assert.equal(step, 2);
  }
  [step, onTime] = nextPace(step, onTime, window(16.7));
  assert.deepEqual([step, onTime], [1, 0]);
  // Never more than each frame
  assert.deepEqual(pace(0, 3, window(16.7)), [0, 0]);
});

test("late is by the display's own frames", () => {
  // 120 Hz, frames 8.3 ms apart: 16.7 is late
  assert.deepEqual(pace(0, 0, window(8.3, 10, 16.7)), [1, 0]);
  // 30 Hz, frames 33 ms apart, all on time
  assert.deepEqual(pace(1, 0, window(33.3)), [1, 1]);
});

test("a page not shown is not one late", () => {
  const hidden = [...window(16.7).slice(1), 5000];
  assert.deepEqual(pace(0, 0, hidden), [0, 1]);
  assert.deepEqual(pace(2, 1, Array(30).fill(5000)), [2, 1]);
});

test("a stray short time apart makes no frame late", () => {
  const [, , shortest] = nextPace(0, 0, [3, ...window(16.7).slice(1)]);
  assert.equal(shortest, 16.7);
  assert.deepEqual(pace(0, 0, window(16.7), shortest), [0, 1]);
});

test("frames slow from the start are late all the same", () => {
  // As a page busy from the first, 100 ms apart: not as a display is
  assert.deepEqual(pace(0, 0, window(100)), [1, 0]);
});

/** The pacer, on frames run by `frame(ms)` and a clock at `clock.now`. */
function fakePacer() {
  const clock = { now: 0 };
  let callbacks = [];
  const { drawPacer } = loadScript(
    new URL("./paced_draws.js", import.meta.url),
    {
      performance: { now: () => clock.now },
      requestAnimationFrame: (callback) => callbacks.push(callback),
    },
  );
  const frame = (ms) => {
    clock.now += ms;
    const due = callbacks;
    callbacks = [];
    due.forEach((callback) => callback(clock.now));
  };
  return { pacer: drawPacer(), clock, frame, waiting: () => callbacks.length };
}

test("frames are watched only while redraws are asked for", () => {
  const { pacer, frame, waiting } = fakePacer();
  assert.equal(waiting(), 0);
  pacer.wake();
  assert.equal(waiting(), 1);
  // Once, however often asked
  pacer.wake();
  assert.equal(waiting(), 1);
  for (let i = 0; i < 30; i++) {
    frame(16.7);
  }
  assert.equal(waiting(), 1);
  // Still past `IDLE_MS`: no more frames asked for
  for (let i = 0; i < 40; i++) {
    frame(16.7);
  }
  assert.equal(waiting(), 0);
  pacer.wake();
  assert.equal(waiting(), 1);
});
