from playwright.sync_api import expect

from app import App


def titled(app: App, title: str) -> list[tuple[str, str, str]]:
    return [toast for toast in app.toasts() if toast[1] == title]


def test_opening_and_closing_are_reported(app: App):
    app.open_port()
    app.wait_for_toast("Port opened")
    [(kind, _, description)] = app.toasts()
    assert kind == "success"
    assert description.endswith("at 9600 bps")

    app.page.get_by_role("button", name="Close port").click()
    app.wait_for_toast("Port closed")
    assert app.toasts()[0][0] == "info"


def test_failing_to_open_shows_the_error(app: App):
    app.mock("openError = 'The port is already open.'")
    app.page.locator(".toolbar-port button").first.click()
    app.page.get_by_placeholder("Baudrate (e.g. 9600)").fill("9600")
    app.page.get_by_role("button", name="Open port").click()

    app.wait_for_toast("Failed to open the port")
    assert app.toasts()[0] == (
        "error",
        "Failed to open the port",
        "The port is already open.",
    )
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()


def test_losing_the_device_is_reported(app: App):
    app.open_port()
    app.mock("lose()")
    app.wait_for_toast("Connection lost")
    assert app.toasts()[0][0] == "error"
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()


def test_repeated_send_failures_show_one_toast(app: App):
    app.open_port()
    app.mock("writeError = 'The device did not respond.'")
    for text in ["one", "two", "three"]:
        app.send_text(text)
    app.wait_for_toast("Failed to send")
    app.page.wait_for_timeout(300)
    assert len(titled(app, "Failed to send")) == 1


def test_cancelling_the_port_chooser_shows_nothing(app: App):
    app.mock("cancelRequest = true")
    app.page.locator(".toolbar-port button").first.click()
    app.page.wait_for_timeout(300)
    assert app.toasts() == []
