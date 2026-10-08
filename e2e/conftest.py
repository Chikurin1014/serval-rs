"""Fixtures for the e2e tests: the app built into `target/e2e` (or
`SERVAL_E2E_APP`), opened in Chromium with a mock Web Serial port."""

import functools
import os
import subprocess
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest
from playwright.sync_api import Browser, sync_playwright

from app import App

ROOT = Path(__file__).resolve().parent.parent
MOCK_SERIAL = Path(__file__).resolve().parent / "mock_serial.js"


@pytest.fixture(scope="session")
def app_dir() -> Path:
    if built := os.environ.get("SERVAL_E2E_APP"):
        return Path(built)
    target = ROOT / "target" / "e2e"
    subprocess.run(
        ["dx", "build", "--platform", "web"],
        cwd=ROOT / "packages" / "web",
        env={**os.environ, "CARGO_TARGET_DIR": str(target)},
        check=True,
    )
    return target / "dx" / "web" / "debug" / "web" / "public"


class _QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass


@pytest.fixture(scope="session")
def base_url(app_dir: Path):
    handler = functools.partial(_QuietHandler, directory=str(app_dir))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    yield f"http://127.0.0.1:{server.server_port}/"
    server.shutdown()


@pytest.fixture(scope="session")
def browser():
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        yield browser
        browser.close()


@pytest.fixture
def app(browser: Browser, base_url: str):
    """The app freshly loaded; fails on an uncaught error in the page."""
    page = browser.new_page(viewport={"width": 1280, "height": 800})
    page.add_init_script(path=str(MOCK_SERIAL))
    app = App(page)
    page.goto(base_url)
    page.locator(".toolbar-tabs").wait_for()
    yield app
    page.close()
    assert app.errors == [], "uncaught errors in the page"
