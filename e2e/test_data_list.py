import re

from playwright.sync_api import expect

from app import App


def test_empty_until_a_port_is_open(app: App):
    app.tab("Data")
    expect(app.page.locator(".data-grid .data-table")).to_be_visible()
    assert app.data_rows() == {}


def test_shows_each_label_with_its_latest_value(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("raw_data", "raw_str", "temp", "volt")
    rows = app.data_rows()
    now = app.page.evaluate("Date.now()")

    assert {label: row[0] for label, row in rows.items()} == {
        "raw_data": "Bytes",
        "raw_str": "String",
        "temp": "Number",
        "volt": "Number",
    }
    assert re.fullmatch(r"(temp|volt):[\d.]+", rows["raw_str"][1])
    assert re.fullmatch(r"[\d.]+", rows["temp"][1])
    for label, (_, _, timestamp) in rows.items():
        assert 0 <= now - int(timestamp) < 2000, f"{label} is not recent"


def test_deletes_a_label(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("temp", "volt")
    # Stop the stream first, or the next reading brings the label back
    app.page.get_by_role("button", name="Close port").click()
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()

    rows = app.page.locator(".data-grid .data-table tbody tr")
    rows.filter(has=app.page.locator("th", has_text="volt")).get_by_role(
        "button", name="Delete label"
    ).click()

    expect(rows).to_have_count(3)
    assert set(app.data_rows()) == {"raw_data", "raw_str", "temp"}
