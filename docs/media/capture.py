"""Records the documents' videos and screenshots of the app as it is built.

    python docs/media/capture.py <app> [<out>...]

`<app>` is the app built (as `dx build` or `nix build .#web-app` gives it), and
each `<out>` where the media go. With none, they go where a working copy has
them: `docs/src/media`, as the documents link to them (and `mdbook build`
takes them from), and `packages/web/public/docs/media`, as the documents
built there (and served by `dx serve`) have them, so they show without
building the documents again.
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
ROOT = HERE.parent.parent
MOCK_SERIAL = ROOT / "e2e" / "mock_serial.js"
SIZE = {"width": 1280, "height": 800}

# Where the media go with no `<out>` given: those of a working copy
DEV_OUTS = [
    ROOT / "docs" / "src" / "media",
    ROOT / "packages" / "web" / "public" / "docs" / "media",
]


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


# The plugin of the Task tab's screenshot: a task pushing data, and one
# printing what the device sends
PLUGIN = """use serval::{data, time};

/// Pushes a sine wave to `wave`, 20 times a second.
pub async fn wave() {
    let start = time::Instant::now();
    loop {
        let t = start.elapsed().as_secs_f64();
        data::push("wave", t.sin())?;
        time::sleep(time::Duration::from_millis(50)).await?;
    }
}

/// Prints each temperature the device sends.
pub async fn watch_temp() {
    let temps = data::get("temp")?;
    while let Some(temp) = temps.next().await {
        println!("temp: {temp}");
    }
}
"""


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


def open_app(browser, url: str, layout: dict | None = None):
    """A page of the app, its port to be asked for; of `layout`, if given."""
    context = browser.new_context(viewport=SIZE)
    page = context.new_page()
    page.add_init_script(path=str(MOCK_SERIAL))
    page.goto(url)
    page.locator(".toolbar-tabs").wait_for()
    if layout is not None:
        load_layout(page, layout)
    return context, page


def open_port(page: Page):
    """The port asked for and opened, reading `DEVICE`."""
    page.locator(".toolbar-port button").first.click()
    page.get_by_placeholder("Baudrate (e.g. 9600)").fill("115200")
    page.get_by_role("button", name="Open port").click()
    page.get_by_role("button", name="Close port").wait_for()
    page.evaluate(DEVICE)


def tab(page: Page, name: str, wait: int = 1500):
    page.locator(".toolbar-tabs").get_by_role("tab", name=name).click()
    page.wait_for_timeout(wait)


def settle(page: Page):
    """The toasts gone, not to be in a screenshot, and a menu just opened
    faded in."""
    page.locator("[role=alertdialog]").first.wait_for(state="detached", timeout=15000)
    page.wait_for_timeout(400)


def shot(page: Page, path: Path):
    settle(page)
    page.screenshot(path=path)


def shot_top(page: Page, path: Path, down_to):
    """The top of the page, across it, down to the bottom of `down_to`."""
    settle(page)
    box = down_to.bounding_box()
    height = min(SIZE["height"], box["y"] + box["height"] + 12)
    page.screenshot(
        path=path, clip={"x": 0, "y": 0, "width": SIZE["width"], "height": height}
    )


def screenshots(browser, url: str, out: Path):
    """The documents' screenshots, each `<out>/<name>.png`."""
    # As one starts: the app's own layout, reading the device
    context, page = open_app(browser, url)
    open_port(page)
    page.wait_for_timeout(3000)
    shot(page, out / "getting-started-console.png")
    tab(page, "Data")
    shot(page, out / "getting-started-data.png")
    tab(page, "Graph", wait=3000)
    shot(page, out / "getting-started-graph.png")

    # The toolbar, and its menus
    tab(page, "Console")
    shot_top(page, out / "toolbar.png", page.locator(".toolbar"))
    page.locator(".port-signal-trigger").click()
    shot_top(
        page, out / "toolbar-signals.png", page.locator(".port-signal-menu-content")
    )
    page.keyboard.press("Escape")
    page.locator(".layout-menu-trigger").click()
    shot_top(page, out / "toolbar-layout.png", page.locator(".layout-menu-content"))
    page.keyboard.press("Escape")

    # The console: in hex, and its record menu
    page.locator(".console-views").get_by_role("tab", name="Hex").click()
    page.wait_for_timeout(1000)
    settle(page)
    page.locator(".console-grid").screenshot(path=out / "console-hex.png")
    page.locator(".console-views").get_by_role("tab", name="Text").click()
    page.locator(".record-start").click()
    page.locator(".record-menu-content").get_by_role("option").first.hover()
    page.wait_for_timeout(500)
    shot(page, out / "console-record.png")
    page.keyboard.press("Escape")

    # The data: a map's card open, and the menu maps are added from
    tab(page, "Data")
    page.locator(".map-card").nth(2).locator(".map-title").click()
    page.wait_for_timeout(1000)
    shot(page, out / "data-map.png")
    page.locator(".add-map-bar").get_by_role(
        "button", name="Number", exact=True
    ).hover()
    page.wait_for_timeout(500)
    shot(page, out / "data-add-map.png")
    context.close()

    # A graph of each kind, as in the introduction's demo
    context, page = open_app(browser, url, INTRO_LAYOUT)
    open_port(page)
    tab(page, "Graph", wait=4500)
    shot(page, out / "graph.png")
    page.locator(".add-graph-bar").get_by_role("button", name="Time series").hover()
    page.wait_for_timeout(500)
    shot(page, out / "graph-add.png")
    context.close()

    # The tasks of `PLUGIN`, both on
    context, page = open_app(browser, url)
    open_port(page)
    page.locator("#plugin-file-input").set_input_files(
        files=[{"name": "demo.rn", "mimeType": "text/plain", "buffer": PLUGIN.encode()}]
    )
    tab(page, "Task", wait=500)
    switches = page.locator(".task-list").get_by_role("switch", name="Toggle task")
    for index in range(switches.count()):
        switches.nth(index).click()
    page.wait_for_timeout(3000)
    shot(page, out / "task.png")
    context.close()


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    app = Path(sys.argv[1])
    outs = [Path(out) for out in sys.argv[2:]] or DEV_OUTS
    # Recorded into the first, then copied to the rest
    out = outs[0]
    out.mkdir(parents=True, exist_ok=True)
    if not shutil.which("ffmpeg"):
        sys.exit("ffmpeg is needed, to cut the recordings")
    url = serve(app)
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        record(browser, url, out, "demo")
        screenshots(browser, url, out)
        browser.close()
    for other in outs[1:]:
        shutil.copytree(out, other, dirs_exist_ok=True)


if __name__ == "__main__":
    main()
