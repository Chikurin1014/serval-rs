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
    getInfo: () => ({ usbVendorId: 0x2341, usbProductId: 0x0043 }),
    open: async () => {
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
      requestPort: async () => port,
      addEventListener() {},
      removeEventListener() {},
    },
  });
})();
