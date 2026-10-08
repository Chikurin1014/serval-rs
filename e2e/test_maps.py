import re

import pytest
from playwright.sync_api import expect

from app import NAME_VALUE, App


def is_whole_line(text: str) -> bool:
    return re.fullmatch(r"(temp|volt):[\d.]+", text) is not None


def test_initial_map_splits_raw_bytes_into_lines(app: App):
    app.open_port()
    app.tab("Data")
    expect(app.map_cards()).to_have_count(3)
    app.wait_for_labels("message")
    assert is_whole_line(app.data_rows()["message"][1])


def test_initial_regex_map_reads_numbers_with_no_label(app: App):
    app.tab("Data")
    card = app.map_cards().nth(1)
    expect(card.locator(".map-title-text")).to_have_text("Regex")
    expect(card.get_by_role("switch")).to_be_checked()
    card.locator(".map-title").click()
    expect(card.get_by_placeholder("Input label")).to_have_value("message")
    expect(card.get_by_placeholder("Text to be matched")).to_have_value("^{number}")
    expect(card.get_by_placeholder("Output label")).to_have_value("anonymous data")
    expect(card.get_by_label("To", exact=True)).to_have_value("$0")

    app.open_port()
    app.mock("receive('-1.5e2\\n')")
    app.wait_for_labels("anonymous data")
    assert app.data_rows()["anonymous data"][1] == "-150.00"
    app.page.wait_for_timeout(300)
    assert app.data_rows()["anonymous data"][1] == "-150.00"


def test_initial_regex_map_reads_name_value_numbers(app: App):
    app.tab("Data")
    card = app.map_cards().nth(2)
    expect(card.locator(".map-title-text")).to_have_text("Regex")
    expect(card.get_by_role("switch")).to_be_checked()
    card.locator(".map-title").click()
    expect(card.get_by_placeholder("Input label")).to_have_value("message")
    expect(card.get_by_placeholder("Text to be matched")).to_have_value(
        "({word}): ({number})"
    )
    expect(card.get_by_placeholder("Output label")).to_have_value("$1")
    expect(card.get_by_label("To", exact=True)).to_have_value("$2")


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
    app.wait_for_labels("message", "temp", "volt")

    app.clear_all()
    app.wait_for_labels("message", "temp", "volt")

    broken = []
    for _ in range(20):
        line = app.data_rows()["message"][1]
        if not is_whole_line(line):
            broken.append(line)
        app.page.wait_for_timeout(100)
    assert broken == []


def test_invalid_regex_shows_an_error(app: App):
    app.tab("Data")
    app.add_regex_map(pattern="(")
    expect(app.page.locator(".field-error")).to_contain_text("Invalid regex")


def test_settings_are_locked_while_the_map_is_on(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map(target="all_$1")
    app.wait_for_labels("all_temp", "all_volt")
    card = app.map_cards().last
    fields = card.locator(".map-content input")
    pattern = card.get_by_placeholder("Text to be matched")
    switch = card.get_by_role("switch")
    tip = card.locator("[role=tooltip]")

    expect(fields).to_have_count(4)
    for field in fields.all():
        expect(field).not_to_be_editable()
    expect(tip).to_be_hidden()
    card.get_by_placeholder("Output label").hover()
    # One for the whole form, in its middle
    expect(tip).to_have_count(1)
    expect(tip).to_be_visible()
    expect(tip).to_have_text("Turn this Map off to edit")
    content, box = card.locator(".map-content").bounding_box(), tip.bounding_box()
    assert (
        abs((content["x"] + content["width"] / 2) - (box["x"] + box["width"] / 2)) < 1
    )
    assert (
        abs((content["y"] + content["height"] / 2) - (box["y"] + box["height"] / 2)) < 1
    )

    switch.click()
    for field in fields.all():
        expect(field).to_be_editable()
    card.get_by_placeholder("Output label").hover()
    expect(tip).to_have_count(0)
    pattern.fill(r"(temp):({number})")
    card.get_by_placeholder("Output label").fill("only_$1")
    switch.click()
    expect(pattern).not_to_be_editable()

    app.wait_for_labels("only_temp")
    app.page.wait_for_timeout(300)
    assert "only_volt" not in app.data_rows()


def test_changed_settings_read_the_input_again(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map(target="first_$1")
    app.wait_for_labels("first_temp", "first_volt")
    app.page.get_by_role("button", name="Close port").click()
    app.page.get_by_role("button", name="Open port").wait_for()

    card = app.map_cards().last
    switch = card.get_by_role("switch")
    switch.click()
    card.get_by_placeholder("Output label").fill("again_$1")
    switch.click()

    # Only possible by reading again from the start
    app.wait_for_labels("again_temp", "again_volt")


def test_adding_and_removing_maps_warns_nothing(app: App):
    app.open_port()
    app.tab("Data")
    start = len(app.console)
    for kind in [
        ("Regex", "String", "Number"),
        ("Decode", "Bytes", "String"),
        ("Regex", "String", "String"),
    ]:
        app.add_map(*kind)
    expect(app.map_cards()).to_have_count(6)

    card = app.map_cards().nth(3)
    card.get_by_placeholder("Input label").fill("message")
    card.get_by_placeholder("Text to be matched").fill(NAME_VALUE)
    card.get_by_placeholder("Output label").fill("$1")
    card.get_by_label("To", exact=True).fill("$2")
    card.get_by_role("switch").click()
    app.wait_for_labels("temp", "volt")

    for _ in range(3):
        app.map_cards().last.get_by_role("button", name="Delete map").click()
    expect(app.map_cards()).to_have_count(3)

    warnings = [
        message.text
        for message in app.console[start:]
        if message.type in ("warning", "error") and "/_dioxus" not in message.text
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
    expect(option.locator(".add-map-title")).to_have_text(
        ["Add", "Subtract", "Multiply", "Divide", "Differentiate", "Integrate", "Regex"]
    )
    expect(option.locator(".map-types").last).to_have_text("StringNumber")

    number.click()
    expect(number).to_have_attribute("aria-expanded", "true")

    string = bar.get_by_role("button", name="String", exact=True)
    string.hover()
    expect(number).to_have_attribute("aria-expanded", "false")
    expect(string).to_have_attribute("aria-expanded", "true")

    app.page.mouse.move(0, 0)
    expect(string).to_have_attribute("aria-expanded", "false")
    expect(bar.get_by_role("option")).to_have_count(0)


def test_bytes_menu_offers_encode(app: App):
    app.tab("Data")
    bar = app.page.locator(".add-map-bar")
    bar.get_by_role("button", name="Bytes", exact=True).hover()
    option = bar.get_by_role("option")
    expect(option.locator(".add-map-title")).to_have_text(["Encode"])
    expect(option.locator(".map-types")).to_have_text(["StringBytes"])


def test_decode_without_delimiter_keeps_each_chunk(app: App):
    app.open_port()
    app.tab("Data")
    app.add_map("Decode", "Bytes", "String")
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("raw_bytes")
    card.get_by_placeholder("Output label").fill("chunk")
    card.get_by_label("Delimiter").fill("")
    card.get_by_role("switch").click()
    app.wait_for_labels("chunk")
    chunk = app.data_rows()["chunk"]
    assert chunk[0] == "String"
    assert re.fullmatch(r"temp:[\d.]+\nvolt:[\d.]+\n", chunk[1])


def test_encode_turns_strings_into_bytes(app: App):
    app.open_port()
    app.tab("Data")
    app.add_map("Encode", "String", "Bytes")
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("message")
    card.get_by_placeholder("Output label").fill("line_bytes")
    expect(card.get_by_label("Delimiter")).to_have_count(0)
    card.get_by_role("switch").click()
    app.wait_for_labels("line_bytes")
    line = app.data_rows()["line_bytes"]
    assert line[0] == "Bytes"
    assert re.fullmatch(r"(temp|volt):[\d.]+", line[1])


def test_cards_open_to_their_settings(app: App):
    app.tab("Data")
    initial = app.map_cards().first
    expect(initial.get_by_placeholder("Input label")).to_be_hidden()
    app.add_map("Regex", "String", "Number")
    expect(app.map_cards().last.get_by_placeholder("Input label")).to_be_visible()

    initial.locator(".map-title").click()
    expect(initial.get_by_placeholder("Input label")).to_be_visible()


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
    expect(latest).to_contain_text("message")

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


def test_output_label_leaves_out_the_text_around_the_match(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map(
        pattern="{number}",
        target="foo",
        replacement="$0",
    )
    app.wait_for_labels("foo")
    assert not any(label.endswith(":foo") for label in app.data_rows())


def test_card_header_shows_the_types(app: App):
    app.tab("Data")
    app.add_map("Regex", "String", "Number")
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Decode", "Regex", "Regex", "Regex"]
    )
    expect(app.map_cards().locator(".map-types")).to_have_text(
        ["BytesString", "StringNumber", "StringNumber", "StringNumber"]
    )


def add_replace_map(app: App, pattern: str, replacement: str, target: str):
    app.add_map("Replace", "String", "String")
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("message")
    card.get_by_placeholder("Text to be replaced").fill(pattern)
    card.get_by_placeholder("Output label").fill(target)
    card.get_by_label("To", exact=True).fill(replacement)
    card.get_by_role("switch").click()


def test_replace_replaces_every_match(app: App):
    app.open_port()
    app.tab("Data")
    add_replace_map(app, r"(\w+):", "$1=", "assigned")
    app.wait_for_labels("assigned")
    assert re.fullmatch(r"(temp|volt)=[\d.]+", app.data_rows()["assigned"][1])

    value = app.map_cards().last.locator(".map-latest > .map-latest-value")
    expect(value.locator('.map-segment[data-from-input="false"]')).to_have_text(["="])
    expect(value.locator('.map-segment[data-from-input="true"]').first).to_have_text(
        re.compile(r"^(temp|volt)$")
    )


def test_concat_joins_the_newest_of_both(app: App):
    app.open_port()
    app.tab("Data")
    add_replace_map(app, ":", "=", "assigned")
    app.add_map("Concat", "String", "String")
    card = app.map_cards().last
    card.get_by_placeholder("First input label").fill("message")
    card.get_by_placeholder("Second input label").fill("assigned")
    card.get_by_placeholder("Output label").fill("joined")
    card.get_by_label("Separator").fill(" | ")
    card.get_by_role("switch").click()
    app.wait_for_labels("joined")
    assert re.fullmatch(
        r"(temp|volt):[\d.]+ \| (temp|volt)=[\d.]+", app.data_rows()["joined"][1]
    )

    expect(card.locator(".map-types-from > span")).to_have_text(["String", "String"])
    expect(card.locator(".map-latest-from .map-latest-label")).to_have_text(
        ["message", "assigned"]
    )


def add_arithmetic_map(app: App, name: str, first: str, second: str, target: str):
    app.add_map(name, "Number", "Number")
    card = app.map_cards().last
    fields = card.get_by_placeholder("Label or number")
    fields.nth(0).fill(first)
    fields.nth(1).fill(second)
    card.get_by_placeholder("Output label").fill(target)
    card.get_by_role("switch").click()
    return card


def latest_numbers(card) -> tuple[list[str], float]:
    """The latest conversion's inputs and result, read at once."""
    inputs, result = card.evaluate(
        """card => [
            [...card.querySelectorAll(".map-latest-from .map-latest-value")].map(v => v.textContent),
            card.querySelector(".map-latest > .map-latest-value").textContent,
        ]"""
    )
    return inputs, float(result)


def test_arithmetic_with_a_constant(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    card = add_arithmetic_map(app, "Multiply", "temp", "2", "doubled")
    app.wait_for_labels("doubled")
    assert app.data_rows()["doubled"][0] == "Number"

    expect(card.locator(".map-latest-from .map-latest-label")).to_have_text(
        ["temp", ""]
    )
    (value, constant), result = latest_numbers(card)
    assert constant == "2.0000"
    assert result == pytest.approx(float(value) * 2, rel=1e-4)


def test_arithmetic_prefers_a_label_to_a_number(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.add_regex_map(target="10")
    app.wait_for_labels("10")
    card = add_arithmetic_map(app, "Add", "temp", "10", "sum")
    app.wait_for_labels("sum")
    expect(card.locator(".map-latest-from .map-latest-label")).to_have_text(
        ["temp", "10"]
    )
    (first, second), result = latest_numbers(card)
    assert result == pytest.approx(float(first) + float(second), rel=1e-4)
    assert float(second) != 10


def test_division_by_zero_shows_an_error(app: App):
    app.tab("Data")
    card = add_arithmetic_map(app, "Divide", "1", "0", "quotient")
    expect(card.locator(".field-error")).to_have_text("Division by zero")


def test_unknown_operand_shows_an_error(app: App):
    app.tab("Data")
    card = add_arithmetic_map(app, "Subtract", "nothing", "1", "difference")
    expect(card.locator(".field-error")).to_contain_text("nothing")


def test_formula_renders_with_katex(app: App):
    failed = []
    app.page.on(
        "response",
        lambda response: response.status >= 400 and failed.append(response.url),
    )
    app.page.on("requestfailed", lambda request: failed.append(request.url))
    app.tab("Data")
    card = add_arithmetic_map(app, "Divide", "a", "b", "quotient")
    formula = card.locator(".map-formula .formula")
    expect(formula).to_have_attribute("data-rendered", "true")
    expect(formula.locator(".katex-mathml annotation")).to_have_text("a / b")
    expect(formula.locator(".formula-fallback")).to_be_hidden()
    app.page.wait_for_function("document.fonts.status === 'loaded'")
    assert app.page.evaluate("document.fonts.check('1em KaTeX_Math')")
    assert failed == []


def regex_item(app: App, output: str):
    trigger = app.page.get_by_role("button", name=output, exact=True)
    menu = app.page.locator(".add-map-bar .hover-menu").filter(has=trigger)
    trigger.hover()
    return menu.get_by_role("option").filter(
        has=app.page.locator(".add-map-title", has_text=re.compile("^Regex$"))
    )


def test_regex_presets_show_beside_the_item(app: App):
    app.tab("Data")
    item = regex_item(app, "Number")
    presets = item.get_by_role("menuitem")
    expect(presets.first).to_be_hidden()
    item.hover()
    expect(presets.locator(".add-map-preset-name")).to_have_text(
        ["value", "name: value", "Arduino", "Teleplot"]
    )

    item = regex_item(app, "String")
    item.hover()
    expect(item.get_by_role("menuitem").locator(".add-map-preset-name")).to_have_text(
        ["name: value", "Arduino"]
    )


def test_regex_preset_adds_a_set_map(app: App):
    app.tab("Data")
    item = regex_item(app, "Number")
    item.hover()
    item.get_by_role("menuitem").filter(has_text="Teleplot").click()

    expect(app.map_cards()).to_have_count(4)
    card = app.map_cards().last
    expect(card.get_by_placeholder("Text to be matched")).to_have_value(
        ">({word}):({number})"
    )
    expect(card.get_by_placeholder("Output label")).to_have_value("$1")
    expect(card.get_by_label("To", exact=True)).to_have_value("$2")
    expect(card.locator(".map-types")).to_have_text("StringNumber")


def test_value_preset_leaves_the_output_label_to_be_set(app: App):
    app.tab("Data")
    item = regex_item(app, "Number")
    item.hover()
    item.get_by_role("menuitem").filter(
        has=app.page.locator(".add-map-preset-name", has_text=re.compile("^value$"))
    ).click()

    card = app.map_cards().last
    expect(card.get_by_placeholder("Text to be matched")).to_have_value("^{number}")
    expect(card.get_by_placeholder("Output label")).to_have_value("")
    expect(card.get_by_label("To", exact=True)).to_have_value("$0")


def test_regex_presets_by_keyboard(app: App):
    app.tab("Data")
    item = regex_item(app, "String")
    item.focus()
    app.page.keyboard.press("ArrowRight")
    presets = item.get_by_role("menuitem")
    expect(presets.first).to_be_focused()
    app.page.keyboard.press("ArrowDown")
    expect(presets.nth(1)).to_be_focused()
    app.page.keyboard.press("ArrowLeft")
    expect(item).to_be_focused()

    app.page.keyboard.press("ArrowRight")
    app.page.keyboard.press("ArrowDown")
    app.page.keyboard.press("Enter")
    expect(app.map_cards()).to_have_count(4)
    expect(app.map_cards().last.get_by_placeholder("Text to be matched")).to_have_value(
        "({word}):(?!{number})(.+)"
    )


def add_calculus_map(app: App, name: str, target: str):
    app.add_map(name, "Number", "Number")
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("temp")
    card.get_by_placeholder("Output label").fill(target)
    card.get_by_role("switch").click()
    return card


def test_differentiate_and_integrate(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    add_calculus_map(app, "Differentiate", "temp_rate")
    add_calculus_map(app, "Integrate", "temp_total")
    derivative = app.map_cards().nth(-2)
    expect(derivative.locator(".map-title-text")).to_have_text("Differentiate")
    app.wait_for_labels("temp_rate", "temp_total")
    rows = app.data_rows()
    assert rows["temp_rate"][0] == "Number"
    assert rows["temp_total"][0] == "Number"

    first = float(app.data_rows()["temp_total"][1])
    app.page.wait_for_timeout(500)
    assert float(app.data_rows()["temp_total"][1]) > first

    expect(derivative.locator(".map-formula .formula")).to_have_attribute(
        "data-rendered", "true"
    )
    expect(derivative.locator(".katex-mathml annotation")).to_have_text(
        r"\frac{d}{dt} f(t)"
    )


def test_cards_slide_open_and_closed(app: App):
    app.tab("Data")
    card = app.map_cards().first
    content = card.locator("[data-open]").last
    field = card.get_by_placeholder("Input label")

    def height():
        return content.evaluate("el => el.getBoundingClientRect().height")

    assert height() < 1
    card.locator(".map-title").click()
    app.page.wait_for_timeout(80)
    midway = height()
    expect(field).to_be_visible()
    app.page.wait_for_timeout(300)
    assert 0 < midway < height()

    card.locator(".map-title").click()
    expect(field).to_be_hidden()
    app.page.wait_for_function(
        "el => el.getBoundingClientRect().height < 1", arg=content.element_handle()
    )
