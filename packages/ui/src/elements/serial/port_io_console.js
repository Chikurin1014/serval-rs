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
  }
  if (reset) {
    node.data = "";
  }
  appendOutput(node, text, MAX_CONSOLE_TEXT);

  // Reading the height lays out all the text: at most once a frame
  if (!frame) {
    frame = requestAnimationFrame(() => {
      frame = 0;
      output.scrollTop = output.scrollHeight;
    });
  }
}

cancelAnimationFrame(frame);
