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
    for kind in [
        ("Regex", "String", "Number"),
        ("Split", "Bytes", "String"),
        ("Regex", "String", "String"),
    ]:
        app.add_map(*kind)
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


def test_add_menus_are_by_output_type(app: App):
    app.tab("Data")
    bar = app.page.locator(".add-map-bar")
    expect(bar.locator(".add-map-label")).to_have_text("Map to")
    expect(bar.get_by_role("button")).to_have_text(["Bytes", "Number", "String"])


def test_add_menus_open_on_hover(app: App):
    app.tab("Data")
    bar = app.page.locator(".add-map-bar")
    number = bar.get_by_role("button", name="Number", exact=True)
    number.hover()
    expect(number).to_have_attribute("aria-expanded", "true")
    option = bar.get_by_role("option")
    expect(option.locator(".add-map-title")).to_have_text(["Regex"])
    expect(option.locator(".map-types")).to_have_text(["StringNumber"])

    # Clicking the trigger of the open menu keeps it open
    number.click()
    expect(number).to_have_attribute("aria-expanded", "true")

    # Moving to another trigger opens its menu instead
    string = bar.get_by_role("button", name="String", exact=True)
    string.hover()
    expect(number).to_have_attribute("aria-expanded", "false")
    expect(string).to_have_attribute("aria-expanded", "true")

    app.page.mouse.move(0, 0)
    expect(string).to_have_attribute("aria-expanded", "false")
    expect(bar.get_by_role("option")).to_have_count(0)


def test_menu_without_kinds_does_not_open(app: App):
    app.tab("Data")
    bytes_menu = app.page.locator(".add-map-bar").get_by_role("button", name="Bytes", exact=True)
    expect(bytes_menu).to_be_disabled()
    bytes_menu.hover(force=True)
    expect(bytes_menu).to_have_attribute("aria-expanded", "false")


def test_cards_open_to_their_settings(app: App):
    app.tab("Data")
    # The initial map starts closed; one added starts open to be set
    initial = app.map_cards().first
    expect(initial.get_by_placeholder("Source label")).to_have_count(0)
    app.add_map("Regex", "String", "Number")
    expect(app.map_cards().last.get_by_placeholder("Source label")).to_be_visible()

    initial.locator(".map-title").click()
    expect(initial.get_by_placeholder("Source label")).to_be_visible()


def test_delete_button_shows_on_hover(app: App):
    app.tab("Data")
    app.page.mouse.move(0, 0)
    remove = app.map_cards().first.get_by_role("button", name="Delete map")
    expect(remove).to_have_css("opacity", "0")
    app.map_cards().first.hover()
    expect(remove).to_have_css("opacity", "1")


def test_title_shows_the_latest_conversion(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    latest = app.map_cards().last.locator(".map-latest")
    expect(latest).to_contain_text("raw_str")

    # The label and value both come from the input's groups (`$1`, `$2`)
    taken = latest.locator('.map-segment[data-from-input="true"]')
    expect(taken).to_have_count(2)
    assert taken.nth(0).text_content() in ("temp", "volt")
    assert re.fullmatch(r"[\d.]+", taken.nth(1).text_content())


def test_fixed_parts_of_the_target_are_not_marked(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map(output="String", target="reading_$1", replacement="ok")
    latest = app.map_cards().last.locator(".map-latest")
    expect(latest.locator(".map-segment")).to_have_count(3)
    expect(latest.locator('.map-segment[data-from-input="false"]')).to_have_text(
        ["reading_", "ok"]
    )


def test_card_header_shows_the_types(app: App):
    app.tab("Data")
    app.add_map("Regex", "String", "Number")
    expect(app.map_cards().locator(".map-title-text")).to_have_text(["Split", "Regex"])
    expect(app.map_cards().locator(".map-types")).to_have_text(
        ["BytesString", "StringNumber"]
    )
