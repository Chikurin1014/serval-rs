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
    assert "".join(app.mock("written")) == "led on\r"


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
