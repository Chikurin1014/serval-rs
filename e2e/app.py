"""Operations on the app shared by the tests."""

import re

from playwright.sync_api import ConsoleMessage, Page, expect

# {label: [type, latest, time]} of the data list in view
_DATA_ROWS = """() => {
    const table = [...document.querySelectorAll('.data-table')].find(t => t.offsetParent);
    if (!table) return {};
    return Object.fromEntries([...table.querySelectorAll('tbody tr.data-row')].map(row => {
        // The latest by its title: a log's level is a tag, there in words before it
        const [label, ...cells] = [...row.children].slice(0, 4)
            .map((c, i) => i === 2 ? c.title : c.textContent);
        return [label, cells];
    }));
}"""

_GRAPH_LEGENDS = """() => [...document.querySelectorAll('.graph-grid .graph')].map(graph =>
    [...graph.querySelectorAll('.u-legend .u-series th')].map(th => th.textContent.trim()).slice(1))"""

# The lines of the console's xterm.js terminal up to the cursor's, joined
_CONSOLE_TEXT = """() => {
    const term = document.querySelector('.console-output')?.xterm;
    if (!term) return '';
    const buffer = term.buffer.active;
    let text = '';
    for (let i = 0; i <= buffer.baseY + buffer.cursorY; i++) {
        const line = buffer.getLine(i);
        if (i > 0 && !line.isWrapped) text += '\\n';
        text += line.translateToString(true);
    }
    return text;
}"""

NAME_VALUE = r"(\w+):([\d.]+)"


class App:
    def __init__(self, page: Page):
        self.page = page
        self.console: list[ConsoleMessage] = []
        self.errors: list[str] = []
        page.on("console", lambda message: self.console.append(message))
        page.on("pageerror", lambda error: self.errors.append(str(error)))

    def open_port(self):
        page = self.page
        page.locator(".toolbar-port button").first.click()
        page.get_by_placeholder("Baudrate (e.g. 9600)").fill("9600")
        page.get_by_role("button", name="Open port").click()
        expect(page.get_by_role("button", name="Close port")).to_be_visible()

    def tab(self, name: str):
        self.page.get_by_role("tab", name=name).click()

    def mock(self, expression: str):
        """Evaluates `expression` on the mock port, e.g. `written`."""
        return self.page.evaluate(f"window.mockSerialPort.{expression}")

    def wait_for_mock(self, condition: str):
        self.page.wait_for_function(f"() => window.mockSerialPort.{condition}")

    def stop_clock(self):
        """Puts the page's timers (from now on) on a clock of the test's own,
        stopped until `page.clock.run_for` moves it on."""
        self.page.clock.install()
        # Else it goes on as the real one
        self.page.clock.pause_at(self.page.evaluate("Date.now()") + 1)

    def run_clock_until(self, done, step_ms: int = 50):
        """Moves the stopped clock on, `step_ms` at a time, until `done()`: as
        what the page does a while after (e.g. on its next frame) waits on it."""
        for _ in range(100):
            if done():
                return
            self.page.clock.run_for(step_ms)
        assert done()

    def send_text(self, text: str):
        """Types `text` in the console, which sends each key as it is."""
        self.page.locator(".console-output").click()
        self.page.keyboard.type(text)

    def console_text(self) -> str:
        """The console's lines, up to the cursor's."""
        return self.page.evaluate(f"() => ({_CONSOLE_TEXT})()")

    def wait_for_console_text(self, longer_than: int = 0):
        self.page.wait_for_function(
            f"length => ({_CONSOLE_TEXT})().length > length", arg=longer_than
        )

    def clear_all(self, within: str = ".data-grid"):
        """Clears all data with the data list's button in `within`."""
        self.page.locator(within).get_by_role("button", name="Clear all").click()

    def add_map(self, name: str, source: str, output: str):
        """Adds the map `name` from `source` to `output` type, from the Data tab."""
        bar = self.page.locator(".add-map-bar")
        bar.get_by_role("button", name=output, exact=True).hover()
        bar.get_by_role("option").filter(
            has=self.page.locator(".add-map-title", has_text=re.compile(f"^{name}$"))
        ).filter(
            has=self.page.locator(".map-types", has_text=f"{source}{output}")
        ).click()

    def add_match_map(
        self,
        output: str = "Number",
        source: str = "log",
        pattern: str = NAME_VALUE,
        target: str = "$1",
        replacement: str = "$2",
    ):
        self.add_map("Match", "Log", output)
        card = self.map_cards().last
        card.get_by_placeholder("Input label").fill(source)
        card.get_by_placeholder("Text to be matched").fill(pattern)
        card.get_by_placeholder("Output label").fill(target)
        # A log's value is its message (of no level, left empty)
        card.get_by_label("Message" if output == "Log" else "To", exact=True).fill(
            replacement
        )
        card.get_by_role("switch").click()

    def map_cards(self):
        return self.page.locator(".map-list [data-slot=card]")

    def map_item(self, output: str, title: str = "Match"):
        """The item `title` of the add menu to `output`, the menu opened."""
        trigger = self.page.get_by_role("button", name=output, exact=True)
        menu = self.page.locator(".add-map-bar .hover-menu").filter(has=trigger)
        trigger.hover()
        return menu.get_by_role("option").filter(
            has=self.page.locator(".add-map-title", has_text=re.compile(f"^{title}$"))
        )

    def add_vector_match_map(
        self, variant: str, pattern: str, target: str, components: list[str]
    ):
        """Adds a Match map to a vector, its `variant` (2D or 3D) picked beside it."""
        item = self.map_item("Vector", "Match")
        item.hover()
        item.get_by_role("menuitem").filter(
            has=self.page.locator(
                ".add-map-preset-name", has_text=re.compile(f"^{variant}$")
            )
        ).click()
        card = self.map_cards().nth(self.map_cards().count() - 1)
        card.get_by_placeholder("Input label").fill("log")
        card.get_by_placeholder("Text to be matched").fill(pattern)
        card.get_by_placeholder("Output label").fill(target)
        for name, component in zip("xyz", components):
            card.get_by_label(name, exact=True).fill(component)
        card.get_by_role("switch").click()
        return card

    def add_log_match(
        self,
        pattern: str,
        level: str,
        message: str,
        target: str = "log",
        source: str = "log",
    ):
        """Adds a Match map of logs to logs, turned on."""
        self.map_item("Log").click()
        card = self.map_cards().nth(self.map_cards().count() - 1)
        card.get_by_placeholder("Input label").fill(source)
        card.get_by_placeholder("Text to be matched").fill(pattern)
        card.get_by_placeholder("Output label").fill(target)
        card.get_by_label("Level", exact=True).fill(level)
        card.get_by_label("Message", exact=True).fill(message)
        card.get_by_role("switch").click()
        return card

    def data_rows(self) -> dict[str, list[str]]:
        return self.page.evaluate(_DATA_ROWS)

    def wait_for_labels(self, *labels: str):
        self.page.wait_for_function(
            f"labels => {{ const rows = ({_DATA_ROWS})(); return labels.every(l => l in rows); }}",
            arg=list(labels),
        )

    def graphs(self):
        return self.page.locator(".graph-grid .graph")

    def graph_legends(self) -> list[list[str]]:
        return self.page.evaluate(_GRAPH_LEGENDS)

    def wait_for_graph_legends(self, legends: list[list[str]]):
        self.page.wait_for_function(
            f"expected => JSON.stringify(({_GRAPH_LEGENDS})()) === JSON.stringify(expected)",
            arg=legends,
        )

    def add_graph(self, preset: str = "Linear"):
        bar = self.page.locator(".add-graph-bar")
        bar.get_by_role("button", name="Time series").hover()
        bar.get_by_role("option", name=preset, exact=True).click()

    def add_scatter(self):
        bar = self.page.locator(".add-graph-bar")
        bar.get_by_role("button", name="Statistics").hover()
        bar.get_by_role("option", name="Scatter", exact=True).click()
        return self.graphs().last

    def set_up_graph(self, index: int, title: str):
        graph = self.graphs().nth(index)
        graph.locator(".graph-title").click()  # opens the settings
        graph.locator(".graph-settings-body input").fill(title)
        graph.locator(".graph-title").click()  # closes them

    def toasts(self) -> list[tuple[str, str, str]]:
        """Newest first, as `(type, title, description)`."""
        toasts = self.page.evaluate(
            """() => [...document.querySelectorAll('[role=alertdialog]')].map(toast => {
                const [title, description] = toast.querySelector('[role=alert]').children;
                return [toast.dataset.type, title.textContent, description?.textContent ?? ''];
            })"""
        )
        return [tuple(toast) for toast in toasts]

    def wait_for_toast(self, title: str):
        self.page.wait_for_function(
            """title => document.querySelector('[role=alertdialog] [role=alert]')
                ?.firstElementChild.textContent === title""",
            arg=title,
        )
