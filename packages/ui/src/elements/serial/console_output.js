// The text side of `port_io_console.js`, apart so it can be tested.

const TRIM_TO = 0.9;

/**
 * Appends `text` to `node`. Past `maxLength`, drops the oldest text down to
 * `TRIM_TO` of it, so the rest is not copied on every chunk.
 */
function appendOutput(node, text, maxLength) {
  node.appendData(text);
  if (node.length > maxLength) {
    const excess = node.length - Math.floor(maxLength * TRIM_TO);
    node.deleteData(0, cutPoint(node.data, excess));
  }
}

/** Where to cut `excess` characters or more off `text`: at a line end. */
function cutPoint(text, excess) {
  if (text[excess - 1] === "\n") {
    return excess;
  }
  const newline = text.indexOf("\n", excess);
  if (newline !== -1) {
    return newline + 1;
  }
  const code = text.charCodeAt(excess);
  return code >= 0xdc00 && code <= 0xdfff ? excess + 1 : excess;
}

/** Whether a scrolled element shows its end, within `slack` pixels. */
function isAtBottom(scrollTop, scrollHeight, clientHeight, slack = 16) {
  return scrollHeight - scrollTop - clientHeight <= slack;
}
