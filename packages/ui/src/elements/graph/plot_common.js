// What the plots' scripts share: translucent colors, Rust's messages, legends
// and tooltips.
// Prepended to each plot's script, after `theme.js` and
// `helper/script/css_colors.js`.

/** A computed `rgb(...)` color, translucent. */
function withAlpha(color, alpha) {
  return color.replace(/^rgb\((.*)\)$/, `rgba($1, ${alpha})`);
}

/** What Rust sends, until it sends `null` as the plot is dropped. */
async function* messages() {
  while (true) {
    const message = await dioxus.recv();
    if (message === null) {
      return;
    }
    yield message;
  }
}

/**
 * The labels turned off in a plot's legend: as Rust has them, but as last
 * turned here until Rust has that too.
 */
function trackHidden() {
  let hidden = new Set();
  // Sent to Rust and not yet echoed back
  let sent = null;
  return {
    has(label) {
      return hidden.has(label);
    },
    list() {
      return [...hidden];
    },
    /** Turns `label` off or on, as in the legend; Rust is told. */
    set(label, off) {
      if (off) {
        hidden.add(label);
      } else {
        hidden.delete(label);
      }
      const labels = [...hidden].sort();
      sent = JSON.stringify(labels);
      dioxus.send(["hidden", labels]);
    },
    toggle(label) {
      this.set(label, !hidden.has(label));
    },
    /** Those Rust has, unless it is yet to have those last turned here. */
    apply(labels) {
      if (sent !== null) {
        if (JSON.stringify([...labels].sort()) !== sent) {
          return;
        }
        sent = null;
      }
      hidden = new Set(labels);
    },
  };
}

/** Shows the series of uPlot's `plot` but those `hidden` has. */
function showSeries(plot, hidden) {
  plot?.series.forEach((s, index) => {
    if (index > 0 && s.show === hidden.has(s.label)) {
      plot.setSeries(index, { show: !hidden.has(s.label) }, false);
    }
  });
}

/** A plot's tooltip (`.plot-tooltip`, and of `className`), hidden. */
function plotTooltip(className) {
  const tooltip = document.createElement("div");
  tooltip.className = `${className} plot-tooltip`;
  tooltip.hidden = true;
  return tooltip;
}

/**
 * Fills `tooltip` with `label`, then a row of each `[name, text]` of `rows`
 * (a name may be nodes, as a color's swatch and a label).
 */
function fillTooltip(tooltip, label, rows) {
  const title = document.createElement("div");
  title.className = "plot-tooltip-label";
  title.textContent = label;
  const list = document.createElement("dl");
  for (const [name, text] of rows) {
    const term = document.createElement("dt");
    term.append(...[name].flat());
    const value = document.createElement("dd");
    value.textContent = text;
    list.append(term, value);
  }
  tooltip.replaceChildren(title, list);
}

/**
 * Places `tooltip` below right of `[x, y]` in an area `width` by `height`, or
 * on the side with room.
 */
function placeTooltip(tooltip, [x, y], [width, height]) {
  const gap = 12;
  const left = x + gap + tooltip.offsetWidth <= width
    ? x + gap
    : x - gap - tooltip.offsetWidth;
  const top = y + gap + tooltip.offsetHeight <= height
    ? y + gap
    : y - gap - tooltip.offsetHeight;
  tooltip.style.left = `${Math.max(0, left)}px`;
  tooltip.style.top = `${Math.max(0, top)}px`;
}

/** How far `[x, y]` is from the segment from `from` to `to`. */
function segmentDistance([x, y], [fromX, fromY], [toX, toY]) {
  const [dx, dy] = [toX - fromX, toY - fromY];
  const length = dx * dx + dy * dy;
  const along = length === 0
    ? 0
    : Math.min(Math.max(((x - fromX) * dx + (y - fromY) * dy) / length, 0), 1);
  return Math.hypot(x - (fromX + along * dx), y - (fromY + along * dy));
}

/**
 * Of `arrows`, each `[index, from, to]` on screen, the index of that nearest
 * `pointer` anywhere along it, within `reach`; of those as near (as by their
 * shared origin), that of the nearer tip. None if none is.
 */
function nearestArrow(arrows, pointer, reach) {
  let nearest = null;
  let best = [reach, Infinity];
  for (const [index, from, to] of arrows) {
    const away = segmentDistance(pointer, from, to);
    const tip = Math.hypot(pointer[0] - to[0], pointer[1] - to[1]);
    const nearer = away < best[0] - 0.5
      || (away <= best[0] + 0.5 && tip < best[1]);
    if (away <= reach && nearer) {
      nearest = index;
      best = [away, tip];
    }
  }
  return nearest;
}

/**
 * A legend as uPlot's, for a plot not of it: a row of each `[label, color]`
 * entry, marked as by `fill`, turned on or off by clicking it (`hidden`
 * toggled, then `onToggle` called).
 */
function legendTable(colors, hidden, fill, onToggle) {
  const element = document.createElement("table");
  element.className = "u-legend u-inline";
  const body = document.createElement("tbody");
  element.append(body);
  // What it was made of: the entries' labels and colors, which are off
  let made = "";
  return {
    element,
    /** Makes it anew, if `entries` or those off changed; whether it did. */
    update(entries) {
      const makings = JSON.stringify([
        entries.map(([label, color]) => [label, color]),
        hidden.list(),
      ]);
      if (makings === made) {
        return false;
      }
      made = makings;
      body.replaceChildren();
      for (const [label, color] of entries) {
        const row = document.createElement("tr");
        row.className = "u-series";
        row.classList.toggle("u-off", hidden.has(label));
        const cell = document.createElement("th");
        const marker = document.createElement("div");
        marker.className = "u-marker";
        const stroke = colors.color(color);
        marker.style.border = `2px solid ${stroke}`;
        marker.style.background = withAlpha(stroke, fill);
        const name = document.createElement("div");
        name.className = "u-label";
        name.textContent = label;
        cell.append(marker, name);
        row.append(cell);
        row.addEventListener("click", () => {
          hidden.toggle(label);
          onToggle();
        });
        body.append(row);
      }
      return true;
    },
    /** Made anew at the next update, as with the theme's colors. */
    invalidate() {
      made = "";
    },
  };
}

// How quickly a plot's extent follows what the data needs, widening and
// narrowing: time constants, so it is most of the way there in about twice each
const WIDEN_MS = 75;
const NARROW_MS = 300;
// How much wider than needed it may be before it narrows, so it does not
// pulse with each new value
const NARROW_PAST = 1.25;

/**
 * An extent of a plot, eased towards what the data needs, `onChange` called as
 * it moves; `container.dataset.settled` tells tests whether it is there.
 */
function easedExtent(container, onChange) {
  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
  let value = null;
  let goal = null;
  let frame = 0;
  let easedAt = 0;
  const settle = () => {
    container.dataset.settled = String(value === goal);
  };
  function ease(now) {
    frame = 0;
    const time = goal > value ? WIDEN_MS : NARROW_MS;
    value += (goal - value) * (1 - Math.exp(-(now - easedAt) / time));
    easedAt = now;
    if (Math.abs(goal - value) <= goal * 0.002) {
      value = goal;
    } else {
      frame = requestAnimationFrame(ease);
    }
    settle();
    onChange();
  }
  return {
    /** Where it is; `null` before it first follows. */
    get value() {
      return value;
    },
    /**
     * Takes it towards `needed`: at once at first, `now`, or for less motion;
     * else over a few frames, and only once it is much too wide to narrow.
     */
    follow(needed, now = false) {
      if (value === null || now || reducedMotion.matches) {
        cancelAnimationFrame(frame);
        frame = 0;
        value = goal = needed;
      } else if (
        needed > value
        || needed > goal
        || needed * NARROW_PAST < value
      ) {
        goal = needed;
        if (!frame) {
          easedAt = performance.now();
          frame = requestAnimationFrame(ease);
        }
      }
      settle();
    },
    cancel() {
      cancelAnimationFrame(frame);
    },
  };
}
