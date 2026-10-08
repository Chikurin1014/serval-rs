import re

from app import App

CHUNK = r"temp:[\d.]+\nvolt:[\d.]+\n"
WHOLE_STREAM = re.compile(f"({CHUNK})*")


def assert_whole_stream(text: str):
    assert WHOLE_STREAM.fullmatch(text)
    chunks = re.findall(CHUNK, text)
    assert all(a != b for a, b in zip(chunks, chunks[1:]))


def wait_for_full_scrollback(app: App):
    app.page.wait_for_function(
        """() => {
            const term = document.querySelector('.console-output').xterm;
            return term.buffer.active.length >= term.options.scrollback + term.rows;
        }""",
        timeout=30000,
    )


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
    """Rendering again (the port closed and opened) does not reset the terminal."""
    app.open_port()
    app.wait_for_console_text()
    app.page.evaluate(
        """() => {
            const term = document.querySelector('.console-output').xterm;
            const reset = term.reset.bind(term);
            window.consoleResets = 0;
            term.reset = () => { window.consoleResets++; reset(); };
        }"""
    )
    start = app.console_text()[:40]

    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").click()
    app.send_text("led on")
    app.page.wait_for_timeout(500)

    assert app.page.evaluate("window.consoleResets") == 0
    assert app.console_text().startswith(start)


def test_keeps_the_newest_lines_past_its_scrollback(app: App):
    app.open_port()
    app.mock("burst(12000)")
    wait_for_full_scrollback(app)
    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").wait_for()
    app.page.wait_for_timeout(500)

    text = app.console_text()
    # The oldest lines dropped, the first kept may be either reading
    assert_whole_stream(re.sub(r"^volt:[\d.]+\n", "", text))
    lines = app.page.evaluate(
        """() => {
            const term = document.querySelector('.console-output').xterm;
            return [term.buffer.active.length, term.options.scrollback, term.rows];
        }"""
    )
    assert lines[0] == lines[1] + lines[2]


def test_stops_scrolling_while_the_user_reads_back(app: App):
    viewport = """() => {
        const buffer = document.querySelector('.console-output').xterm.buffer.active;
        return [buffer.viewportY, buffer.baseY];
    }"""
    app.open_port()
    app.mock("burst(200)")
    app.page.wait_for_function(
        f"() => {{ const [y, base] = ({viewport})(); return base > 0 && y === base; }}"
    )

    term = "document.querySelector('.console-output').xterm"
    app.page.evaluate(f"() => {term}.scrollLines(-20)")
    held, _ = app.page.evaluate(viewport)
    app.page.wait_for_timeout(500)
    y, base = app.page.evaluate(viewport)
    assert y == held < base

    app.page.evaluate(f"() => {term}.scrollToBottom()")
    app.page.wait_for_function(
        f"base => {{ const [y, now] = ({viewport})(); return now > base && y === now; }}",
        arg=base,
    )
