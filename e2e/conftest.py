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


def pytest_configure(config: pytest.Config):
    """Builds the app unless `SERVAL_E2E_APP` gives it: once, before the
    workers of `pytest-xdist` start (as they take this process's environment),
    not in each of them."""
    if os.environ.get("SERVAL_E2E_APP") or hasattr(config, "workerinput"):
        return
    target = ROOT / "target" / "e2e"
    # Into `packages/web/public/docs`, which `dx` serves at `/docs/`
    subprocess.run(["mdbook", "build", "docs"], cwd=ROOT, check=True)
    subprocess.run(
        ["dx", "build", "--platform", "web"],
        cwd=ROOT / "packages" / "web",
        env={
            **os.environ,
            "CARGO_TARGET_DIR": str(target),
            # A tenth of the wasm with no debug info, and faster to run; still
            # with its debug assertions (as `dx serve` builds)
            "CARGO_PROFILE_WASM_DEV_OPT_LEVEL": "1",
            "CARGO_PROFILE_WASM_DEV_DEBUG": "false",
        },
        check=True,
    )
    os.environ["SERVAL_E2E_APP"] = str(
        target / "dx" / "web" / "debug" / "web" / "public"
    )


@pytest.fixture(scope="session")
def app_dir() -> Path:
    return Path(os.environ["SERVAL_E2E_APP"])


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
