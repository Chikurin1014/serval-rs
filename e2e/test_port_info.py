from playwright.sync_api import expect

from app import App


def details(app: App) -> dict[str, str]:
    return app.page.evaluate(
        """() => Object.fromEntries([...document.querySelectorAll('.port-info dt')]
            .map(dt => [dt.textContent, dt.nextElementSibling.textContent]))"""
    )


def log(app: App) -> list[tuple[str, str]]:
    """The log, newest first, as `(kind, title)`."""
    return [
        tuple(entry)
        for entry in app.page.evaluate(
            """() => [...document.querySelectorAll('.port-log li')]
                .map(li => [li.dataset.kind, li.querySelector('.port-log-title').textContent])"""
        )
    ]


def test_shows_the_device_and_its_settings(app: App):
    app.open_port()
    shown = details(app)
    assert shown["Vendor"] == "Arduino SA"
    assert shown["Product"] == "Uno R3 (CDC ACM)"
    assert shown["Baudrate"] == "9600 bps"
    assert (shown["Data bits"], shown["Parity"], shown["Stop bits"]) == (
        "8",
        "none",
        "1",
    )
    assert shown["Flow control"] == "none"


def test_counts_the_bytes_received_and_sent(app: App):
    app.open_port()
    app.page.wait_for_function(
        "() => [...document.querySelectorAll('.port-info dt')]"
        ".find(dt => dt.textContent === 'Received').nextElementSibling.textContent !== '0 B'"
    )
    field = app.page.get_by_placeholder("Type text to send to the active port")
    field.fill("led on")
    field.press("Enter")
    app.page.wait_for_function("() => window.mockSerialPort.written.length > 0")
    app.page.wait_for_timeout(100)
    assert details(app)["Sent"] == "6 B"


def test_logs_connecting_and_failures(app: App):
    app.open_port()
    app.page.evaluate(
        "window.mockSerialPort.writeError = 'The device did not respond.'"
    )
    field = app.page.get_by_placeholder("Type text to send to the active port")
    for text in ["one", "two"]:
        field.fill(text)
        field.press("Enter")
    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").wait_for()
    app.page.wait_for_timeout(200)

    # Every failure is logged, though the toast holds back repeats
    assert log(app) == [
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

    # Narrow: the panel goes under the console, too short for the details
    app.page.set_viewport_size({"width": 800, "height": 600})
    expect(details).to_be_hidden()
    expect(summary).to_be_visible()
    expect(summary).to_contain_text("Uno R3 (CDC ACM) · Arduino SA")
    expect(summary).to_contain_text("9600 bps 8N1")
    expect(summary).to_contain_text("Port opened")
