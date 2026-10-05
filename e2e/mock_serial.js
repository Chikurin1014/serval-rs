// A Web Serial port for the end-to-end tests, injected before the app loads.
//
// Once opened, it sends a `temp` and a `volt` reading every 50 ms, as one
// chunk of two lines: "temp:20.00\nvolt:3.700\n". Cancelling the reader ends the
// stream.
(() => {
  const encoder = new TextEncoder();
  let tick = 0;
  let timer;

  const port = {
    readable: null,
    writable: null,
    // What the app sent, as text; read by the tests
    written: [],
    // How many chunks it sent; read by the tests
    sent: 0,
    // Set by the tests to make it fail: a message for `open` or `write` to
    // fail with, or `cancelRequest` for the user closing the port chooser
    openError: null,
    writeError: null,
    cancelRequest: false,
    getInfo: () => ({ usbVendorId: 0x2341, usbProductId: 0x0043 }),
    open: async () => {
      if (port.openError) {
        throw new DOMException(port.openError, "NetworkError");
      }
      port.readable = new ReadableStream({
        start(controller) {
          const send = () => {
            tick++;
            port.sent++;
            const temp = (20 + 3 * Math.sin(tick / 10)).toFixed(2);
            const volt = (3.3 + 0.4 * Math.cos(tick / 6)).toFixed(3);
            controller.enqueue(encoder.encode(`temp:${temp}\nvolt:${volt}\n`));
          };
          timer = setInterval(send, 50);
          // As unplugging the device does: reading fails
          port.lose = () => {
            clearInterval(timer);
            controller.error(
              new DOMException("The device has been lost.", "NetworkError"),
            );
          };
          // Sends `count` chunks at once, as a fast device would; for the tests
          port.burst = (count) => {
            for (let i = 0; i < count; i++) {
              send();
            }
          };
        },
        cancel() {
          clearInterval(timer);
        },
      });
      port.writable = new WritableStream({
        write(chunk) {
          if (port.writeError) {
            throw new DOMException(port.writeError, "NetworkError");
          }
          port.written.push(new TextDecoder().decode(chunk));
        },
      });
    },
    close: async () => {
      // As a real port does, so the app must stop reading first
      if (port.readable.locked) {
        throw new TypeError("The port's stream is locked");
      }
      clearInterval(timer);
    },
  };

  window.mockSerialPort = port;

  Object.defineProperty(navigator, "serial", {
    value: {
      getPorts: async () => [port],
      requestPort: async () => {
        if (port.cancelRequest) {
          throw new DOMException("No port selected by the user.", "NotFoundError");
        }
        return port;
      },
      addEventListener() {},
      removeEventListener() {},
    },
  });
})();
