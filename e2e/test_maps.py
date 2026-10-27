import re

import pytest
from playwright.sync_api import expect

from app import NAME_VALUE, App, name_value_map, saved_map

# The types of two vectors' maps
V2 = ["Vector2d", "Vector2d"]
V3 = ["Vector3d", "Vector3d"]


def is_whole_line(text: str) -> bool:
    return re.fullmatch(r"(temp|volt):[\d.]+", text) is not None


def test_initial_maps_read_what_the_port_sends(app: App):
    app.tab("Data")
    decode, numbers, name_value = (app.map_cards().nth(index) for index in range(3))
    expect(app.map_cards()).to_have_count(3)
    # Lines of raw bytes, as logs of their levels
    expect(decode.locator(".map-title-text")).to_have_text("Decode")
    decode.locator(".map-title").click()
    expect(decode.get_by_placeholder("Input label")).to_have_value("raw_bytes")
    expect(decode.get_by_placeholder("Text to be matched")).to_have_value(
        r"^(?:\[(?:.*? )??({level})(?: .*?)?\] )?(.*)$"
    )
    expect(decode.get_by_placeholder("Output label")).to_have_value("log")
    expect(decode.get_by_label("Level", exact=True)).to_have_value("$1")
    expect(decode.get_by_label("Message", exact=True)).to_have_value("$2")
    # Numbers with no label
    expect(numbers.locator(".map-title-text")).to_have_text("Match")
    expect(numbers.get_by_role("switch")).to_be_checked()
    numbers.locator(".map-title").click()
    expect(numbers.get_by_placeholder("Input label")).to_have_value("log")
    expect(numbers.get_by_placeholder("Text to be matched")).to_have_value("^{number}")
    expect(numbers.get_by_placeholder("Output label")).to_have_value("anonymous data")
    expect(numbers.get_by_label("To", exact=True)).to_have_value("$0")
    # Numbers after their names
    expect(name_value.locator(".map-title-text")).to_have_text("Match")
    expect(name_value.get_by_role("switch")).to_be_checked()
    name_value.locator(".map-title").click()
    expect(name_value.get_by_placeholder("Input label")).to_have_value("log")
    expect(name_value.get_by_placeholder("Text to be matched")).to_have_value(
        "({word}): ({number})"
    )
    expect(name_value.get_by_placeholder("Output label")).to_have_value("$1")
    expect(name_value.get_by_label("To", exact=True)).to_have_value("$2")

    # Whole lines, of no level: the line alone
    app.open_port()
    app.wait_for_labels("log")
    kind, latest, _ = app.data_rows()["log"]
    assert kind == "Log" and latest.startswith(("temp:", "volt:"))
    assert is_whole_line(latest)

    app.mock("mute()")
    app.mock("receive('[warning] disk full\\n')")
    wait_for_latest(app, "log", "WARN disk full")
    # Its level as a tag in its color, in the data list too
    row = app.page.locator(".data-row").filter(
        has=app.page.locator("th", has_text="log")
    )
    tag = row.locator(".log-level-tag")
    expect(tag).to_have_text("WARN")
    expect(tag).to_have_attribute("data-level", "WARN")
    colors = tag.evaluate(
        "t => [getComputedStyle(t).color, getComputedStyle(t.parentElement).color]"
    )
    assert colors[0] != colors[1]
    # Its level by its name, in capitals
    latest = decode.locator(".map-latest-to")
    expect(latest.locator(".log-level-tag")).to_have_text("WARN")
    expect(latest).to_contain_text("disk full")
    # The bytes it took, in hex
    expect(decode.locator(".map-latest-from .map-latest-value")).to_have_text(
        " ".join(f"{byte:02X}" for byte in b"[warning] disk full\n")
    )

    app.mock("receive('-1.5e2\\n')")
    app.wait_for_labels("anonymous data")
    assert app.data_rows()["anonymous data"][1] == "-150.00"
    app.page.wait_for_timeout(300)
    assert app.data_rows()["anonymous data"][1] == "-150.00"


def test_match_map_extracts_numbers(app: App):
    app.open_port()
    app.tab("Data")
    app.add_match_map()
    app.wait_for_labels("temp", "volt")
    rows = app.data_rows()
    assert rows["temp"][0] == "Number"
    assert rows["volt"][0] == "Number"


def test_lines_stay_whole_after_clearing_all_data(app: App):
    app.open_port()
    app.tab("Data")
    app.add_match_map()
    app.wait_for_labels("log", "temp", "volt")

    app.clear_all()
    app.wait_for_labels("log", "temp", "volt")

    broken = []
    for _ in range(20):
        line = app.data_rows()["log"][1]
        if not is_whole_line(line):
            broken.append(line)
        app.page.wait_for_timeout(100)
    assert broken == []


def test_settings_are_locked_while_the_map_is_on(app: App):
    app.open_port()
    app.tab("Data")
    app.add_match_map(target="all_$1")
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
    app.add_match_map(target="first_$1")
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
        ("Match", "Log", "Number"),
        ("Decode", "Bytes", "Log"),
        ("Match", "Log", "Log"),
    ]:
        app.add_map(*kind)
    expect(app.map_cards()).to_have_count(6)

    card = app.map_cards().nth(3)
    card.get_by_placeholder("Input label").fill("log")
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
        if message.type in ("warning", "error")
        and "/_dioxus" not in message.text
        # The headless browser's GPU driver, under the console's WebGL renderer
        and "GL Driver Message" not in message.text
    ]
    assert warnings == []


def test_add_menus_offer_the_maps_by_output_type(app: App):
    app.tab("Data")
    bar = app.page.locator(".add-map-bar")
    expect(bar.locator(".add-map-label")).to_have_text("Map to")
    expect(bar.get_by_role("button")).to_have_text(["Bytes", "Log", "Number", "Vector"])

    # Opened as they are pointed at
    number = bar.get_by_role("button", name="Number", exact=True)
    number.hover()
    expect(number).to_have_attribute("aria-expanded", "true")
    option = bar.get_by_role("option")
    expect(option.locator(".add-map-title")).to_have_text(
        [
            "Match",
            "Add",
            "Subtract",
            "Multiply",
            "Divide",
            "Pow",
            "Differentiate",
            "Integrate",
            "Dot",
            "Cross",
            "Magnitude",
            "Decompose",
        ]
    )
    expect(option.locator("> .map-types").first).to_have_text("LogNumber")

    number.click()
    expect(number).to_have_attribute("aria-expanded", "true")

    string = bar.get_by_role("button", name="Log", exact=True)
    string.hover()
    expect(number).to_have_attribute("aria-expanded", "false")
    expect(string).to_have_attribute("aria-expanded", "true")

    app.page.mouse.move(0, 0)
    expect(string).to_have_attribute("aria-expanded", "false")
    expect(bar.get_by_role("option")).to_have_count(0)

    # Bytes of logs, by Encode
    bar.get_by_role("button", name="Bytes", exact=True).hover()
    option = bar.get_by_role("option")
    expect(option.locator(".add-map-title")).to_have_text(["Encode"])
    expect(option.locator(".map-types")).to_have_text(["LogBytes"])

    # Presets of Match beside its item
    item = app.map_item("Number")
    presets = item.get_by_role("menuitem")
    expect(presets.first).to_be_hidden()
    item.hover()
    expect(presets.locator(".add-map-preset-name")).to_have_text(
        ["value", "name: value", "Arduino", "Teleplot"]
    )

    # None for logs: Decode reads them
    expect(app.map_item("Log").get_by_role("menuitem")).to_have_count(0)


def test_decode_splits_at_the_chosen_delimiters(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_map("Decode", "Bytes", "Log")
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("raw_bytes")
    card.get_by_placeholder("Output label").fill("line")
    delimiter = card.get_by_role("group", name="Delimiter")
    cr, lf, crlf = (
        delimiter.get_by_role("button", name=name, exact=True)
        for name in ["\\r", "\\n", "\\r\\n"]
    )
    for button in (cr, lf, crlf):
        expect(button).to_have_attribute("data-state", "on")

    lf.click()
    crlf.click()
    # One is always chosen
    cr.click()
    expect(cr).to_have_attribute("data-state", "on")
    expect(lf).to_have_attribute("data-state", "off")
    expect(crlf).to_have_attribute("data-state", "off")

    card.get_by_role("switch").click()
    app.mock("receive('x1\\ry2\\r')")
    app.wait_for_labels("line")
    expect(card.locator(".map-title")).to_contain_text("y2")
    assert app.data_rows()["line"][1] == "y2"


def test_decode_delimiters_are_locked_while_the_map_is_on(app: App):
    app.tab("Data")
    card = app.map_cards().first
    card.locator(".map-title").click()
    lf = card.get_by_role("group", name="Delimiter").get_by_role(
        "button", name="\\n", exact=True
    )
    lf.click(force=True)
    expect(lf).to_have_attribute("data-state", "on")
    expect(lf).to_be_disabled()


def test_encode_turns_strings_into_bytes(app: App):
    app.open_port()
    app.tab("Data")
    app.add_map("Encode", "Log", "Bytes")
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("log")
    card.get_by_placeholder("Output label").fill("line_bytes")
    expect(card.get_by_label("Delimiter")).to_have_count(0)
    card.get_by_role("switch").click()
    app.wait_for_labels("line_bytes")
    line = app.data_rows()["line_bytes"]
    assert line[0] == "Bytes"
    # In hex: the text's bytes
    assert re.fullmatch(r"[0-9A-F]{2}( [0-9A-F]{2})*", line[1])
    assert re.fullmatch(r"(temp|volt):[\d.]+", bytes.fromhex(line[1]).decode())
    expect(card.locator(".map-latest-to .map-latest-value")).to_have_text(
        re.compile(r"^[0-9A-F]{2}( [0-9A-F]{2})*$")
    )


def test_cards_show_their_types_and_open_to_their_settings(app: App):
    app.tab("Data")
    # Its delete button shown as a card is pointed at
    app.page.mouse.move(0, 0)
    remove = app.map_cards().first.get_by_role("button", name="Delete map")
    expect(remove).to_have_css("opacity", "0")
    app.map_cards().first.hover()
    expect(remove).to_have_css("opacity", "1")

    # Closed, those of the start; open as it is added
    initial = app.map_cards().first
    expect(initial.get_by_placeholder("Input label")).to_be_hidden()
    app.add_map("Match", "Log", "Number")
    expect(app.map_cards().last.get_by_placeholder("Input label")).to_be_visible()
    initial.locator(".map-title").click()
    expect(initial.get_by_placeholder("Input label")).to_be_visible()

    # Each its title and types in its header
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Decode", "Match", "Match", "Match"]
    )
    expect(app.map_cards().locator(".map-types")).to_have_text(
        ["BytesLog", "LogNumber", "LogNumber", "LogNumber"]
    )


def test_title_shows_the_latest_conversion(app: App):
    app.open_port()
    app.tab("Data")
    app.add_match_map()
    latest = app.map_cards().last.locator(".map-latest")
    expect(latest).to_contain_text("log")

    taken = latest.locator('.map-segment[data-from-input="true"]')
    expect(taken).to_have_count(2)
    assert taken.nth(0).text_content() in ("temp", "volt")
    assert re.fullmatch(r"[\d.]+", taken.nth(1).text_content())


def add_replace_map(app: App, pattern: str, replacement: str, target: str):
    app.add_map("Replace", "Log", "Log")
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("log")
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

    value = app.map_cards().last.locator(".map-latest-to > .map-latest-value")
    expect(value.locator('.map-segment[data-from-input="false"]')).to_have_text(["="])
    expect(value.locator('.map-segment[data-from-input="true"]').first).to_have_text(
        re.compile(r"^(temp|volt)$")
    )


def test_concat_joins_the_newest_of_both(app: App):
    app.open_port()
    app.tab("Data")
    add_replace_map(app, ":", "=", "assigned")
    app.add_map("Concat", "Log", "Log")
    card = app.map_cards().last
    card.get_by_placeholder("First input label").fill("log")
    card.get_by_placeholder("Second input label").fill("assigned")
    card.get_by_placeholder("Output label").fill("joined")
    card.get_by_label("Separator").fill(" | ")
    card.get_by_role("switch").click()
    app.wait_for_labels("joined")
    assert re.fullmatch(
        r"(temp|volt):[\d.]+ \| (temp|volt)=[\d.]+", app.data_rows()["joined"][1]
    )

    expect(card.locator(".map-types-from > span")).to_have_text(["Log", "Log"])
    expect(card.locator(".map-latest-from .map-latest-label")).to_have_text(
        ["log", "assigned"]
    )


def add_arithmetic_map(app: App, name: str, first: str, second: str, target: str):
    app.add_map(name, "Number", "Number")
    card = app.map_cards().last
    fields = card.get_by_placeholder("Label, number or #pi")
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
            card.querySelector(".map-latest-to > .map-latest-value").textContent,
        ]"""
    )
    return inputs, float(result)


def test_arithmetic_with_a_constant(app: App):
    app.open_port()
    app.tab("Data")
    app.add_match_map()
    card = add_arithmetic_map(app, "Multiply", "temp", "2", "doubled")
    app.wait_for_labels("doubled")
    assert app.data_rows()["doubled"][0] == "Number"

    expect(card.locator(".map-latest-from .map-latest-label")).to_have_text(
        ["temp", ""]
    )
    (value, constant), result = latest_numbers(card)
    assert constant == "2.0000"
    assert result == pytest.approx(float(value) * 2, rel=1e-4)


def test_pow_raises_a_to_the_power_of_b(app: App):
    app.tab("Data")
    card = add_arithmetic_map(app, "Pow", "2", "10", "power")
    expect(card.locator(".map-formula .katex-mathml annotation")).to_have_text("a^b")
    app.wait_for_labels("power")
    assert latest_numbers(card)[1] == 1024

    # 1 / 0: no error, as Rust's `f64` has it
    card = add_arithmetic_map(app, "Pow", "0", "-1", "inverse")
    app.wait_for_labels("inverse")
    assert app.data_rows()["inverse"][1] == "inf"
    expect(card.locator(".field-error")).to_have_count(0)


def test_an_output_label_of_a_hash_is_an_error(app: App):
    app.tab("Data")
    card = add_arithmetic_map(app, "Add", "1", "2", "#sum")
    failed = card.locator('.map-latest[data-failed="true"]')
    expect(failed).to_have_text("A label can't start with '#': '#sum'")
    assert "#sum" not in app.data_rows()

    # Gone as it is turned on again, with a label it can take
    card.get_by_role("switch").click()
    card.get_by_placeholder("Output label").fill("sum")
    card.get_by_role("switch").click()
    app.wait_for_labels("sum")
    expect(failed).to_have_count(0)


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


def test_match_preset_adds_a_set_map(app: App):
    app.tab("Data")
    item = app.map_item("Number")
    item.hover()
    item.get_by_role("menuitem").filter(has_text="Teleplot").click()

    expect(app.map_cards()).to_have_count(4)
    card = app.map_cards().last
    expect(card.get_by_placeholder("Text to be matched")).to_have_value(
        ">({word}):({number})"
    )
    expect(card.get_by_placeholder("Output label")).to_have_value("$1")
    expect(card.get_by_label("To", exact=True)).to_have_value("$2")
    expect(card.locator(".map-types")).to_have_text("LogNumber")


def test_value_preset_leaves_the_output_label_to_be_set(app: App):
    app.tab("Data")
    item = app.map_item("Number")
    item.hover()
    item.get_by_role("menuitem").filter(
        has=app.page.locator(".add-map-preset-name", has_text=re.compile("^value$"))
    ).click()

    card = app.map_cards().last
    expect(card.get_by_placeholder("Text to be matched")).to_have_value("^{number}")
    expect(card.get_by_placeholder("Output label")).to_have_value("")
    expect(card.get_by_label("To", exact=True)).to_have_value("$0")


def test_match_presets_by_keyboard(app: App):
    app.tab("Data")
    item = app.map_item("Number")
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
        "({word}): ({number})"
    )


def test_differentiate_and_integrate(app: App):
    app.tab("Data")
    app.load_layout(
        [
            name_value_map(),
            saved_map(
                "Differentiate",
                ["Number"],
                "Number",
                from_label="temp",
                to_label="temp_rate",
            ),
            saved_map(
                "Integrate",
                ["Number"],
                "Number",
                from_label="temp",
                to_label="temp_total",
            ),
        ]
    )
    derivative = app.map_cards().nth(-2)
    expect(derivative.locator(".map-title-text")).to_have_text("Differentiate")
    # Its readings, each 50 ms apart, as the clock moves on
    app.stop_clock()
    app.open_port()
    app.run_clock_until(lambda: {"temp_rate", "temp_total"} <= app.data_rows().keys())
    rows = app.data_rows()
    assert rows["temp_rate"][0] == "Number"
    assert rows["temp_total"][0] == "Number"

    first = float(app.data_rows()["temp_total"][1])
    app.page.clock.run_for(500)
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
    # Its height each frame as it opens: some between none and all, however
    # busy the machine (a height at a set time might be all already)
    content.evaluate(
        """el => {
            window.heights = [];
            const start = performance.now();
            const frame = () => {
                window.heights.push(el.getBoundingClientRect().height);
                if (performance.now() - start < 1000) requestAnimationFrame(frame);
            };
            requestAnimationFrame(frame);
        }"""
    )
    card.locator(".map-title").click()
    expect(field).to_be_visible()
    app.page.wait_for_timeout(1100)
    heights = app.page.evaluate("window.heights")
    full = height()
    assert full > 1 and heights[-1] == full
    assert any(0 < each < full for each in heights)

    card.locator(".map-title").click()
    expect(field).to_be_hidden()
    app.page.wait_for_function(
        "el => el.getBoundingClientRect().height < 1", arg=content.element_handle()
    )


def map_titles(app: App) -> list[str]:
    return app.map_cards().locator(".map-title-text").all_inner_texts()


def map_items(app: App):
    return app.page.locator(".map-list li[aria-roledescription='sortable item']")


def test_maps_are_reordered_by_dragging(app: App):
    app.tab("Data")
    app.add_map("Replace", "Log", "Log")
    expect(app.map_cards()).to_have_count(4)
    assert map_titles(app) == ["Decode", "Match", "Match", "Replace"]

    items = map_items(app)
    # Not while it is open
    replace = app.map_cards().nth(3)
    items.nth(3).locator("> svg").drag_to(
        items.nth(0), target_position={"x": 40, "y": 4}
    )
    app.page.wait_for_timeout(300)
    assert map_titles(app) == ["Decode", "Match", "Match", "Replace"]
    expect(items.nth(3).locator("> svg")).to_have_css("opacity", "0.3")
    replace.locator(".map-title").click()
    expect(items.nth(3).locator("> svg")).to_have_css("opacity", "1")
    items.nth(3).locator("> svg").drag_to(
        items.nth(0), target_position={"x": 40, "y": 4}
    )
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Replace", "Decode", "Match", "Match"]
    )
    # Kept by the maps, not the list: so as it is made anew
    app.tab("Console")
    app.tab("Data")
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Replace", "Decode", "Match", "Match"]
    )
    # A map added after them comes last
    app.add_map("Replace", "Log", "Log")
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Replace", "Decode", "Match", "Match", "Replace"]
    )


def test_maps_are_reordered_by_keyboard(app: App):
    app.tab("Data")
    app.add_map("Replace", "Log", "Log")
    expect(app.map_cards()).to_have_count(4)
    map_items(app).nth(3).focus()
    for key in ["Enter", "ArrowUp", "ArrowUp", "Enter"]:
        app.page.keyboard.press(key)
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Decode", "Replace", "Match", "Match"]
    )
    # Delete on the item leaves the map: its card's button deletes it
    map_items(app).nth(1).focus()
    app.page.keyboard.press("Delete")
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Decode", "Replace", "Match", "Match"]
    )


def test_keys_typed_in_a_card_stay_in_it(app: App):
    app.tab("Data")
    app.add_map("Replace", "Log", "Log")
    card = app.map_cards().last
    field = card.get_by_placeholder("Input label")
    field.click()
    app.page.keyboard.type("my label")
    # Those the list takes to reorder, and to remove
    for key in ["ArrowUp", "Enter", " ", "Home", "End", "Backspace", "Delete"]:
        app.page.keyboard.press(key)
    # As in any field: the space at the start, Backspace at the end
    expect(field).to_have_value(" my labe")
    expect(app.map_cards().locator(".map-title-text")).to_have_text(
        ["Decode", "Match", "Match", "Replace"]
    )


def add_compose_map(app: App, variant: str, components: list[str], target: str):
    """Adds a Compose map, its `variant` (2D or 3D) picked beside it."""
    item = app.map_item("Vector", "Compose")
    item.hover()
    item.get_by_role("menuitem", name=variant).click()
    # This one, not whichever is last when it is used
    card = app.map_cards().nth(app.map_cards().count() - 1)
    for name, label in zip("xyz", components):
        card.get_by_label(name, exact=True).fill(label)
    card.get_by_placeholder("Output label").fill(target)
    card.get_by_role("switch").click()
    return card


def submenu_side(app: App, output: str, title: str) -> str:
    item = app.map_item(output, title)
    item.hover()
    submenu = item.locator("> .add-map-presets")
    expect(submenu).to_be_visible()
    item_box, submenu_box = item.bounding_box(), submenu.bounding_box()
    width = app.page.viewport_size["width"]
    assert submenu_box["x"] >= 0 and submenu_box["x"] + submenu_box["width"] <= width
    if submenu_box["x"] >= item_box["x"] + item_box["width"] - 1:
        return "right"
    assert submenu_box["x"] + submenu_box["width"] <= item_box["x"] + 1
    return "left"


def test_submenus_open_to_the_right_unless_past_the_window(app: App):
    app.page.set_viewport_size({"width": 2560, "height": 800})
    app.tab("Data")
    assert submenu_side(app, "Number", "Match") == "right"
    assert submenu_side(app, "Vector", "Compose") == "right"

    app.page.set_viewport_size({"width": 1280, "height": 800})
    assert submenu_side(app, "Vector", "Compose") == "left"


def vector_variant(app: App, title: str, variant: str, menu: str = "Vector"):
    """`variant` (2D or 3D) beside the `title` item of `menu`, pointed at."""
    item = app.map_item(menu, title)
    item.hover()
    variant = item.locator(".add-map-variant").filter(
        has=app.page.locator(
            "> [role=menuitem] .add-map-preset-name",
            has_text=re.compile(f"^{variant}$"),
        )
    )
    variant.hover()
    return variant


def add_vector_arithmetic_map(
    app: App, title: str, variant: str, first: str, second: str, target: str
):
    """Adds the vector map `title`, its `variant` (2D or 3D) picked beside it."""
    vector_variant(app, title, variant).locator("> [role=menuitem]").click()
    # This one, not whichever is last when it is used
    card = app.map_cards().nth(app.map_cards().count() - 1)
    card.get_by_label("a", exact=True).fill(first)
    card.get_by_label("b", exact=True).fill(second)
    card.get_by_placeholder("Output label").fill(target)
    card.get_by_role("switch").click()
    return card


def wait_for_latest(app: App, label: str, latest: str):
    app.page.wait_for_function(
        """([label, latest]) => [...document.querySelectorAll('.data-row')].some(row =>
            row.children[0].textContent === label && row.children[2].title === latest)""",
        arg=[label, latest],
    )


def test_vector_maps_in_the_menus(app: App):
    app.tab("Data")
    trigger = app.page.get_by_role("button", name="Vector", exact=True)
    menu = app.page.locator(".add-map-bar .hover-menu").filter(has=trigger)
    trigger.hover()
    options = menu.get_by_role("option")
    # No Divide
    expect(options.locator(".add-map-title")).to_have_text(
        [
            "Match",
            "Compose",
            "Add",
            "Subtract",
            "Multiply",
            "Cross",
            "Differentiate",
            "Integrate",
        ]
    )
    expect(options.locator("> .map-types")).to_have_text(
        [
            "LogVector",
            "NumberVector",
            "VectorVector",
            "VectorVector",
            "NumberVectorVector",
            # 3D alone, so as it is
            "Vector3dVector3dVector3d",
            "VectorVector",
            "VectorVector",
        ]
    )

    item = app.map_item("Vector", "Compose")
    item.hover()
    expect(item.get_by_role("menuitem")).to_have_text(
        ["2DNumberNumberVector2d", "3DNumberNumberNumberVector3d"]
    )
    # Its inputs' types one above another
    types = item.get_by_role("menuitem").last.locator(".map-types-from > span")
    boxes = [types.nth(index).bounding_box() for index in range(3)]
    assert boxes[0]["x"] == boxes[1]["x"] == boxes[2]["x"]
    assert boxes[0]["y"] < boxes[1]["y"] < boxes[2]["y"]

    # Match's presets beside each dimension, not before it is pointed at
    item = app.map_item("Vector", "Match")
    item.hover()
    expect(
        item.locator(".add-map-variant > [role=menuitem] .add-map-preset-name")
    ).to_have_text(["2D", "3D"])
    expect(item.locator(".add-map-variant .add-map-presets")).to_have_count(2)
    expect(item.locator(".add-map-variant .add-map-presets").first).to_be_hidden()
    for variant, presets in [
        ("2D", ["(x, y)", "name: (x, y)"]),
        ("3D", ["(x, y, z)", "name: (x, y, z)"]),
    ]:
        nested = vector_variant(app, "Match", variant).locator(".add-map-presets")
        expect(nested).to_be_visible()
        expect(nested.locator(".add-map-preset-name")).to_have_text(presets)

    item = app.map_item("Number", "Dot")
    item.hover()
    expect(item.locator("> .map-types")).to_have_text("VectorNumber")
    expect(
        item.locator(".add-map-variant > [role=menuitem] .add-map-preset-name")
    ).to_have_text(["2D", "3D"])


def test_vector_presets_by_keyboard(app: App):
    app.tab("Data")
    item = app.map_item("Vector", "Match")
    item.focus()
    focused = lambda: app.page.evaluate("document.activeElement.textContent")  # noqa: E731
    app.page.keyboard.press("ArrowRight")
    assert focused().startswith("2D")
    app.page.keyboard.press("ArrowDown")
    assert focused().startswith("3D")
    app.page.keyboard.press("ArrowRight")
    assert focused().startswith("(x, y, z)")
    app.page.keyboard.press("ArrowDown")
    assert focused().startswith("name: (x, y, z)")
    app.page.keyboard.press("ArrowLeft")
    assert focused().startswith("3D")
    app.page.keyboard.press("ArrowLeft")
    assert focused().startswith("Match")


def test_vectors_are_composed(app: App):
    app.tab("Data")
    app.add_match_map()
    compose = add_compose_map(app, "2D", ["temp", "volt"], "p")
    # Constants too
    add_compose_map(app, "3D", ["temp", "volt", "12"], "v")
    add_compose_map(app, "2D", ["1.5", "-2"], "fixed")
    add_compose_map(app, "3D", ["temp", "volt", "temp"], "twice")
    # Each tells what it cannot take
    bad_compose = add_compose_map(app, "2D", ["nope", "1"], "bad_compose")
    bad_add = add_vector_arithmetic_map(
        app, "Add", "2D", "(1, 2, 3)", "(1, 1)", "bad_add"
    )
    expect(bad_compose.locator(".field-error")).to_have_text(
        "No Number label or number 'nope'"
    )
    expect(bad_add.locator(".field-error")).to_have_text(
        "No Vector2d label or vector '(1, 2, 3)'"
    )
    # Of named constants
    add_vector_arithmetic_map(app, "Add", "2D", "(#pi, 0)", "(0, #e)", "named")
    wait_for_latest(app, "named", "(3.1416, 2.7183)")

    app.open_port()
    app.mock("mute()")
    app.mock("receive('temp:3\\nvolt:4\\n')")
    wait_for_latest(app, "p", "(3.0000, 4.0000)")
    wait_for_latest(app, "v", "(3.0000, 4.0000, 12.000)")
    wait_for_latest(app, "twice", "(3.0000, 4.0000, 3.0000)")
    app.wait_for_labels("fixed")
    rows = app.data_rows()
    assert rows["p"][0] == "Vector2d"
    assert rows["v"][0] == "Vector3d"
    assert rows["fixed"][1] == "(1.5000, -2.0000)"
    expect(compose.locator(".map-latest")).to_contain_text("p")


def test_vectors_are_matched(app: App):
    app.tab("Data")
    matched = app.add_vector_match_map(
        "2D", r"pos:({number}),({number})", "position", ["$1", "$2"]
    )
    # Labelled by a group
    app.add_vector_match_map(
        "3D", r"({word})=({number}),({number}),({number})", "$1", ["$2", "$3", "$4"]
    )
    vector_variant(app, "Match", "3D").get_by_role(
        "menuitem", name="name: (x, y, z)"
    ).click()
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("log")
    expect(card.get_by_placeholder("Output label")).to_have_value("$1")
    expect(card.get_by_label("z", exact=True)).to_have_value("$4")
    card.get_by_role("switch").click()
    vector_variant(app, "Match", "2D").get_by_role(
        "menuitem", name="(x, y)"
    ).first.click()
    card = app.map_cards().last
    card.get_by_placeholder("Input label").fill("log")
    expect(card.get_by_label("y", exact=True)).to_have_value("$2")
    # No name in the text to take the output label from
    card.get_by_placeholder("Output label").fill("pt")
    card.get_by_role("switch").click()
    bad = app.add_vector_match_map("2D", r"(led):({number})", "bad_match", ["$2", "$1"])

    app.open_port()
    app.mock("mute()")
    app.mock(
        "receive('pos:1.5,-2\\nacc=1,2,3\\ngyro=-1,0,0.5\\n"
        "mag: (1, 2, 3)\\nmag2:(4,5,6)\\n(1.5,-2)\\nled:1\\n')"
    )
    app.wait_for_labels("position", "acc", "gyro", "mag", "mag2", "pt")
    rows = app.data_rows()
    assert rows["position"][:2] == ["Vector2d", "(1.5000, -2.0000)"]
    assert rows["acc"][:2] == ["Vector3d", "(1.0000, 2.0000, 3.0000)"]
    assert rows["gyro"][:2] == ["Vector3d", "(-1.0000, 0.0000, 0.50000)"]
    assert rows["mag"][:2] == ["Vector3d", "(1.0000, 2.0000, 3.0000)"]
    assert rows["mag2"][:2] == ["Vector3d", "(4.0000, 5.0000, 6.0000)"]
    assert rows["pt"][:2] == ["Vector2d", "(1.5000, -2.0000)"]
    expect(matched.locator(".map-latest")).to_contain_text("(1.5000, -2.0000)")
    expect(bad.locator(".field-error")).to_have_text("The result 'led' is not a number")


def p_and_v_maps() -> list[dict]:
    """p = (temp, volt) and v = (temp, volt, 12), of numbers yet to come."""
    return [
        name_value_map(),
        compose_map(to_label="p", x="temp", y="volt"),
        compose_map(to_label="v", x="temp", y="volt", z="12"),
    ]


def compose_map(**settings) -> dict:
    """A Compose map of `settings`, 3D if it has a z."""
    sources = ["Number"] * (3 if "z" in settings else 2)
    output = "Vector3d" if "z" in settings else "Vector2d"
    return saved_map("Compose", sources, output, **settings)


def receive_p_and_v(app: App):
    """p = (3, 4) and v = (3, 4, 12)."""
    app.open_port()
    app.mock("mute()")
    app.mock("receive('temp:3\\nvolt:4\\n')")
    wait_for_latest(app, "p", "(3.0000, 4.0000)")


def test_vector_maps_compute_vectors(app: App):
    app.tab("Data")
    app.load_layout(
        p_and_v_maps()
        + [
            saved_map("Add", V2, "Vector2d", first="p", second="(1, 1)", to_label="q"),
            saved_map("Subtract", V2, "Vector2d", first="q", second="p", to_label="r"),
            # The number first, as `a`
            saved_map(
                "Multiply",
                ["Number", "Vector3d"],
                "Vector3d",
                first="temp",
                second="v",
                to_label="w",
            ),
            saved_map(
                "Multiply",
                ["Number", "Vector2d"],
                "Vector2d",
                first="0.5",
                second="(4, -2)",
                to_label="half",
            ),
            saved_map(
                "Cross",
                V3,
                "Vector3d",
                first="v",
                second="(0, 0, 1)",
                to_label="v_cross",
            ),
        ]
    )
    multiply = app.map_cards().nth(-3)

    receive_p_and_v(app)
    wait_for_latest(app, "q", "(4.0000, 5.0000)")
    wait_for_latest(app, "r", "(1.0000, 1.0000)")
    wait_for_latest(app, "w", "(9.0000, 12.000, 36.000)")
    wait_for_latest(app, "half", "(2.0000, -1.0000)")
    wait_for_latest(app, "v_cross", "(4.0000, -3.0000, 0.0000)")
    rows = app.data_rows()
    assert rows["r"][0] == "Vector2d"
    assert rows["v_cross"][0] == "Vector3d"
    expect(multiply.locator(".map-latest")).to_contain_text("temp")


def test_vector_maps_compute_numbers(app: App):
    app.tab("Data")
    app.load_layout(
        p_and_v_maps()
        + [
            saved_map(
                "Dot", V2, "Number", first="p", second="(1, 2)", to_label="p_dot"
            ),
            saved_map(
                "Dot", V3, "Number", first="v", second="(1, 1, 1)", to_label="v_dot"
            ),
            saved_map(
                "Cross", V2, "Number", first="p", second="(1, 0)", to_label="p_cross"
            ),
            saved_map(
                "Magnitude", ["Vector2d"], "Number", from_label="p", to_label="p_length"
            ),
            saved_map(
                "Magnitude", ["Vector3d"], "Number", from_label="v", to_label="v_length"
            ),
            # y left out
            saved_map(
                "Decompose",
                ["Vector3d"],
                "Number",
                from_label="v",
                x_label="v_x",
                y_label="",
                z_label="v_z",
            ),
        ]
    )
    length, decompose = app.map_cards().nth(-2), app.map_cards().last
    expect(decompose.get_by_placeholder("Output label")).to_have_count(3)

    receive_p_and_v(app)
    wait_for_latest(app, "p_dot", "11.000")
    wait_for_latest(app, "v_dot", "19.000")
    # (3, 4) × (1, 0): its z
    wait_for_latest(app, "p_cross", "-4.0000")
    wait_for_latest(app, "p_length", "5.0000")
    wait_for_latest(app, "v_length", "13.000")
    wait_for_latest(app, "v_x", "3.0000")
    wait_for_latest(app, "v_z", "12.000")
    rows = app.data_rows()
    assert [rows[label][0] for label in ("p_dot", "p_cross", "v_length", "v_z")] == [
        "Number"
    ] * 4
    assert "v_y" not in rows
    expect(length.locator(".map-latest")).to_contain_text("13.000")
    # As the inputs: each label beside its value, one above another
    latest = decompose.locator(".map-latest-to")
    expect(latest.locator(".map-latest-label")).to_have_text(["v_x", "v_z"])
    expect(latest.locator(".map-latest-value")).to_have_text(["3.0000", "12.000"])
    x, z = (
        latest.locator(".map-latest-label").nth(index).bounding_box()
        for index in (0, 1)
    )
    value = latest.locator(".map-latest-value").first.bounding_box()
    assert z["y"] > x["y"] and value["x"] > x["x"] and abs(value["y"] - x["y"]) < 1


def test_vectors_differentiate_and_integrate_by_component(app: App):
    app.tab("Data")
    app.load_layout(
        [
            name_value_map(),
            compose_map(to_label="p", x="temp", y="volt"),
            compose_map(to_label="v", x="temp", y="volt", z="1"),
            saved_map(
                "Differentiate", ["Vector2d"], "Vector2d", from_label="p", to_label="dp"
            ),
            saved_map(
                "Integrate", ["Vector3d"], "Vector3d", from_label="v", to_label="iv"
            ),
        ]
    )
    app.stop_clock()
    app.open_port()
    app.run_clock_until(lambda: {"dp", "iv"} <= app.data_rows().keys())
    rows = app.data_rows()
    assert rows["dp"][0] == "Vector2d"
    assert re.fullmatch(r"\(-?[\d.e]+, -?[\d.e]+\)", rows["dp"][1])
    assert rows["iv"][0] == "Vector3d"
    # The integral of z = 1: the seconds the clock moved on
    z = lambda: float(re.fullmatch(r"\(.+, .+, (.+)\)", app.data_rows()["iv"][1])[1])  # noqa: E731
    first = z()
    app.page.clock.run_for(1000)
    assert z() - first == pytest.approx(1, abs=0.1)


def choose_weakest(card, level: str, input: int = 0):
    """Takes the levels of the card's `input`th input down to `level`."""
    # Its choices as the pointer is over it
    card.get_by_role("button", name="Weakest level").nth(input).hover()
    card.get_by_role("option", name=level, exact=True).click()
    expect(card.get_by_role("button", name="Weakest level").nth(input)).to_have_text(
        f"Filter by{level}"
    )


def test_maps_of_logs_take_all_levels_by_default(app: App):
    app.tab("Data")
    app.add_map("Match", "Log", "Number")
    card = app.map_cards().last
    weakest = card.get_by_role("button", name="Weakest level")
    # The weakest of all: so all of them
    expect(weakest).to_have_text("Filter byALL")
    # At the end of the input label's field, after what it is
    field = card.locator(".field").filter(
        has=app.page.get_by_placeholder("Input label")
    )
    expect(field.locator(".field-level")).to_have_text("Filter byALL")
    # Side by side, as the pointer is over it; gone as it leaves
    expect(card.get_by_role("option")).to_have_count(0)
    weakest.hover()
    options = card.get_by_role("option")
    expect(options).to_have_text(["ERROR", "WARN", "INFO", "DEBUG", "TRACE", "ALL"])
    first, last = options.first.bounding_box(), options.last.bounding_box()
    assert abs(first["y"] - last["y"]) < 1 and first["x"] < last["x"]
    expect(options.locator(".log-level-tag")).to_have_count(5)
    # Within the map list, which clips what goes past it
    within = app.page.locator(".map-list").bounding_box()
    for option in (first, last):
        assert within["x"] <= option["x"]
        assert option["x"] + option["width"] <= within["x"] + within["width"]
    app.page.mouse.move(0, 0)
    expect(card.get_by_role("option")).to_have_count(0)


def test_concat_leaves_out_weaker_levels_of_each_input(app: App):
    app.tab("Data")
    # Each input of its own lines, so each new value is of one alone (with both of
    # one line, which pairs first would be up to the order the maps run in)
    app.add_log_match("^first (.*)", "${level:0}", "$1", "first")
    app.add_log_match("^second (.*)", "${level:0}", "$1", "second")
    app.add_map("Concat", "Log", "Log")
    card = app.map_cards().nth(app.map_cards().count() - 1)
    card.get_by_placeholder("First input label").fill("first")
    card.get_by_placeholder("Second input label").fill("second")
    expect(card.get_by_role("button", name="Weakest level")).to_have_count(2)
    choose_weakest(card, "ERROR")
    card.get_by_placeholder("Output label").fill("joined")
    card.get_by_role("switch").click()
    app.open_port()
    app.mock("mute()")
    # Weaker than the first's: left out
    app.mock("receive('[info] first a\\n')")
    app.mock("receive('[info] second x\\n')")
    wait_for_latest(app, "second", "INFO x")
    wait_for_latest(app, "first", "INFO a")
    app.page.wait_for_timeout(300)
    assert "joined" not in app.data_rows()
    app.mock("receive('[error] first b\\n')")
    # The second's taken whatever its level, with the first's
    wait_for_latest(app, "joined", "ERROR bx")


def test_text_in_an_open_card_is_chosen_by_the_pointer(app: App):
    app.tab("Data")
    app.add_map("Replace", "Log", "Log")
    card = app.map_cards().nth(app.map_cards().count() - 1)
    field = card.get_by_placeholder("Text to be replaced")
    field.fill("hello world")
    box = field.bounding_box()
    mouse = app.page.mouse
    # Across the text, from its start: a choice of it, not a drag of the card
    mouse.move(box["x"] + 2, box["y"] + box["height"] / 2)
    mouse.down()
    mouse.move(box["x"] + box["width"] - 2, box["y"] + box["height"] / 2, steps=10)
    mouse.up()
    chosen = field.evaluate("f => f.value.slice(f.selectionStart, f.selectionEnd)")
    assert chosen == "hello world"
    assert map_titles(app) == ["Decode", "Match", "Match", "Replace"]


def test_latest_values_are_cut_off_at_30_characters(app: App):
    app.tab("Data")
    card = app.map_cards().first
    app.open_port()
    app.mock("mute()")
    app.mock("receive('" + "x" * 80 + "\\n')")
    value = card.locator(".map-latest-to .map-latest-value")
    expect(value).to_contain_text("x" * 80)
    # Shown up to 30 characters' width, its whole text kept
    sizes = value.evaluate(
        """v => {
            const ch = document.createElement('span');
            ch.style.font = getComputedStyle(v).font;
            ch.textContent = '0'.repeat(30);
            document.body.append(ch);
            const width = ch.getBoundingClientRect().width;
            ch.remove();
            return [v.clientWidth, v.scrollWidth, width];
        }"""
    )
    shown, whole, thirty = sizes
    assert shown <= thirty + 1 and whole > shown
