// A Web Serial port for the e2e tests: once opened, it sends
// "temp:20.00\nvolt:3.700\n" every 50 ms.
(() => {
  const encoder = new TextEncoder();
  let tick = 0;
  let timer;

  const port = {
    readable: null,
    writable: null,
    written: [],
    writtenBytes: [],
    sent: 0,
    // Set by the tests to make it fail
    openError: null,
    writeError: null,
    // Set by the tests to hold the writes until `release()`
    holdWrites: false,
    release: () => {},
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
          // As unplugging the device
          port.lose = () => {
            clearInterval(timer);
            controller.error(
              new DOMException("The device has been lost.", "NetworkError"),
            );
          };
          port.receive = (text) => controller.enqueue(encoder.encode(text));
          // Stops the readings, for `receive` alone
          port.mute = () => clearInterval(timer);
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
        async write(chunk) {
          if (port.holdWrites) {
            await new Promise((resolve) => {
              const before = port.release;
              port.release = () => {
                before();
                resolve();
              };
            });
          }
          if (port.writeError) {
            throw new DOMException(port.writeError, "NetworkError");
          }
          port.written.push(new TextDecoder().decode(chunk));
          port.writtenBytes.push(Array.from(chunk));
        },
      });
    },
    close: async () => {
      // As a real port does
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
          throw new DOMException(
            "No port selected by the user.",
            "NotFoundError",
          );
        }
        return port;
      },
      addEventListener() {},
      removeEventListener() {},
    },
  });
})();
