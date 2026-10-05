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
// Where this last scrolled to: the scroll event that follows arrives late,
// when more text may have come, so it would look like the user scrolled up
let scrolledTo = -1;
const onScroll = () => {
  if (watched.scrollTop === scrolledTo) {
    return;
  }
  following = isAtBottom(
    watched.scrollTop,
    watched.scrollHeight,
    watched.clientHeight,
  );
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
  }
  appendOutput(node, text, MAX_CONSOLE_TEXT);

  // At most once a frame, for the same reason
  if (following && !frame) {
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (following) {
        output.scrollTop = output.scrollHeight;
        scrolledTo = output.scrollTop;
      }
    });
  }
}

cancelAnimationFrame(frame);
watched?.removeEventListener("scroll", onScroll);
