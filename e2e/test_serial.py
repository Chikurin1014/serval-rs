import re

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


def test_sends_text_to_the_open_port(app: App):
    app.open_port()
    app.send_text("led on")

    app.wait_for_mock("written.length > 0")
    assert app.mock("written") == ["led on"]
    expect(
        app.page.get_by_placeholder("Type text to send to the active port")
    ).to_have_value("")


def test_refresh_lists_the_ports_granted_before(app: App):
    trigger = app.page.locator(".port-selector-trigger")
    expect(trigger).to_have_text("No Devices allowed")

    app.page.locator(".port-selector button").last.click()

    expect(trigger).not_to_have_text("No Devices allowed")
    expect(trigger).not_to_have_text("No Device selected")


def choose_send_format(app: App, name: str):
    trigger = app.page.get_by_role("button", name="Send format")
    trigger.click()
    app.page.locator(".console-format-menu").get_by_role("option", name=name).click()
    expect(trigger).to_have_text(name)


def test_sends_bytes_written_in_hex_bin_and_dec(app: App):
    app.open_port()
    send = app.page.locator(".console-send-button")

    choose_send_format(app, "HEX")
    expect(app.page.locator(".console-send-prefix")).to_have_text("0x")
    field = app.page.get_by_placeholder("A number to send, e.g. 17fff")
    field.fill("17fff")
    field.press("Enter")

    choose_send_format(app, "BIN")
    expect(app.page.locator(".console-send-prefix")).to_have_text("0b")
    app.page.get_by_placeholder("A number to send, e.g. 1111_0000").fill("1_00001010")
    send.click()

    choose_send_format(app, "DEC")
    app.page.get_by_placeholder("A number to send, e.g. 1024").fill("65535")
    send.click()

    app.wait_for_mock("writtenBytes.length === 3")
    assert app.mock("writtenBytes") == [
        [0x01, 0x7F, 0xFF],
        [0x01, 0b00001010],
        [0xFF, 0xFF],
    ]


def test_bytes_it_cannot_read_are_not_sent(app: App):
    app.open_port()
    choose_send_format(app, "DEC")
    field = app.page.get_by_placeholder("A number to send, e.g. 1024")
    field.fill("1.5")
    expect(field).to_have_attribute("aria-invalid", "true")
    expect(field).to_have_attribute("title", re.compile("not a number in base 10"))
    expect(app.page.locator(".console-send-button")).to_be_disabled()
    field.press("Enter")
    assert app.mock("writtenBytes") == []
