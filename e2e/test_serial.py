from playwright.sync_api import expect

from app import App


def test_closes_and_reopens_the_port(app: App):
    app.open_port()
    app.wait_for_console_text()

    # A port does not close while it is being read
    app.page.get_by_role("button", name="Close port").click()
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()
    stopped = len(app.console_text())
    app.page.wait_for_timeout(300)
    assert len(app.console_text()) == stopped

    app.page.get_by_role("button", name="Open port").click()
    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()
    app.wait_for_console_text(longer_than=stopped)


def test_sends_each_key_typed_in_the_console(app: App):
    app.open_port()
    app.send_text("led on")
    app.page.keyboard.press("Enter")

    app.wait_for_mock("written.length >= 7")
    # As Tera Term: Enter sends CR
    assert "".join(app.mock("written")) == "led on\r"


def test_backspace_and_delete_send_bs_and_del(app: App):
    app.open_port()
    app.page.locator(".console-output").click()
    app.page.keyboard.press("Backspace")
    app.page.keyboard.press("Delete")

    app.wait_for_mock("written.length >= 2")
    assert app.mock("written") == ["\b", "\x7f"]


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
    expect(app.page.locator(".console-send-buffer")).to_have_text("ok\u240d")
    assert send_buffer(app) == [["o", False], ["k", False], ["\u240d", False]]
    expect(app.page.locator(".console-send-buffer span[data-sent=true]")).to_have_count(
        0
    )

    app.mock("holdWrites = false")
    app.mock("release()")
    expect(app.page.locator(".console-send-buffer span[data-sent=true]")).to_have_count(
        3
    )
    assert "".join(app.mock("written")) == "ok\r"


def test_send_buffer_empties_from_the_left_after_three_seconds(app: App):
    app.open_port()
    app.send_text("a")
    app.page.wait_for_timeout(1000)
    app.send_text("b")
    buffer = app.page.locator(".console-send-buffer")
    expect(buffer).to_have_text("ab")

    expect(buffer).to_have_text("b", timeout=3000)
    expect(buffer).to_have_text("", timeout=2000)


def test_send_buffer_keeps_each_send_three_seconds_after_it_is_sent(app: App):
    app.open_port()
    app.mock("holdWrites = true")
    app.send_text("x")
    app.page.wait_for_timeout(3500)
    buffer = app.page.locator(".console-send-buffer")
    expect(buffer).to_have_text("x")

    app.mock("holdWrites = false")
    app.mock("release()")
    expect(buffer.locator("span[data-sent=true]")).to_have_count(1)
    app.page.wait_for_timeout(2000)
    expect(buffer).to_have_text("x")
    expect(buffer).to_have_text("", timeout=2000)


def test_keys_are_not_sent_while_the_port_is_closed(app: App):
    app.send_text("x")
    app.page.wait_for_timeout(200)
    assert app.mock("written") == []


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
    expect(buffer).to_have_text("6F 6B 0D")
    expect(buffer.locator("span[data-sent=true]")).to_have_count(0)

    app.mock("holdWrites = false")
    app.mock("release()")
    expect(buffer.locator("span[data-sent=true]")).to_have_count(3)

    app.page.get_by_role("tab", name="Text").click()
    expect(buffer).to_have_text("ok␍")
