// The data side of `time_series.js`, kept free of the page and uPlot so it can
// be tested; `time_series.rs` runs this ahead of `time_series.js`.
//
// `series` maps each plotted label to its points, as parallel arrays of times
// (in seconds, uPlot's unit) and values: `Map<string, {t: number[], v: number[]}>`.

/**
 * Applies a `[labels, updates]` message from Rust (see `time_series.js`) to
 * `series`, keeping at most `maxPoints` per label (the latest).
 */
function applyMessage(series, [labels, updates], maxPoints) {
    for (const label of [...series.keys()]) {
        if (!labels.includes(label)) {
            series.delete(label);
        }
    }
    for (const label of labels) {
        if (!series.has(label)) {
            series.set(label, { t: [], v: [] });
        }
    }
    for (const [label, reset, points] of updates) {
        const s = reset ? { t: [], v: [] } : series.get(label);
        series.set(label, s);
        for (const [timestampMs, value] of points) {
            s.t.push(timestampMs / 1000);
            s.v.push(value);
        }
        const excess = s.t.length - maxPoints;
        if (excess > 0) {
            s.t.splice(0, excess);
            s.v.splice(0, excess);
        }
    }
}

/**
 * uPlot's data for `labels`: one shared, sorted array of times, then each
 * label's values at those times, `null` where it has no point.
 */
function alignedData(series, labels) {
    const xs = [...new Set(labels.flatMap((label) => series.get(label).t))].sort((a, b) => a - b);
    const index = new Map(xs.map((x, i) => [x, i]));
    const ys = labels.map((label) => {
        const { t, v } = series.get(label);
        const y = new Array(xs.length).fill(null);
        t.forEach((x, i) => {
            y[index.get(x)] = v[i];
        });
        return y;
    });
    return [xs, ...ys];
}

/** `rgb(r, g, b)` (as `getComputedStyle` gives it) with an alpha. */
function withAlpha(color, alpha) {
    return color.replace(/^rgb\((.*)\)$/, `rgba($1, ${alpha})`);
}
