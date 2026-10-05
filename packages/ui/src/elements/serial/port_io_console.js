// Shows the text received by `PortIoConsole` (port_io_console.rs), appending
// to a text node of its own rather than having Dioxus render the whole text
// on every chunk. Runs after `console_output.js`, whose functions it uses.
//
// Messages from Rust:
//   - `[reset, text]`: appends `text`, after clearing the output if `reset`
//   - `null` when the component unmounts

// As Arduino IDE 2's serial monitor
const MAX_CONSOLE_TEXT = 1000000;

const node = document.createTextNode("");
let frame = 0;
// Follows new text only while the user stays at the bottom: scrolled up, they
// are reading. Checked on scrolling, not on each chunk, as reading the
// height lays out all the text
let following = true;
let watched = null;
// Where it was scrolled to last. New text only grows at the bottom, so going
// up from there is the user's doing
let lastTop = 0;
const onScroll = () => {
  const top = watched.scrollTop;
  if (top < lastTop) {
    following = false;
  } else if (top > lastTop) {
    // Text that came since the scroll is not seen yet: allow for some
    following = isAtBottom(
      top,
      watched.scrollHeight,
      watched.clientHeight,
      watched.clientHeight / 4,
    );
  }
  lastTop = top;
};

while (true) {
  const message = await dioxus.recv();
  if (message === null) {
    break;
  }
  const [reset, text] = message;

  const output = document.querySelector("[data-port-io-console]");
  if (!output) {
    continue;
  }
  if (node.parentNode !== output) {
    output.replaceChildren(node);
    watched?.removeEventListener("scroll", onScroll);
    watched = output;
    watched.addEventListener("scroll", onScroll);
  }
  if (reset) {
    node.data = "";
    // Nothing left to read back
    following = true;
    lastTop = 0;
  }
  appendOutput(node, text, MAX_CONSOLE_TEXT);

  // At most once a frame, for the same reason
  if (following && !frame) {
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (following) {
        output.scrollTop = output.scrollHeight;
        lastTop = output.scrollTop;
      }
    });
  }
}

cancelAnimationFrame(frame);
watched?.removeEventListener("scroll", onScroll);
