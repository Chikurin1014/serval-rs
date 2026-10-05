import re

from app import App

# What the mock sends, chunk after chunk, with nothing lost or repeated
WHOLE_STREAM = re.compile(r"(temp:[\d.]+\nvolt:[\d.]+\n)*")


def console_text(app: App) -> str:
    return app.page.locator(".console-output").text_content()


def assert_whole_stream(text: str):
    assert WHOLE_STREAM.fullmatch(text)
    # Readings change every chunk, so a repeat means a chunk was shown twice
    chunks = re.findall(r"temp:[\d.]+\nvolt:[\d.]+\n", text)
    assert all(a != b for a, b in zip(chunks, chunks[1:]))


def test_shows_the_received_text_as_it_arrives(app: App):
    app.open_port()
    app.page.wait_for_function(
        "() => document.querySelector('.console-output').textContent.length > 0"
    )
    first = console_text(app)
    app.page.wait_for_timeout(500)
    later = console_text(app)

    assert later.startswith(first)
    assert len(later) > len(first)
    assert_whole_stream(later)


def test_starts_over_after_clearing_all_data(app: App):
    app.open_port()
    app.page.wait_for_timeout(1000)
    before = console_text(app)

    app.tab("Data")
    app.page.locator(".data-grid").get_by_role("button", name="Clear all").click()
    app.tab("Console")
    app.page.wait_for_timeout(200)
    after = console_text(app)

    assert len(after) < len(before)
    assert_whole_stream(after)


def test_keeps_up_past_the_data_limit(app: App):
    """Past `MAX_ENTRIES_PER_LABEL`, the oldest raw data is dropped: the
    console must carry on appending, not mistake that for a clear."""
    app.open_port()
    app.page.evaluate("window.mockSerialPort.burst(12000)")
    app.page.wait_for_function(
        "() => document.querySelector('.console-output').textContent.split('\\n').length > 24000",
        timeout=30000,
    )
    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").wait_for()

    text = console_text(app)
    assert WHOLE_STREAM.fullmatch(text)
    chunks = text.count("temp:")
    assert chunks == app.page.evaluate("window.mockSerialPort.sent")
