from playwright.sync_api import expect

from app import App


def details(app: App) -> dict[str, str]:
    return app.page.evaluate(
        """() => Object.fromEntries([...document.querySelectorAll('.port-info dt')]
            .map(dt => [dt.textContent, dt.nextElementSibling.textContent]))"""
    )


def notifications(app: App) -> list[tuple[str, str]]:
    """Newest first, as `(kind, title)`."""
    return [
        tuple(entry)
        for entry in app.page.evaluate(
            """() => [...document.querySelectorAll('.port-notifications li')]
                .map(li => [li.dataset.kind, li.querySelector('.port-notification-title').textContent])"""
        )
    ]


def test_shows_the_device_its_settings_and_its_bytes(app: App):
    app.open_port()
    shown = details(app)
    assert shown["Vendor"] == "Arduino SA"
    assert shown["Product"] == "Uno R3 (CDC ACM)"
    assert shown["Baud rate"] == "9600 bps"
    assert (shown["Data bits"], shown["Parity"], shown["Stop bits"]) == (
        "8",
        "none",
        "1",
    )
    assert shown["Flow control"] == "none"

    # The bytes received and sent, counted
    app.page.wait_for_function(
        "() => [...document.querySelectorAll('.port-info dt')]"
        ".find(dt => dt.textContent === 'Received').nextElementSibling.textContent !== '0 B'"
    )
    app.send_text("led on")
    app.wait_for_mock("written.length > 0")
    app.page.wait_for_timeout(100)
    assert details(app)["Sent"] == "6 B"


def test_notifies_connecting_and_failures(app: App):
    app.open_port()
    app.mock("writeError = 'The device did not respond.'")
    for key in ["1", "2"]:
        app.send_text(key)
    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").wait_for()
    app.page.wait_for_timeout(200)

    assert notifications(app) == [
        ("info", "Port closed"),
        ("error", "Failed to send"),
        ("error", "Failed to send"),
        ("success", "Port opened"),
    ]


def test_shows_a_summary_when_too_short_for_the_details(app: App):
    app.open_port()
    details = app.page.locator(".port-info")
    summary = app.page.locator(".port-summary")
    expect(details).to_be_visible()
    expect(summary).to_be_hidden()

    app.page.set_viewport_size({"width": 800, "height": 600})
    expect(details).to_be_hidden()
    expect(summary).to_be_visible()
    expect(summary).to_contain_text("Uno R3 (CDC ACM) · Arduino SA")
    expect(summary).to_contain_text("9600 bps 8N1")
    expect(summary).to_contain_text("Port opened")


def test_opens_with_the_settings_chosen_beside_the_baud_rate(app: App):
    page = app.page
    baudrate = page.get_by_placeholder("Baud rate (e.g. 9600)")
    trigger = page.get_by_role("button", name="Port settings")
    # Nothing to set with no port
    expect(baudrate).to_be_disabled()
    expect(trigger).to_be_disabled()
    page.locator(".toolbar-port button").first.click()
    # Emptied, it has no rate to open at: not the last one typed
    baudrate.fill("9600")
    baudrate.fill("")
    expect(page.get_by_role("button", name="Open port")).to_be_disabled()
    baudrate.fill("9600")
    expect(trigger).to_have_text("8N1")
    trigger.click()
    for setting, choice in [
        ("Data bits", "7"),
        ("Parity", "even"),
        ("Stop bits", "2"),
        ("Flow control", "hardware"),
    ]:
        page.get_by_role("group", name=setting).get_by_text(choice, exact=True).click()
    expect(trigger).to_have_text("7E2")
    page.keyboard.press("Escape")

    page.get_by_role("button", name="Open port").click()
    expect(page.get_by_role("button", name="Close port")).to_be_visible()
    # Nor while it is open
    expect(baudrate).to_be_disabled()
    expect(trigger).to_be_disabled()
    assert app.mock("options") == {
        "baudRate": 9600,
        "dataBits": 7,
        "stopBits": 2,
        "parity": "even",
        "flowControl": "hardware",
    }
    shown = details(app)
    assert (shown["Data bits"], shown["Parity"], shown["Stop bits"]) == (
        "7",
        "even",
        "2",
    )
    assert shown["Flow control"] == "hardware"


def test_has_the_toolbar_in_the_menu_when_narrow(app: App):
    page = app.page
    menu_button = page.get_by_role("button", name="Menu")
    toolbar = page.locator(".toolbar")
    # All on the toolbar, and the app's tools on the navbar
    expect(menu_button).to_be_hidden()
    page.set_viewport_size({"width": 900, "height": 800})
    expect(toolbar).to_be_hidden()

    menu_button.click()
    menu = page.get_by_role("dialog")
    expect(menu.get_by_role("region", name="App")).to_have_count(0)
    menu.get_by_title("Request Serial Port").click()
    menu.get_by_placeholder("Baud rate (e.g. 9600)").fill("115200")
    expect(menu.get_by_role("button", name="Port settings")).to_have_text("8N1")
    menu.get_by_role("button", name="Open port").click()
    expect(menu.get_by_role("button", name="Close port")).to_be_visible()
    assert app.mock("options")["baudRate"] == 115200

    # A tab picked, the menu gives way to it
    menu.get_by_role("tab", name="Data").click()
    expect(menu).to_be_hidden()
    expect(page.locator(".data-grid")).to_be_visible()
