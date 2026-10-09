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


def test_keeps_its_text_after_clearing_all_data(app: App):
    """It shows the port's history, not the data."""
    app.open_port()
    app.page.wait_for_timeout(1000)
    before = app.console_text()

    app.tab("Data")
    app.clear_all()
    app.tab("Console")
    app.wait_for_console_text(longer_than=len(before))
    after = app.console_text()

    assert after.startswith(before)
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


# Each row in view: its text, whether a long line wraps onto it, and the time
# beside it
VIEW_ROWS = """() => {
    const term = document.querySelector('.console-output').xterm;
    const buffer = term.buffer.active;
    const times = [...document.querySelectorAll('#console-times div')];
    return times.map((cell, row) => {
        const line = buffer.getLine(buffer.viewportY + row);
        return [line?.translateToString(true) ?? '', line?.isWrapped ?? false, cell.textContent];
    });
}"""

TIME = re.compile(r"\d{2}:\d{2}:\d{2}\.\d{3}")


def seconds(time: str) -> float:
    hours, minutes, rest = time.split(":")
    return int(hours) * 3600 + int(minutes) * 60 + float(rest)


def view_rows(app: App):
    app.page.wait_for_timeout(200)
    return app.page.evaluate(VIEW_ROWS)


def test_times_show_beside_the_lines_with_text(app: App):
    app.open_port()
    app.mock("mute()")
    app.mock("receive('a long line ' + 'x'.repeat(300) + '\\n\\nok\\n')")
    rows = view_rows(app)
    shown = [time for text, wrapped, time in rows if text and not wrapped]
    assert shown and all(TIME.fullmatch(time) for time in shown)
    # None beside blank rows, nor those a long line wraps onto
    assert all(time == "" for text, wrapped, time in rows if not text or wrapped)
    assert any(wrapped for _, wrapped, _ in rows)
    times = [seconds(time) for time in shown]
    assert times == sorted(times)


def test_a_line_shows_when_its_first_character_came(app: App):
    app.open_port()
    app.mock("mute()")
    app.mock("receive('\\nfirst')")
    app.page.wait_for_timeout(1200)
    app.mock("receive(' half\\nnext\\n')")
    times = {
        text: time for text, _, time in view_rows(app) if text in ("first half", "next")
    }
    assert seconds(times["next"]) - seconds(times["first half"]) >= 1


def test_times_stay_as_the_console_renders_again(app: App):
    app.open_port()
    app.mock("mute()")
    app.mock("receive('\\none\\ntwo\\n')")
    before = [row for row in view_rows(app) if row[0] in ("one", "two")]
    app.tab("Data")
    app.tab("Console")
    app.wait_for_console_text()
    after = [row for row in view_rows(app) if row[0] in ("one", "two")]
    assert before == after and len(after) == 2


def test_times_cannot_be_touched(app: App):
    style = app.page.evaluate(
        "() => { const s = getComputedStyle(document.getElementById('console-times'));"
        " return [s.pointerEvents, s.userSelect]; }"
    )
    assert style == ["none", "none"]


def test_times_stay_beside_their_lines_past_the_scrollback(app: App):
    app.open_port()
    app.mock("burst(6000)")
    wait_for_full_scrollback(app)
    app.mock("receive('last\\n')")

    def all_beside():
        rows = view_rows(app)
        return all(
            bool(TIME.fullmatch(time)) == bool(text and not wrapped)
            for text, wrapped, time in rows
        )

    assert all_beside()
    assert "last" in [text for text, _, _ in view_rows(app)]
    # The oldest lines kept, after thousands trimmed off before them
    app.page.evaluate("document.querySelector('.console-output').xterm.scrollToTop()")
    assert all_beside()
