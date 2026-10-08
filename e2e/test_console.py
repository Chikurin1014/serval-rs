import re

from app import App

WHOLE_STREAM = re.compile(r"(temp:[\d.]+\nvolt:[\d.]+\n)*")


def assert_whole_stream(text: str):
    assert WHOLE_STREAM.fullmatch(text)
    chunks = re.findall(r"temp:[\d.]+\nvolt:[\d.]+\n", text)
    assert all(a != b for a, b in zip(chunks, chunks[1:]))


def test_shows_the_received_text_as_it_arrives(app: App):
    app.open_port()
    app.wait_for_console_text()
    first = app.console_text()
    app.page.wait_for_timeout(500)
    later = app.console_text()

    assert later.startswith(first)
    assert len(later) > len(first)
    assert_whole_stream(later)


def test_starts_over_after_clearing_all_data(app: App):
    app.open_port()
    app.page.wait_for_timeout(1000)
    before = app.console_text()

    app.tab("Data")
    app.clear_all()
    app.tab("Console")
    app.page.wait_for_timeout(200)
    after = app.console_text()

    assert len(after) < len(before)
    assert_whole_stream(after)


def test_keeps_its_text_when_it_renders_again(app: App):
    """Rendering again (typing in the send field) does not start the output over."""
    app.open_port()
    app.mock("burst(12000)")
    app.wait_for_console_lines(more_than=24000)
    start = app.console_text()[:100]
    app.page.get_by_placeholder("Type text to send to the active port").fill("led on")
    app.page.wait_for_timeout(500)
    text = app.console_text()
    assert text.startswith(start)
    assert_whole_stream(text)


def test_keeps_up_past_the_data_limit(app: App):
    """Dropping the oldest raw data is not taken for a clear."""
    app.open_port()
    app.mock("burst(12000)")
    app.wait_for_console_lines(more_than=24000)
    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").wait_for()

    text = app.console_text()
    assert WHOLE_STREAM.fullmatch(text)
    chunks = text.count("temp:")
    assert chunks == app.mock("sent")


def test_stops_scrolling_while_the_user_reads_back(app: App):
    output = app.page.locator(".console-output")
    app.open_port()
    app.mock("burst(200)")
    app.page.wait_for_function(
        "() => { const o = document.querySelector('.console-output');"
        " return o.scrollTop > 0 && o.scrollHeight - o.scrollTop - o.clientHeight < 16; }"
    )

    output.evaluate("o => { o.scrollTop = 0; }")
    app.page.wait_for_timeout(500)
    assert output.evaluate("o => o.scrollTop") == 0

    output.evaluate("o => { o.scrollTop = o.scrollHeight; }")
    before = output.evaluate("o => o.scrollTop")
    app.page.wait_for_function(
        f"() => document.querySelector('.console-output').scrollTop > {before}"
    )
