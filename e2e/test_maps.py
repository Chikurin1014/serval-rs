import re

from playwright.sync_api import expect

from app import NAME_VALUE, App


def is_whole_line(text: str) -> bool:
    return re.fullmatch(r"(temp|volt):[\d.]+", text) is not None


def test_initial_map_splits_raw_bytes_into_lines(app: App):
    app.open_port()
    app.tab("Data")
    expect(app.map_cards()).to_have_count(1)
    app.wait_for_labels("raw_str")
    assert is_whole_line(app.data_rows()["raw_str"][1])


def test_regex_map_extracts_numbers(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("temp", "volt")
    rows = app.data_rows()
    assert rows["temp"][0] == "Number"
    assert rows["volt"][0] == "Number"


def test_lines_stay_whole_after_clearing_all_data(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("raw_str", "temp", "volt")

    app.page.locator(".data-grid").get_by_role("button", name="Clear all").click()
    app.wait_for_labels("raw_str", "temp", "volt")

    # The maps pick up from the new data, without splitting a line in two
    broken = []
    for _ in range(20):
        line = app.data_rows()["raw_str"][1]
        if not is_whole_line(line):
            broken.append(line)
        app.page.wait_for_timeout(100)
    assert broken == []


def test_invalid_regex_shows_an_error(app: App):
    app.tab("Data")
    app.add_regex_map(pattern="(")
    expect(app.page.locator(".field-error")).to_contain_text("Invalid regex")


def test_adding_and_removing_maps_warns_nothing(app: App):
    app.open_port()
    app.tab("Data")
    start = len(app.console)
    for kind in ["Regex (to Number)", "Split (from Byte)", "Regex (to String)"]:
        app.add_map(kind)
    expect(app.map_cards()).to_have_count(4)

    # Run the first one added, so its signals are used before it is removed
    card = app.map_cards().nth(1)
    card.get_by_placeholder("Source label").fill("raw_str")
    card.get_by_placeholder("Text to be matched").fill(NAME_VALUE)
    card.get_by_placeholder("Target label").fill("$1")
    card.get_by_label("To", exact=True).fill("$2")
    card.get_by_role("switch").click()
    app.wait_for_labels("temp", "volt")

    for _ in range(3):
        app.map_cards().last.get_by_role("button", name="Delete map").click()
    expect(app.map_cards()).to_have_count(1)

    warnings = [
        message.text
        for message in app.console[start:]
        if message.type in ("warning", "error")
        # `dx serve`'s hot reload, absent from a static server
        and "/_dioxus" not in message.text
    ]
    assert warnings == []
