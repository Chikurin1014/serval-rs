// The text side of `port_io_console.js`, kept free of the page so it can be
// tested; `port_io_console.rs` runs this ahead of `port_io_console.js`.

/**
 * Appends `text` to `node` (a DOM `Text`, or anything with its `data`,
 * `length`, `appendData` and `deleteData`), then drops text from the front
 * until at most `maxLength` characters are left.
 *
 * Only the new text is copied in, so a long output costs no more per chunk
 * than a short one.
 */
function appendOutput(node, text, maxLength) {
  node.appendData(text);
  const excess = node.length - maxLength;
  if (excess > 0) {
    node.deleteData(0, cutPoint(node.data, excess));
  }
}

/**
 * Where to cut `excess` characters or more off the front of `text`: after the
 * line they end in, so the first line kept is whole, or right after them
 * where there is no line break (never inside a surrogate pair).
 */
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
