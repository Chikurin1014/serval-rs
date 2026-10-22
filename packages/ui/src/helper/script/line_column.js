// A column of text beside an xterm.js terminal, by its lines.

/**
 * A column in `column` beside xterm.js' `term`, of `scrollback` lines: each
 * line marked with a text, drawn row by row as the terminal scrolls (none in
 * the alternate screen). Lines are counted from the first ever written: those
 * trimmed off the scrollback, then the buffer's own. One marker on the newest
 * line marked tells how many were trimmed: xterm.js tells each marker of each
 * trim, too slow for one a line.
 */
function lineColumn(term, column, scrollback) {
  // `{ line, text }`, in line order
  let marks = [];
  // The marker on the cursor's line as last settled, and its count
  let anchor = null;
  let showing = false;

  const trimmed = () => (anchor ? anchor.count - anchor.marker.line : 0);

  function cursorLine() {
    const buffer = term.buffer.active;
    return trimmed() + buffer.baseY + buffer.cursorY;
  }

  function show() {
    if (!showing) {
      showing = true;
      requestAnimationFrame(draw);
    }
  }

  function draw() {
    showing = false;
    if (!column) {
      return;
    }
    const buffer = term.buffer.active;
    const screen = term.element?.querySelector(".xterm-screen");
    const rowHeight = screen ? screen.clientHeight / term.rows : 0;
    while (column.children.length < term.rows) {
      column.appendChild(document.createElement("div"));
    }
    while (column.children.length > term.rows) {
      column.lastChild.remove();
    }
    // From the first mark in view: they are in line order
    const top = trimmed() + buffer.viewportY;
    let index = 0;
    let high = marks.length;
    while (index < high) {
      const middle = (index + high) >> 1;
      if (marks[middle].line < top) {
        index = middle + 1;
      } else {
        high = middle;
      }
    }
    for (let row = 0; row < term.rows; row++) {
      const line = top + row;
      const cell = column.children[row];
      cell.style.height = cell.style.lineHeight = `${rowHeight}px`;
      while (index < marks.length && marks[index].line < line) {
        index++;
      }
      const mark = marks[index];
      cell.textContent =
        buffer.type === "normal" && mark?.line === line ? mark.text : "";
    }
  }

  term.onScroll(show);
  term.onRender(show);
  term.onResize(show);

  return {
    /** How many lines were trimmed off the scrollback. */
    trimmed,
    /** The cursor's line, counted from the first ever. */
    cursorLine,
    /** The last line marked, counted from the first ever; -1 for none. */
    lastMarked: () => marks.at(-1)?.line ?? -1,
    /** Marks `line` (from the first ever, after those marked) with `text`. */
    mark(line, text) {
      marks.push({ line, text });
    },
    /** Done marking for now: the marks kept to the scrollback, drawn anew. */
    settle() {
      const lost = trimmed();
      const cursor = cursorLine();
      const marker = term.registerMarker(0);
      if (marker) {
        anchor?.marker.dispose();
        anchor = { marker, count: cursor };
      }
      // Those trimmed off, in a while: not one by one
      if (marks.length > 2 * scrollback) {
        marks = marks.slice(marks.findIndex((mark) => mark.line >= lost));
      }
      show();
    },
    /** Forgets all marked, as the terminal is reset. */
    forget() {
      anchor?.marker.dispose();
      anchor = null;
      marks = [];
      show();
    },
  };
}
