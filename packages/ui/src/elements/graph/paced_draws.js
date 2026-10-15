// Paces the graphs' redraws by how the page's frames come: each frame while
// they come on time, fewer as they come late, so the page stays smooth while
// the graphs (their canvases, painted after the scripts) are heavy. Prepended
// to each graph's script; the pacer is one for the page, on `globalThis`.

// Times apart for redraws, in ms: each frame, then 30, 20 and 10 a second
const PACE_STEPS = [0, 33, 50, 100];
// Frames looked at together: about half a second's
const PACE_WINDOW = 30;
// A frame is late past this many times the frames' time apart at best: the
// least of the windows' middle ones so far (as the display's refresh is,
// whatever it is; even as all of a window are late, and not by one stray)
const LATE_FRAME_FACTOR = 1.5;
// But no longer than a 30 Hz display's: of a page busy from the start, frames
// as slow as that are not as it should be
const SLOWEST_FRAME_MS = 1000 / 30;
// Fewer redraws once this part of a window's frames are late
const LATE_PART = 0.2;
// More again after this many windows of no late frame: slower down than up,
// so it does not swing as the redraws it saves let the frames come on time
const ON_TIME_WINDOWS = 4;
// Times apart longer than this are of a page not shown (a tab in the back),
// not of one too busy
const HIDDEN_GAP_MS = 1000;

/**
 * The step of `PACE_STEPS` after a window of frames `gaps` (ms apart), from
 * `step` with `onTime` windows of no late frame before, frames at best
 * `shortest` apart before: `[step, onTime, shortest]`.
 */
function nextPace(step, onTime, gaps, shortest = Infinity) {
  const shown = gaps.filter((gap) => gap <= HIDDEN_GAP_MS);
  if (!shown.length) {
    return [step, onTime, shortest];
  }
  const sorted = [...shown].sort((a, b) => a - b);
  shortest = Math.min(shortest, sorted[Math.floor(sorted.length / 2)]);
  const frame = Math.min(Math.max(shortest, 4), SLOWEST_FRAME_MS);
  const late = shown.filter((gap) => gap > frame * LATE_FRAME_FACTOR).length;
  if (late > shown.length * LATE_PART) {
    return [Math.min(step + 1, PACE_STEPS.length - 1), 0, shortest];
  }
  if (late) {
    return [step, 0, shortest];
  }
  if (onTime + 1 >= ON_TIME_WINDOWS) {
    return [Math.max(step - 1, 0), 0, shortest];
  }
  return [step, onTime + 1, shortest];
}

/** The page's pacer, made as a graph first asks for it. */
function drawPacer() {
  if (!globalThis.servalDrawPacer) {
    let step = 0;
    let onTime = 0;
    let shortest = Infinity;
    let gaps = [];
    let last = null;
    const frame = (now) => {
      if (last !== null) {
        gaps.push(now - last);
      }
      last = now;
      if (gaps.length >= PACE_WINDOW) {
        [step, onTime, shortest] = nextPace(step, onTime, gaps, shortest);
        gaps = [];
      }
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
    globalThis.servalDrawPacer = {
      /** The time apart for redraws now, in ms. */
      interval: () => PACE_STEPS[step],
    };
  }
  return globalThis.servalDrawPacer;
}

/**
 * Runs `draw` on a frame as it is scheduled: at most once a frame, and each
 * the pacer's time apart, or `atLeastMs`, after the last.
 */
function pacedDraws(draw, atLeastMs = 0) {
  let frame = 0;
  let waiting = 0;
  let drawnAt = -Infinity;
  const onFrame = () => {
    waiting = 0;
    frame = requestAnimationFrame((now) => {
      frame = 0;
      drawnAt = now;
      draw();
    });
  };
  return {
    schedule() {
      if (frame || waiting) {
        return;
      }
      const every = Math.max(drawPacer().interval(), atLeastMs);
      const wait = drawnAt + every - performance.now();
      if (wait > 0) {
        waiting = setTimeout(onFrame, wait);
      } else {
        onFrame();
      }
    },
    cancel() {
      cancelAnimationFrame(frame);
      clearTimeout(waiting);
    },
  };
}
