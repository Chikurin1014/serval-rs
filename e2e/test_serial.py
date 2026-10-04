from playwright.sync_api import expect

from app import App


def console_length(app: App) -> int:
    """How long the console text is, which grows with each chunk received."""
    return len(app.page.locator(".console-output").text_content())


def test_closes_and_reopens_the_port(app: App):
    app.open_port()
    app.page.wait_for_function(
        "() => document.querySelector('.console-output').textContent.length > 0"
    )

    # A port does not close while it is being read, so this checks that
    # reading stops first
    app.page.get_by_role("button", name="Close port").click()
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()
    stopped = console_length(app)
    app.page.wait_for_timeout(300)
    assert console_length(app) == stopped

    app.page.get_by_role("button", name="Open port").click()
    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()
    app.page.wait_for_function(
        f"() => document.querySelector('.console-output').textContent.length > {stopped}"
    )


def test_sends_text_to_the_open_port(app: App):
    app.open_port()
    field = app.page.get_by_placeholder("Type text to send to the active port")
    field.fill("led on")
    field.press("Enter")

    app.page.wait_for_function("() => window.mockSerialPort.written.length > 0")
    assert app.page.evaluate("window.mockSerialPort.written") == ["led on"]
    expect(field).to_have_value("")


def test_refresh_lists_the_ports_granted_before(app: App):
    trigger = app.page.locator(".port-selector-trigger")
    expect(trigger).to_have_text("No Devices available")

    app.page.locator(".port-selector button").last.click()

    expect(trigger).not_to_have_text("No Devices available")
    expect(trigger).not_to_have_text("No Device selected")
