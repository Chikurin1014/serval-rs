from playwright.sync_api import expect

from app import App


def titled(app: App, title: str) -> list[tuple[str, str, str]]:
    return [toast for toast in app.toasts() if toast[1] == title]


def test_each_turn_of_the_port_is_reported(app: App):
    # The chooser cancelled: nothing to tell
    app.mock("cancelRequest = true")
    app.page.locator(".toolbar-port button").first.click()
    app.page.wait_for_timeout(300)
    assert app.toasts() == []
    app.mock("cancelRequest = false")

    app.mock("openError = 'The port is already open.'")
    app.page.locator(".toolbar-port button").first.click()
    app.page.get_by_placeholder("Baud rate (e.g. 9600)").fill("9600")
    app.page.get_by_role("button", name="Open port").click()
    app.wait_for_toast("Failed to open the port")
    assert app.toasts()[0] == (
        "error",
        "Failed to open the port",
        "The port is already open.",
    )
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()

    app.mock("openError = null")
    app.page.get_by_role("button", name="Open port").click()
    app.wait_for_toast("Port opened")
    kind, _, description = app.toasts()[0]
    assert kind == "success"
    assert description.endswith("at 9600 bps")

    app.page.get_by_role("button", name="Close port").click()
    app.wait_for_toast("Port closed")
    assert app.toasts()[0][0] == "info"

    # Each its own toast: opened once more
    app.page.get_by_role("button", name="Open port").click()
    expect(
        app.page.locator("[role=alertdialog]", has_text="Port opened")
    ).to_have_count(2)

    app.mock("lose()")
    app.wait_for_toast("Connection lost")
    assert app.toasts()[0][0] == "error"
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()
    assert [title for _, title, _ in app.toasts()] == [
        "Connection lost",
        "Port opened",
        "Port closed",
        "Port opened",
        "Failed to open the port",
    ]


def test_each_send_failure_shows_its_toast(app: App):
    app.open_port()
    app.mock("writeError = 'The device did not respond.'")
    # A key each
    app.send_text("abc")
    expect(
        app.page.locator("[role=alertdialog]", has_text="Failed to send")
    ).to_have_count(3)
    assert titled(app, "Failed to send")[0] == (
        "error",
        "Failed to send",
        "The device did not respond.",
    )
