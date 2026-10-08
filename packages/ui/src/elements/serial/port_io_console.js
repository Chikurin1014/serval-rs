// Appends the text `PortIoConsole` receives to a text node of its own.
// From Rust: `[reset, text]`, then `null` on unmount.

// As Arduino IDE 2's serial monitor
const MAX_CONSOLE_TEXT = 1000000;

const node = document.createTextNode("");
let frame = 0;
// Follows new text only while at the bottom; checked on scrolling, as reading
// the height lays out all the text
let following = true;
let watched = null;
let lastTop = 0;
const onScroll = () => {
  const top = watched.scrollTop;
  if (top < lastTop) {
    following = false;
  } else if (top > lastTop) {
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
    following = true;
    lastTop = 0;
  }
  appendOutput(node, text, MAX_CONSOLE_TEXT);

  // At most once a frame
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
