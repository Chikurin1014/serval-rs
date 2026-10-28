from playwright.sync_api import expect

from app import App


def test_closes_and_reopens_the_port(app: App):
    app.open_port()
    app.wait_for_console_text()

    # A port does not close while it is being read
    app.page.get_by_role("button", name="Close port").click()
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()
    # What came before it closed may show a moment after: then nothing more
    app.page.wait_for_timeout(300)
    stopped = len(app.console_text())
    app.page.wait_for_timeout(500)
    assert len(app.console_text()) == stopped

    app.page.get_by_role("button", name="Open port").click()
    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()
    app.wait_for_console_text(longer_than=stopped)


def test_keys_are_sent_as_tera_term_sends_them(app: App):
    # None while the port is closed
    app.send_text("x")
    app.page.wait_for_timeout(200)
    assert app.mock("written") == []

    app.open_port()
    app.send_text("led on")
    app.page.keyboard.press("Enter")
    app.wait_for_mock("written.length >= 7")
    # Enter sends CR LF at first
    assert "".join(app.mock("written")) == "led on\r\n"

    # Backspace sends BS and Delete DEL
    app.page.keyboard.press("Backspace")
    app.page.keyboard.press("Delete")
    app.wait_for_mock("written.length >= 9")
    assert app.mock("written")[-2:] == ["\b", "\x7f"]

    # Alt+B sends a break, nothing typed
    app.page.keyboard.press("Alt+B")
    app.wait_for_mock("signals.length === 2")
    assert app.mock("signals") == [{"break": True}, {"break": False}]
    assert len(app.mock("written")) == 9

    # Alt+R sends the clipboard, the line ending for each line break and after
    app.page.context.grant_permissions(["clipboard-read", "clipboard-write"])
    app.page.evaluate("navigator.clipboard.writeText('led on\\nled off')")
    app.page.keyboard.press("Alt+R")
    app.wait_for_mock("written.length >= 10")
    assert app.mock("written")[-1] == "led on\r\nled off\r\n"


def test_enter_sends_the_line_ending_chosen(app: App):
    app.open_port()
    choice = app.page.get_by_role("group", name="Line ending")
    expect(choice.locator("[data-state=on]")).to_have_text(["CR", "LF"])
    # Pressed in turn: CR off, then on again
    for name, ending in [("CR", "\n"), ("CR", "\r\n")]:
        choice.get_by_text(name, exact=True).click()
        before = len(app.mock("written"))
        app.send_text("a")
        app.page.keyboard.press("Enter")
        app.wait_for_mock(f"written.length >= {before + 2}")
        assert app.mock("written")[before:] == ["a", ending]

    # The last pressed stays so
    choice.get_by_text("CR", exact=True).click()
    choice.get_by_text("LF", exact=True).click()
    expect(choice.locator("[data-state=on]")).to_have_text("LF")

    # Ctrl+M still sends a CR alone
    app.page.locator(".console-output").click()
    app.page.keyboard.press("Control+KeyM")
    app.wait_for_mock(f"written.length >= {before + 3}")
    assert app.mock("written")[-1] == "\r"

    # Alt+R: the line ending for each line break, and after
    app.page.context.grant_permissions(["clipboard-read", "clipboard-write"])
    app.page.evaluate("navigator.clipboard.writeText('a\\nb')")
    app.page.keyboard.press("Alt+R")
    app.wait_for_mock(f"written.length >= {before + 4}")
    assert app.mock("written")[-1] == "a\nb\n"


def send_buffer(app: App) -> list[list]:
    return app.page.evaluate(
        """() => [...document.querySelectorAll('.console-send-buffer span')]
            .map(span => [span.textContent, span.dataset.sent === 'true'])"""
    )


def test_send_buffer_marks_what_the_port_has_taken(app: App):
    app.open_port()
    app.mock("holdWrites = true")
    app.send_text("ok")
    app.page.keyboard.press("Enter")
    expect(app.page.locator(".console-send-buffer")).to_have_text("ok\u240d\u240a")
    assert send_buffer(app) == [["o", False], ["k", False], ["\u240d\u240a", False]]
    expect(app.page.locator(".console-send-buffer span[data-sent=true]")).to_have_count(
        0
    )

    app.mock("holdWrites = false")
    app.mock("release()")
    expect(app.page.locator(".console-send-buffer span[data-sent=true]")).to_have_count(
        3
    )
    assert "".join(app.mock("written")) == "ok\r\n"


def test_send_buffer_empties_from_the_left_after_three_seconds(app: App):
    app.open_port()
    app.stop_clock()
    # Its time jumped on, the timers due fired once: not each frame on the way
    app.send_text("a")
    buffer = app.page.locator(".console-send-buffer")
    expect(buffer).to_have_text("a")
    app.page.clock.fast_forward(1000)
    app.send_text("b")
    expect(buffer).to_have_text("ab")

    app.page.clock.fast_forward(1900)
    expect(buffer).to_have_text("ab")
    app.page.clock.fast_forward(200)
    expect(buffer).to_have_text("b")
    app.page.clock.fast_forward(1000)
    expect(buffer).to_have_text("")


def test_send_buffer_keeps_each_send_three_seconds_after_it_is_sent(app: App):
    app.open_port()
    app.stop_clock()
    app.mock("holdWrites = true")
    app.send_text("x")
    buffer = app.page.locator(".console-send-buffer")
    expect(buffer).to_have_text("x")
    app.page.clock.fast_forward(3500)
    expect(buffer).to_have_text("x")

    app.mock("holdWrites = false")
    app.mock("release()")
    expect(buffer.locator("span[data-sent=true]")).to_have_count(1)
    app.page.clock.fast_forward(2900)
    expect(buffer).to_have_text("x")
    app.page.clock.fast_forward(200)
    expect(buffer).to_have_text("")


def test_refresh_lists_the_ports_granted_before(app: App):
    trigger = app.page.locator(".port-selector-trigger")
    expect(trigger).to_have_text("No Devices allowed")

    app.page.locator(".port-selector button").last.click()

    expect(trigger).not_to_have_text("No Devices allowed")
    expect(trigger).not_to_have_text("No Device selected")


def test_send_buffer_shows_hex_in_the_hex_view(app: App):
    app.open_port()
    app.mock("holdWrites = true")
    app.page.get_by_role("tab", name="HEX").click()
    app.send_text("ok")
    app.page.keyboard.press("Enter")
    buffer = app.page.locator(".console-send-buffer")
    expect(buffer).to_have_text("6F 6B 0D 0A")
    expect(buffer.locator("span[data-sent=true]")).to_have_count(0)

    app.mock("holdWrites = false")
    app.mock("release()")
    expect(buffer.locator("span[data-sent=true]")).to_have_count(3)

    app.page.get_by_role("tab", name="Text").click()
    expect(buffer).to_have_text("ok␍␊")


def signal_action(app: App, name: str):
    app.page.get_by_role("button", name="Signal action").click()
    app.page.locator(".port-signal-menu-content").get_by_role(
        "option", name=name, exact=True
    ).click()


def test_signals_are_set_as_each_action_has_them(app: App):
    # Restart (Arduino) the default, of no port yet
    restart = app.page.get_by_role("button", name="Restart (Arduino)")
    expect(restart).to_be_disabled()
    app.open_port()

    # Manual: DTR and RTS toggled, on as the port opens
    signal_action(app, "Manual")
    dtr = app.page.get_by_role("button", name="DTR")
    rts = app.page.get_by_role("button", name="RTS")
    expect(dtr).to_have_attribute("aria-pressed", "true")
    expect(rts).to_have_attribute("aria-pressed", "true")
    dtr.click()
    expect(dtr).to_have_attribute("aria-pressed", "false")
    rts.click()
    expect(rts).to_have_attribute("aria-pressed", "false")
    dtr.click()
    expect(dtr).to_have_attribute("aria-pressed", "true")
    assert app.mock("signals") == [
        {"dataTerminalReady": False},
        {"requestToSend": False},
        {"dataTerminalReady": True},
    ]

    # Arduino's: DTR and RTS off a moment, then on again
    signal_action(app, "Restart (Arduino)")
    restart.click()
    app.wait_for_mock("signals.length === 5")
    assert app.mock("signals")[3:] == [
        {"dataTerminalReady": False, "requestToSend": False},
        {"dataTerminalReady": True, "requestToSend": True},
    ]

    # Espressif's: RTS pulsed with DTR off
    signal_action(app, "Restart (Espressif)")
    app.page.get_by_role("button", name="Restart (Espressif)").click()
    app.wait_for_mock("signals.length === 7")
    assert app.mock("signals")[5:] == [
        {"dataTerminalReady": False, "requestToSend": True},
        {"requestToSend": False},
    ]


FILE = bytes(range(256)) * 40


def pick_file(app: App, contents: bytes = FILE):
    app.page.get_by_label("Send a file").set_input_files(
        files=[
            {
                "name": "data.bin",
                "mimeType": "application/octet-stream",
                "buffer": contents,
            }
        ]
    )


def test_a_file_is_sent_as_it_is_a_part_at_a_time(app: App):
    expect(app.page.get_by_label("Send a file")).to_be_disabled()
    app.open_port()
    pick_file(app)
    app.wait_for_toast("File sent")
    written = app.mock("writtenBytes")
    assert [len(part) for part in written] == [4096, 4096, 2048]
    assert bytes(byte for part in written for byte in part) == FILE
    expect(app.page.locator(".console-send-file")).to_have_text("data.bin (10,240 B)")
    expect(app.page.locator(".console-send-file")).to_have_attribute(
        "data-sent", "true"
    )


def test_sending_a_file_stops_when_asked(app: App):
    app.open_port()
    app.mock("holdWrites = true")
    pick_file(app)
    stop = app.page.get_by_role("button", name="Stop sending the file")
    expect(stop).to_be_visible()
    # Keys go not in between its parts
    app.send_text("x")

    stop.click()
    app.mock("holdWrites = false")
    app.mock("release()")
    app.wait_for_toast("Sending stopped")
    assert ("info", "Sending stopped", "data.bin: 4,096 B of 10,240 B") in app.toasts()
    assert [len(part) for part in app.mock("writtenBytes")] == [4096]
    expect(app.page.locator(".console-send-file")).to_have_text(
        "data.bin (10,240 B) stopped"
    )
    expect(app.page.get_by_label("Send a file")).to_be_enabled()


def test_a_file_shows_its_progress_on_all_the_bar(app: App):
    app.open_port()
    app.mock("holdWrites = true")
    pick_file(app)
    progress = app.page.get_by_role("progressbar", name="Sending data.bin")
    expect(progress).to_have_attribute("aria-valuenow", "0")
    # In place of the send buffer, cut off at its right
    expect(app.page.locator(".console-send-buffer")).to_have_count(0)

    for percent in ["40", "80"]:
        app.mock("release()")
        expect(progress).to_have_attribute("aria-valuenow", percent)
    expect(progress).to_contain_text("data.bin · 8,192 B / 10,240 B (80%)")

    app.mock("holdWrites = false")
    app.mock("release()")
    expect(progress).to_have_count(0)
    expect(app.page.locator(".console-send-file")).to_have_attribute(
        "data-sent", "true"
    )


def drag_file(app: App, events: list[str], contents: bytes = FILE):
    """Drags a file over the console, as from the desktop, by these events."""
    transfer = app.page.evaluate_handle(
        """bytes => {
            const transfer = new DataTransfer();
            transfer.items.add(new File([new Uint8Array(bytes)], 'data.bin'));
            return transfer;
        }""",
        list(contents),
    )
    for event in events:
        app.page.dispatch_event(".console-output", event, {"dataTransfer": transfer})


def test_a_file_dropped_on_the_console_is_sent_while_the_port_is_open(app: App):
    console = app.page.locator(".console")
    # Not while it is closed
    drag_file(app, ["dragenter", "dragover"])
    expect(console).to_have_attribute("data-dropping", "false")
    drag_file(app, ["drop"])
    app.page.wait_for_timeout(300)
    assert app.mock("writtenBytes") == []
    expect(app.page.locator(".console-send-file")).to_have_count(0)

    app.open_port()
    drag_file(app, ["dragenter", "dragover"])
    expect(console).to_have_attribute("data-dropping", "true")
    drag_file(app, ["drop"])
    expect(console).to_have_attribute("data-dropping", "false")
    app.wait_for_mock(f"writtenBytes.flat().length === {len(FILE)}")
    assert bytes(sum(app.mock("writtenBytes"), [])) == FILE
    expect(app.page.locator(".console-send-file")).to_have_text("data.bin (10,240 B)")
