"""Records the documents' videos and screenshots of the app as it is built.

    python docs/media/capture.py <app> <out>

`<app>` is the app built (as `dx build` or `nix build .#web-app` gives it), and
`<out>` where the media go (`docs/src/media`, as the documents link to them).
The app is driven in Chromium through the e2e tests' mock Web Serial port, fed
the readings of a made-up device; each recording is cut to what follows its
setup.
"""

import functools
import json
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from playwright.sync_api import Page, sync_playwright

HERE = Path(__file__).resolve().parent
MOCK_SERIAL = HERE.parent.parent / "e2e" / "mock_serial.js"
SIZE = {"width": 1280, "height": 800}


# The introduction's demo: the maps the app starts with (Decode, then numbers by
# `name: value`), a Match of vectors by `name=(x, y, z)` and one of the logs of
# the motor, the battery and the sensor; a graph of each kind, of three series
# each (of `DEVICE`'s readings)
INTRO_LAYOUT = json.loads((HERE / "intro_capture_layout.json").read_text())


# A made-up device's readings, in place of the mock's own: three numbers and
# three vectors each 100 ms, and now and then a log of its motor, battery or
# sensor
DEVICE = """() => {
    const port = window.mockSerialPort;
    port.mute();
    let tick = 0;
    // Not of `name: value`, which the maps would take as numbers
    const logs = [
        '[info] motor started at 1200 rpm',
        '[info] sensor calibrated',
        '[warn] battery low (3.4 V)',
        '[info] motor at 1800 rpm',
        '[error] motor stalled',
        '[warn] sensor drifting',
        '[info] battery full',
        '[warn] motor hot (72 C)',
    ];
    const f = (x, digits = 2) => x.toFixed(digits);
    window.demoDevice = setInterval(() => {
        tick++;
        const t = tick / 10;
        let text =
            `temp: ${f(22 + 2 * Math.sin(t / 3) + Math.random() * 0.3)}\\n` +
            `humid: ${f(45 + 6 * Math.cos(t / 5) + Math.random())}\\n` +
            `volt: ${f(12 + 1.5 * Math.sin(t) + Math.random() * 0.2, 3)}\\n` +
            `acc=(${f(Math.sin(t))}, ${f(Math.cos(t))}, ${f(0.6)})\\n` +
            `gyro=(${f(0.8 * Math.cos(t / 2))}, ${f(0.5)}, ${f(0.8 * Math.sin(t / 2))})\\n` +
            `mag=(${f(-0.4)}, ${f(0.7 * Math.sin(t / 4))}, ${f(0.7 * Math.cos(t / 4))})\\n`;
        if (tick % 4 === 0) {
            text += logs[(tick / 4) % logs.length] + '\\n';
        }
        port.receive(text);
    }, 100);
}"""

# A pointer drawn where the mouse is, and a ring where it clicks: the recording
# shows neither of its own
POINTER = """() => {
    const pointer = document.createElement('div');
    pointer.style.cssText = 'position:fixed;z-index:99999;left:0;top:0;width:18px;' +
        'height:18px;margin:-9px 0 0 -9px;border-radius:50%;pointer-events:none;' +
        'background:rgba(37,99,235,0.35);border:2px solid rgba(37,99,235,0.9);' +
        'transition:transform 120ms';
    document.body.append(pointer);
    document.addEventListener('mousemove', (event) => {
        pointer.style.left = `${event.clientX}px`;
        pointer.style.top = `${event.clientY}px`;
    }, true);
    document.addEventListener('mousedown', () => pointer.style.transform = 'scale(0.6)', true);
    document.addEventListener('mouseup', () => pointer.style.transform = '', true);
}"""


class _QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass


def serve(app: Path) -> str:
    handler = functools.partial(_QuietHandler, directory=str(app))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{server.server_port}/"


def load_layout(page: Page, layout: dict):
    page.locator("#layout-file-input").set_input_files(
        files=[
            {
                "name": "layout.json",
                "mimeType": "application/json",
                "buffer": json.dumps(layout).encode(),
            }
        ]
    )
    toast = page.locator("[role=alertdialog]", has_text="Layout imported")
    toast.wait_for()
    # Gone, not to be in the recording
    toast.wait_for(state="detached", timeout=15000)


def click(page: Page, locator, steps: int = 20):
    """Moves the pointer to `locator`, as one would, and clicks it."""
    box = locator.bounding_box()
    page.mouse.move(
        box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, steps=steps
    )
    page.wait_for_timeout(150)
    locator.click()


def demo(page: Page):
    """About 15 s: the port asked for and opened, then each tab in turn."""
    toolbar = page.locator(".toolbar-port")
    click(page, toolbar.locator("button").first)
    baudrate = page.get_by_placeholder("Baudrate (e.g. 9600)")
    click(page, baudrate)
    baudrate.press_sequentially("115200", delay=60)
    click(page, page.get_by_role("button", name="Open port"))
    page.get_by_role("button", name="Close port").wait_for()
    page.evaluate(DEVICE)
    page.wait_for_timeout(3000)
    tabs = page.locator(".toolbar-tabs")
    click(page, tabs.get_by_role("tab", name="Data"))
    page.wait_for_timeout(3500)
    click(page, tabs.get_by_role("tab", name="Graph"))
    page.wait_for_timeout(4500)


def record(browser, url: str, out: Path, name: str):
    """Records `demo` into `out/<name>.webm`, with its last frame as the poster."""
    with tempfile.TemporaryDirectory() as videos:
        context = browser.new_context(
            viewport=SIZE, record_video_dir=videos, record_video_size=SIZE
        )
        page = context.new_page()
        started = time.monotonic()
        page.add_init_script(path=str(MOCK_SERIAL))
        page.goto(url)
        page.locator(".toolbar-tabs").wait_for()
        load_layout(page, INTRO_LAYOUT)
        page.evaluate(POINTER)
        page.mouse.move(SIZE["width"] / 2, SIZE["height"] / 2)
        page.wait_for_timeout(300)
        setup = time.monotonic() - started
        demo(page)
        page.screenshot(path=out / f"{name}.png")
        raw = page.video.path()
        context.close()
        # From the end of the setup, smaller as VP9
        subprocess.run(
            [
                "ffmpeg",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                f"{setup:.2f}",
                "-i",
                str(raw),
                "-c:v",
                "libvpx-vp9",
                "-b:v",
                "0",
                "-crf",
                "36",
                "-row-mt",
                "1",
                "-an",
                str(out / f"{name}.webm"),
            ],
            check=True,
        )


def main():
    app, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    if not shutil.which("ffmpeg"):
        sys.exit("ffmpeg is needed, to cut the recordings")
    url = serve(app)
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        record(browser, url, out, "demo")
        browser.close()


if __name__ == "__main__":
    main()
