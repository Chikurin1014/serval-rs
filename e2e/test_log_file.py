import re

from playwright.sync_api import expect

from app import App

# A file picker whose file is `window.savedLog`, as it was last closed; an
# existing one is named `window.savedName`
FAKE_PICKER = """() => {
    window.savedLog = null;
    window.savedName = 'old.log';
    window.pickerCancels = false;
    window.writesFail = false;
    const handle = {
        get name() { return window.savedName; },
        getFile: async () => {
            const bytes = new TextEncoder().encode(window.savedLog ?? '');
            return { size: bytes.length, slice: start => ({ arrayBuffer: async () => bytes.slice(start).buffer }) };
        },
        createWritable: async ({ keepExistingData }) => {
            let pending = keepExistingData ? (window.savedLog ?? '') : '';
            return {
                seek: async () => {},
                write: async bytes => {
                    if (window.writesFail) throw new DOMException('disk full', 'QuotaExceededError');
                    pending += new TextDecoder().decode(bytes);
                },
                close: async () => { window.savedLog = pending; },
            };
        },
    };
    window.showSaveFilePicker = async options => {
        window.pickedName = options.suggestedName;
        if (window.pickerCancels) throw new DOMException('aborted', 'AbortError');
        return handle;
    };
    window.showOpenFilePicker = async () => [handle];
}"""


def log_item(app: App, kind: str):
    return (
        app.page.locator(".log-menu-content")
        .get_by_role("option")
        .filter(
            has=app.page.locator(".log-item-name", has_text=re.compile(f"^{kind}$"))
        )
    )


def start_log(app: App, kind: str, variant: str | None = None):
    """Picks `kind` from the REC menu, or its `variant` beside it."""
    app.page.get_by_role("button", name="Start logging").click()
    item = log_item(app, kind)
    if variant is None:
        item.click()
    else:
        item.hover()
        item.get_by_role("menuitem", name=variant, exact=True).click()


def test_rec_logs_to_a_file_until_stop(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.open_port()
    app.mock("mute()")

    start_log(app, "Text file")
    stop = app.page.get_by_role("button", name="Stop logging")
    expect(stop).to_have_text("STOP")
    assert app.page.evaluate("window.pickedName").endswith(".log")

    app.mock("receive('led: on\\r\\nok')")
    app.page.wait_for_timeout(200)
    stop.click()
    expect(app.page.get_by_role("button", name="Start logging")).to_have_text("REC")
    app.page.wait_for_function("window.savedLog !== null")
    # The unfinished line too, as it stops
    assert app.page.evaluate("window.savedLog").endswith("led: on\nok\n")


def test_sent_lines_are_marked_when_asked(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.open_port()
    app.mock("mute()")

    start_log(app, "Text file", "with time and sent")
    expect(app.page.get_by_role("button", name="Stop logging")).to_be_visible()
    app.send_text("ls")
    app.page.keyboard.press("Enter")
    app.wait_for_mock("written.length >= 3")
    app.page.get_by_role("button", name="Stop logging").click()
    app.page.wait_for_function("window.savedLog !== null")
    assert any(
        line.endswith("] > ls")
        for line in app.page.evaluate("window.savedLog").splitlines()
    )


def test_cancelling_the_picker_starts_no_log(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.page.evaluate("window.pickerCancels = true")
    start_log(app, "Raw bytes")
    app.page.wait_for_timeout(300)
    expect(app.page.get_by_role("button", name="Start logging")).to_be_visible()
    assert app.page.evaluate("window.pickedName").endswith(".bin")


def test_items_show_their_icons_and_variants(app: App):
    app.page.get_by_role("button", name="Start logging").click()
    icons = []
    for item_name, variants in [
        ("Text file", ["with time", "with time and sent"]),
        ("Hex file", ["with time", "with time and sent"]),
        ("Raw bytes", []),
        ("Continue with existing file", []),
    ]:
        item = log_item(app, item_name)
        icon = item.locator("svg.log-item-icon")
        expect(icon).to_have_count(1)
        icons.append(icon.inner_html())
        item.hover()
        expect(item.get_by_role("menuitem")).to_have_text(variants)
    # Type, Hexagon, Binary and File
    assert len(set(icons)) == 4


def test_a_variant_is_picked_by_keyboard(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.page.get_by_role("button", name="Start logging").click()
    log_item(app, "Hex file").focus()
    app.page.keyboard.press("ArrowRight")
    expect(
        log_item(app, "Hex file").get_by_role("menuitem", name="with time", exact=True)
    ).to_be_focused()
    app.page.keyboard.press("Enter")
    expect(app.page.get_by_role("button", name="Stop logging")).to_be_visible()
    assert app.page.evaluate("window.pickedName").endswith(".tsv")


def test_an_existing_file_goes_on_in_its_format(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.page.evaluate("window.savedLog = '[1791409892.542] old\\n'")
    app.open_port()
    app.mock("mute()")

    start_log(app, "Continue with existing file")
    expect(app.page.get_by_role("button", name="Stop logging")).to_be_visible()
    app.wait_for_toast("Logging continued")
    assert ("info", "Logging continued", "old.log (Text with time)") in app.toasts()

    app.mock("receive('new\\n')")
    app.page.wait_for_timeout(200)
    app.page.get_by_role("button", name="Stop logging").click()
    app.page.wait_for_function("window.savedLog.split('\\n').length > 2")
    old, new = app.page.evaluate("window.savedLog").splitlines()
    assert old == "[1791409892.542] old"
    assert re.fullmatch(r"\[\d+\.\d{3}\] new", new)


def test_a_failed_write_stops_the_log_at_once(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.open_port()
    start_log(app, "Text file")
    expect(app.page.get_by_role("button", name="Stop logging")).to_be_visible()

    app.page.evaluate("window.writesFail = true")
    app.wait_for_toast("Logging stopped")
    expect(app.page.get_by_role("button", name="Start logging")).to_be_visible()
    assert (
        "error",
        "Logging stopped",
        "Failed to write the log: disk full",
    ) in app.toasts()


LEAVING = """() => {
    const event = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(event);
    return event.defaultPrevented;
}"""


def test_leaving_the_page_asks_while_logging(app: App):
    app.page.evaluate(FAKE_PICKER)
    assert app.page.evaluate(LEAVING) is False

    start_log(app, "Text file")
    stop = app.page.get_by_role("button", name="Stop logging")
    expect(stop).to_be_visible()
    # On whichever tab is shown
    app.tab("Data")
    app.page.wait_for_function(LEAVING)

    app.tab("Console")
    stop.click()
    app.page.wait_for_function(f"() => !({LEAVING})()")


def test_text_tells_the_port_opening_and_closing(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.open_port()
    app.mock("mute()")
    start_log(app, "Text file", "with time")
    expect(app.page.get_by_role("button", name="Stop logging")).to_be_visible()

    # The last line cut by the close: written before it
    app.mock("receive('ok\\nhal')")
    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").click()
    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()
    app.mock("lose()")
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()
    app.page.get_by_role("button", name="Stop logging").click()

    app.page.wait_for_function("window.savedLog !== null")
    lines = [
        re.sub(r"^\[\d+\.\d{3}\] ", "", line)
        for line in app.page.evaluate("window.savedLog").splitlines()
    ]
    events = [line for line in lines if line.startswith("---")]
    assert lines[:2] == ["ok", "hal"]
    assert events[0] == lines[2]
    assert re.fullmatch(r"--- Port closed: .+ ---", events[0])
    assert re.fullmatch(r"--- Port opened: .+ at 9600 bps ---", events[1])
    assert re.fullmatch(r"--- Connection lost: .+ ---", events[2])


def test_hex_tells_no_events(app: App):
    app.page.evaluate(FAKE_PICKER)
    app.open_port()
    app.mock("mute()")
    start_log(app, "Hex file")
    expect(app.page.get_by_role("button", name="Stop logging")).to_be_visible()
    app.mock("receive('ok')")
    app.page.get_by_role("button", name="Close port").click()
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()
    app.page.get_by_role("button", name="Stop logging").click()
    app.page.wait_for_function("window.savedLog !== null")
    assert app.page.evaluate("window.savedLog") == "6F 6B\n"
