import json
import re

import pytest
from playwright.sync_api import expect

from app import App, name_value_map

GRID_COLUMNS = (
    "getComputedStyle(document.querySelector('.graph-grid'))"
    ".gridTemplateColumns.split(' ').length"
)


def open_graphs_with_numbers(app: App):
    app.open_port()
    app.tab("Data")
    app.add_match_map()
    app.wait_for_labels("temp", "volt")
    app.tab("Graph")


def test_plots_the_labels_the_data_list_shows(app: App):
    open_graphs_with_numbers(app)
    app.set_up_graph(0, "Readings")
    app.wait_for_graph_legends([["temp", "volt"]])
    expect(app.graphs().first.locator(".graph-title-text")).to_have_text("Readings")
    app.graphs().first.locator(".graph-title").click()
    expect(
        app.graphs().first.locator(".graph-settings-body [role=grid]")
    ).to_have_count(0)

    app.page.get_by_role("button", name="Toggle data list").click()
    board = app.page.locator(".graph-board")
    board.get_by_placeholder("Label filter").fill("temp")
    board.get_by_role("button", name="Add filter").click()
    app.wait_for_graph_legends([["temp"]])

    board.get_by_placeholder("Label filter").fill("nothing")
    board.get_by_role("button", name="Add filter").click()
    board.get_by_role("button", name="Remove Show filter temp").click(force=True)
    expect(app.graphs().first.locator(".graph-empty")).to_contain_text("filters")


def test_graphs_survive_switching_tabs(app: App):
    open_graphs_with_numbers(app)
    app.add_graph()
    app.add_graph()
    titles = ["Temp", "Volt", "Both"]
    for index, title in enumerate(titles):
        app.set_up_graph(index, title)
    legends = [["temp", "volt"]] * 3
    app.wait_for_graph_legends(legends)
    assert app.page.evaluate(GRID_COLUMNS) == 2

    app.tab("Console")
    app.tab("Graph")
    app.wait_for_graph_legends(legends)
    expect(app.graphs().locator(".graph-title-text")).to_have_text(titles)


def test_removing_a_graph_keeps_the_others(app: App):
    open_graphs_with_numbers(app)
    app.add_graph()
    app.add_graph()
    for index, title in enumerate(["Temp", "Volt", "Both"]):
        app.set_up_graph(index, title)

    middle = app.graphs().nth(1)
    middle.hover()  # the button shows on hover
    middle.get_by_role("button", name="Remove graph").click()

    expect(app.graphs().locator(".graph-title-text")).to_have_text(["Temp", "Both"])
    app.wait_for_graph_legends([["temp", "volt"]] * 2)
    expect(app.page.locator(".uplot")).to_have_count(2)


def test_clearing_data_from_the_sidebar_keeps_plotting(app: App):
    open_graphs_with_numbers(app)
    app.wait_for_graph_legends([["temp", "volt"]])

    app.page.get_by_role("button", name="Toggle data list").click()
    app.clear_all(".graph-board")

    app.wait_for_labels("temp")
    app.wait_for_graph_legends([["temp", "volt"]])


def test_value_axis_can_be_linear_or_log(app: App):
    open_graphs_with_numbers(app)
    graph = app.graphs().first
    plot = graph.locator(".graph-plot")
    expect(plot).to_have_attribute("data-value-scale", "linear")

    graph.locator(".graph-title").click()  # opens the settings
    scale = graph.get_by_role("group", name="Value axis")
    linear = scale.get_by_role("button", name="Linear")
    log = scale.get_by_role("button", name="Log")
    expect(linear).to_have_attribute("aria-pressed", "true")
    log.click()
    expect(log).to_have_attribute("aria-pressed", "true")
    expect(linear).to_have_attribute("aria-pressed", "false")
    expect(plot).to_have_attribute("data-value-scale", "log")
    app.wait_for_graph_legends([["temp", "volt"]])

    app.tab("Console")
    app.tab("Graph")
    expect(app.graphs().first.locator(".graph-plot")).to_have_attribute(
        "data-value-scale", "log"
    )


def test_values_can_be_drawn_as_points_lines_or_steps(app: App):
    open_graphs_with_numbers(app)
    graph = app.graphs().first
    plot = graph.locator(".graph-plot")
    expect(plot).to_have_attribute("data-draw-style", "linear")

    graph.locator(".graph-title").click()  # opens the settings
    draw = graph.get_by_role("group", name="Draw")
    expect(draw.get_by_role("button")).to_have_text(["Points", "Linear", "Stepped"])
    expect(draw.get_by_role("button", name="Linear")).to_have_attribute(
        "aria-pressed", "true"
    )
    for name, style in [
        ("Points", "points"),
        ("Stepped", "stepped"),
        ("Linear", "linear"),
    ]:
        draw.get_by_role("button", name=name).click()
        expect(draw.get_by_role("button", name=name)).to_have_attribute(
            "aria-pressed", "true"
        )
        expect(plot).to_have_attribute("data-draw-style", style)
        app.wait_for_graph_legends([["temp", "volt"]])


def test_time_range_fits_the_data_up_to_its_seconds(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    # Of a alone: none of the readings before it was muted
    app.add_match_map(pattern=r"(a):([\d.]+)")
    app.stop_clock()
    # Two values, 25 s apart: of the shortest time range, and the next
    app.mock("receive('a:1\\n')")
    app.run_clock_until(lambda: "a" in app.data_rows())
    app.page.clock.run_for(25_000)
    app.mock("receive('a:2\\n')")
    app.tab("Graph")
    app.add_graph()
    graph = app.graphs().last

    def span() -> float:
        return float(graph.locator(".graph-plot").get_attribute("data-time-span") or 0)

    graph.locator(".graph-title").click()  # opens the settings
    slider = graph.get_by_role("slider", name="Time range")
    expect(slider).to_have_attribute("aria-valuemin", "10")
    expect(slider).to_have_attribute("aria-valuemax", "300")
    expect(slider).to_have_attribute("aria-valuenow", "10")
    value = graph.locator(".graph-time-range-value")
    expect(value).to_have_text("10 s")
    app.run_clock_until(lambda: 9.99 <= span() <= 10.01)

    # 30 s: the data, as short as it is
    slider.focus()
    app.page.keyboard.press("ArrowRight")
    app.page.keyboard.press("ArrowRight")
    expect(value).to_have_text("30 s")
    app.run_clock_until(lambda: 25 <= span() <= 26)
    app.page.keyboard.press("ArrowLeft")
    expect(value).to_have_text("20 s")
    app.run_clock_until(lambda: 19.99 <= span() <= 20.01)

    # All the time: all the data, the slider off
    all_time = graph.get_by_role("button", name="All")
    all_time.click()
    expect(all_time).to_have_attribute("aria-pressed", "true")
    expect(slider).to_be_disabled()
    expect(value).to_have_attribute("data-off", "true")
    app.run_clock_until(lambda: 25 <= span() <= 26)
    # Off again: its time range as it was
    all_time.click()
    expect(slider).to_be_enabled()
    app.run_clock_until(lambda: 19.99 <= span() <= 20.01)

    app.tab("Console")
    app.tab("Graph")
    graph = app.graphs().last
    app.run_clock_until(lambda: 19.99 <= span() <= 20.01)


def test_cursor_moves_together_across_graphs(app: App):
    open_graphs_with_numbers(app)
    app.add_graph()
    app.wait_for_graph_legends([["temp", "volt"]] * 2)
    first, second = app.graphs().nth(0), app.graphs().nth(1)

    def legend_values(graph):
        return graph.locator(".u-legend .u-value").all_text_contents()

    def cursor_offset(graph):
        return graph.locator(".u-over").evaluate(
            "over => over.parentElement.querySelector('.u-cursor-x').getBoundingClientRect().left"
            " - over.getBoundingClientRect().left"
        )

    over = first.locator(".u-over")
    box = over.bounding_box()
    app.page.mouse.move(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
    expect(second.locator(".u-legend .u-value").first).not_to_have_text("--")
    assert all(value != "--" for value in legend_values(second))
    assert abs(cursor_offset(first) - cursor_offset(second)) < 3


def test_settings_and_legend_toggles_stay(app: App):
    open_graphs_with_numbers(app)
    app.add_graph()
    app.wait_for_graph_legends([["temp", "volt"]] * 2)

    def turned_off(graph):
        return graph.locator(".u-legend .u-series.u-off th").all_text_contents()

    def check():
        first, second = app.graphs().nth(0), app.graphs().nth(1)
        expect(first.locator(".u-legend .u-series.u-off")).to_have_count(1)
        assert [label.strip() for label in turned_off(first)] == ["temp"]
        expect(second.locator(".u-legend .u-series.u-off")).to_have_count(0)
        expect(first.locator(".graph-title-text")).to_have_text("Readings")
        plot = first.locator(".graph-plot")
        expect(plot).to_have_attribute("data-value-scale", "log")
        expect(plot).to_have_attribute("data-draw-style", "points")

    first = app.graphs().first
    app.set_up_graph(0, "Readings")
    first.locator(".graph-title").click()  # opens the settings
    first.get_by_role("group", name="Value axis").get_by_role(
        "button", name="Log"
    ).click()
    first.get_by_role("group", name="Draw").get_by_role("button", name="Points").click()
    first.locator(".graph-title").click()  # closes them
    first.locator(".u-legend .u-series th", has_text="temp").click()
    check()

    app.tab("Console")
    app.tab("Graph")
    app.wait_for_graph_legends([["temp", "volt"]] * 2)
    check()

    app.page.get_by_role("button", name="Toggle data list").click()
    app.clear_all(".graph-board")
    app.wait_for_labels("temp", "volt")
    app.wait_for_graph_legends([["temp", "volt"]] * 2)
    check()


def test_adds_time_series_graphs_from_their_presets(app: App):
    open_graphs_with_numbers(app)
    bar = app.page.locator(".add-graph-bar")
    trigger = bar.get_by_role("button", name="Time series")
    trigger.hover()
    expect(trigger).to_have_attribute("aria-expanded", "true")
    expect(bar.get_by_role("option")).to_have_text(["Points", "Linear", "Stepped"])

    for preset in ["Points", "Linear", "Stepped"]:
        app.add_graph(preset)
    styles = app.graphs().locator(".graph-plot")
    expect(styles).to_have_count(4)
    for index, style in enumerate(["points", "linear", "stepped"], start=1):
        expect(styles.nth(index)).to_have_attribute("data-draw-style", style)


def test_legend_shows_the_time_as_hh_mm_ss_sss(app: App):
    import re

    open_graphs_with_numbers(app)
    app.wait_for_graph_legends([["temp", "volt"]])
    over = app.graphs().first.locator(".u-over")
    box = over.bounding_box()
    app.page.mouse.move(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
    time = app.graphs().first.locator(".u-legend .u-series").first.locator(".u-value")
    expect(time).to_have_text(re.compile(r"^\d{2}:\d{2}:\d{2}\.\d{3}$"))


def test_statistics_menu_adds_a_scatter_plot(app: App):
    app.tab("Graph")
    bar = app.page.locator(".add-graph-bar")
    expect(bar.get_by_role("button")).to_have_text(
        ["Time series", "Statistics", "Vector", "Text"]
    )
    bar.get_by_role("button", name="Statistics").hover()
    expect(bar.get_by_role("option")).to_have_text(["Box plot", "Scatter"])
    graph = app.add_scatter()
    expect(app.graphs()).to_have_count(2)
    expect(graph.locator(".graph-title-text")).to_have_text("Graph 2")
    expect(graph.locator(".graph-empty")).to_have_text(
        "No number data yet. Add a map to a number to plot it."
    )


def test_scatter_plots_one_label_against_another(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_match_map()
    app.mock("receive('a:1\\nb:10\\na:2\\nb:20\\na:0\\nb:30\\n')")
    app.wait_for_labels("a", "b")
    app.tab("Graph")
    graph = app.add_scatter()
    graph.locator(".graph-title").click()  # opens the settings
    expect(graph.locator(".graph-empty")).to_contain_text("Choose the Number labels")
    graph.get_by_label("X label").fill("a")
    expect(graph.locator(".graph-empty")).to_contain_text("Choose the Number labels")
    graph.get_by_label("Y label").fill("nothing")
    expect(graph.locator(".graph-empty")).to_have_text(
        "No number data of these labels yet."
    )
    graph.get_by_label("Y label").fill("b")
    expect(graph.locator(".graph-empty")).to_have_count(0)
    plot = graph.locator(".graph-plot")
    # Each a with the b after it
    expect(plot).to_have_attribute("data-points", "3")
    app.mock("receive('a:4\\nb:40\\n')")
    expect(plot).to_have_attribute("data-points", "4")

    # Each axis on its own: those of 0 or less left out of a log one
    for axis, attribute in [("X", "data-x-scale"), ("Y", "data-y-scale")]:
        scale = graph.get_by_role("group", name=f"{axis} axis")
        expect(scale.get_by_role("button", name="Linear")).to_have_attribute(
            "aria-pressed", "true"
        )
        scale.get_by_role("button", name="Log").click()
        expect(plot).to_have_attribute(attribute, "log")
    expect(plot).to_have_attribute("data-points", "3")
    graph.get_by_role("group", name="X axis").get_by_role(
        "button", name="Linear"
    ).click()
    expect(plot).to_have_attribute("data-x-scale", "linear")
    expect(plot).to_have_attribute("data-y-scale", "log")

    # As they were, after the board is made anew
    app.tab("Console")
    app.tab("Graph")
    graph = app.graphs().last
    expect(graph.locator(".graph-plot")).to_have_attribute("data-points", "4")
    expect(graph.locator(".graph-plot")).to_have_attribute("data-y-scale", "log")


def test_scatter_shows_its_regression_line_and_equation(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_match_map()
    app.mock("receive('a:1\\nb:3\\n')")
    app.wait_for_labels("a", "b")
    app.tab("Graph")
    graph = app.add_scatter()
    graph.locator(".graph-title").click()  # opens the settings
    graph.get_by_label("X label").fill("a")
    graph.get_by_label("Y label").fill("b")
    plot = graph.locator(".graph-plot")
    statistics = plot.locator(".scatter-statistics")
    equation = graph.locator(".graph-equation")
    expect(plot).to_have_attribute("data-points", "1")
    expect(statistics).to_be_hidden()

    # On from the start
    toggle = graph.get_by_role("group", name="Regression").get_by_role("button")
    expect(toggle).to_have_attribute("aria-pressed", "true")
    expect(equation).to_have_text("Not enough points")
    expect(plot).to_have_attribute("data-line", "")

    app.mock("receive('a:2\\nb:5\\n')")
    expect(equation).to_have_attribute("aria-label", "Y = 2.00X + 1.00")
    expect(plot).to_have_attribute("data-line", "2 1")
    app.mock("receive('a:3\\nb:4\\n')")
    expect(equation).to_have_attribute("aria-label", "Y = 0.50X + 3.00")
    # As a tooltip, by the pointer over the plot
    area = plot.locator(".u-over").bounding_box()
    at = (area["x"] + area["width"] / 3, area["y"] + area["height"] / 3)
    app.page.mouse.move(*at)
    expect(statistics).to_be_visible()
    # Covariance (over n − 1) above correlation
    expect(statistics.locator("dt")).to_have_text(["Covariance", "Correlation"])
    expect(statistics.locator("dd")).to_have_text(["0.50", "0.50"])
    box = statistics.bounding_box()
    assert 0 < box["x"] - at[0] < 20 and 0 < box["y"] - at[1] < 20
    app.page.mouse.move(0, 0)
    expect(statistics).to_be_hidden()
    # What stays as it is by KaTeX, the numbers as text
    formulas = equation.locator(".formula")
    expect(formulas).to_have_count(2)
    for formula in formulas.all():
        expect(formula).to_have_attribute("data-rendered", "true")
    expect(formulas.locator(".katex-mathml annotation")).to_have_text(
        ["Y ={}", "X +{}"]
    )
    expect(equation.locator(":scope > span:not(:has(.formula))")).to_have_text(
        ["0.50", "3.00"]
    )
    expect(equation).to_have_css("box-shadow", "none")
    # Beside the toggle, as tall as it
    button, box = toggle.bounding_box(), equation.bounding_box()
    assert box["x"] > button["x"] + button["width"]
    assert abs(box["height"] - button["height"]) < 1
    assert abs(box["y"] - button["y"]) < 1
    # Of a linear equation, on log axes too
    graph.get_by_role("group", name="X axis").get_by_role("button", name="Log").click()
    expect(plot).to_have_attribute("data-x-scale", "log")
    expect(equation).to_have_attribute("aria-label", "Y = 0.50X + 3.00")

    toggle.click()
    expect(equation).to_have_count(0)
    expect(plot).to_have_attribute("data-line", "")
    # Not of the line: shown still
    app.page.mouse.move(*at)
    expect(statistics).to_be_visible()


def test_box_plot_shows_each_label_and_its_statistics_on_hover(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_match_map()
    app.mock("receive('a:1\\na:2\\na:3\\na:4\\na:100\\nb:5\\nb:6\\n')")
    app.wait_for_labels("a", "b")
    app.tab("Graph")
    bar = app.page.locator(".add-graph-bar")
    bar.get_by_role("button", name="Statistics").hover()
    bar.get_by_role("option", name="Box plot", exact=True).click()
    graph = app.graphs().last
    plot = graph.locator(".graph-plot")
    boxes = lambda: [  # noqa: E731
        label for label, *_ in json.loads(plot.get_attribute("data-boxes") or "[]")
    ]

    # Each Number label the data list shows, in a legend as the others'
    legend = graph.locator(".u-legend .u-series")
    expect(legend.locator("th", has_text="a")).to_have_count(1)
    expect(legend.locator("th", has_text="b")).to_have_count(1)
    app.page.wait_for_function(
        "plot => ['\"a\"', '\"b\"'].every(label => plot.dataset.boxes?.includes(label))",
        arg=plot.element_handle(),
    )
    assert {"a", "b"} <= set(boxes())
    # Below the plot
    area = plot.locator(".u-over").bounding_box()
    assert legend.first.bounding_box()["y"] > area["y"] + area["height"]

    # Each turned off in it, and on, as it stays so
    legend.locator("th", has_text="b").click()
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["b"])
    app.page.wait_for_function(
        "plot => !plot.dataset.boxes.includes('\"b\"')", arg=plot.element_handle()
    )
    app.tab("Console")
    app.tab("Graph")
    graph = app.graphs().last
    plot = graph.locator(".graph-plot")
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["b"])
    # A box of a, its one outlier (100) apart
    app.page.wait_for_function(
        """plot => JSON.parse(plot.dataset.boxes ?? "[]")
            .some(([label, outliers]) => label === "a" && outliers === 1)""",
        arg=plot.element_handle(),
    )
    assert "b" not in boxes()
    hover = plot.locator(".box-plot-hover")
    expect(hover).to_be_hidden()
    label = hover.locator(".plot-tooltip-label")
    # Over a's box: again, should the plot have been resized as the graph came
    for _ in range(20):
        boxes = json.loads(plot.get_attribute("data-boxes"))
        x = next(place for each, _, place in boxes if each == "a")
        area = plot.locator(".u-over").bounding_box()
        app.page.mouse.move(area["x"] + x, area["y"] + area["height"] / 2)
        if hover.is_visible() and label.text_content() == "a":
            break
        app.page.wait_for_timeout(100)
    expect(label).to_have_text("a")
    expect(hover.locator("dt")).to_have_text(
        ["Mean", "Variance", "Median", "Max", "Min", "Q3", "Q1"]
    )
    # Its variance over n − 1
    expect(hover.locator("dd")).to_have_text(
        ["22.000", "1902.5", "3.0000", "100.00", "1.0000", "4.0000", "2.0000"]
    )
    # Beside the box
    beside = hover.bounding_box()
    assert beside["x"] > area["x"] + x or beside["x"] + beside["width"] < area["x"] + x

    app.page.mouse.move(0, 0)
    expect(hover).to_be_hidden()

    graph.locator(".graph-title").click()  # opens the settings
    graph.get_by_role("group", name="Value axis").get_by_role(
        "button", name="Log"
    ).click()
    expect(plot).to_have_attribute("data-value-scale", "log")


def test_graphs_pause_and_play_as_the_port_is_read(app: App):
    open_graphs_with_numbers(app)
    graph = app.add_scatter()
    graph.locator(".graph-title").click()  # opens the settings
    graph.get_by_label("X label").fill("temp")
    graph.get_by_label("Y label").fill("volt")
    plot = graph.locator(".graph-plot")
    points = lambda: int(plot.get_attribute("data-points") or 0)  # noqa: E731
    app.page.wait_for_function(
        "plot => Number(plot.dataset.points) > 2", arg=plot.element_handle()
    )

    # At the bar's right end, a square a little rounded
    bar = app.page.locator(".graph-bar")
    pause = bar.get_by_role("button", name="Pause graphs")
    expect(pause).to_have_attribute("aria-pressed", "false")
    # lucide's Pause, its two bars
    expect(pause.locator("svg rect")).to_have_count(2)
    button, row = pause.bounding_box(), bar.bounding_box()
    assert abs((row["x"] + row["width"]) - (button["x"] + button["width"])) < 1
    expect(pause).to_have_css("border-radius", "4px")

    pause.click()
    play = bar.get_by_role("button", name="Play graphs")
    expect(play).to_have_attribute("aria-pressed", "true")
    # lucide's Play, a triangle
    expect(play.locator("svg rect")).to_have_count(0)
    expect(play.locator("svg path, svg polygon")).to_have_count(1)
    # Up to the time paused at, sent anew
    app.page.wait_for_timeout(200)
    held = points()
    # A reading each 50 ms meanwhile, the graphs as they were
    app.page.wait_for_timeout(600)
    assert points() == held

    # Their settings changed meanwhile, as they are: of the data until then
    graph.get_by_role("group", name="X axis").get_by_role("button", name="Log").click()
    expect(plot).to_have_attribute("data-x-scale", "log")
    assert points() == held
    first = app.graphs().first
    first.locator(".graph-title").click()  # opens its settings
    first.get_by_role("group", name="Value axis").get_by_role(
        "button", name="Log"
    ).click()
    expect(first.locator(".graph-plot")).to_have_attribute("data-value-scale", "log")
    app.page.wait_for_timeout(300)
    assert points() == held

    # All that came meanwhile, at once
    play.click()
    expect(bar.get_by_role("button", name="Pause graphs")).to_be_visible()
    app.page.wait_for_function(
        "([plot, held]) => Number(plot.dataset.points) >= held + 8",
        arg=[plot.element_handle(), held],
    )


def test_arrows_point_from_the_origin_to_the_newest_vectors(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_vector_match_map(
        "2D", r"({word})=({number}),({number})", "$1", ["$2", "$3"]
    )
    app.mock("receive('v=1,1\\nv=3,4\\nw=-2,0\\n')")
    app.wait_for_labels("v", "w")
    app.tab("Graph")
    bar = app.page.locator(".add-graph-bar")
    bar.get_by_role("button", name="Vector").hover()
    expect(bar.get_by_role("option")).to_have_text(["Arrows (2D)", "Arrows (3D)"])
    bar.get_by_role("option", name="Arrows (2D)", exact=True).click()
    graph = app.graphs().last
    plot = graph.locator(".graph-plot")
    arrows = lambda: json.loads(plot.get_attribute("data-arrows") or "[]")  # noqa: E731
    shown = lambda: [label for label, *_ in arrows()]  # noqa: E731

    # Each 2D vector label the data list shows, in the legend
    legend = graph.locator(".u-legend .u-series th")
    expect(legend).to_have_text(["v", "w"])
    app.page.wait_for_function(
        "plot => JSON.parse(plot.dataset.arrows ?? '[]').length === 2"
        " && plot.dataset.settled === 'true'",
        arg=plot.element_handle(),
    )

    # As many units a pixel across as up, so the arrows point as they are
    across, up = json.loads(plot.get_attribute("data-units-per-pixel"))
    assert across == pytest.approx(up, rel=0.01)
    # The origin in the middle, the furthest arrow within
    area = plot.locator(".u-over").bounding_box()
    origin = json.loads(plot.get_attribute("data-origin"))
    assert origin == pytest.approx([area["width"] / 2, area["height"] / 2], abs=1)
    assert 4 / up < area["height"] / 2
    tips = {label: (x, y) for label, x, y in arrows()}
    # From w (−2, 0) to v (3, 4), its newest: 5 right, 4 up
    assert tips["v"][0] - tips["w"][0] == pytest.approx(5 / across, abs=2)
    assert tips["w"][1] - tips["v"][1] == pytest.approx(4 / up, abs=2)

    hover = plot.locator(".arrows-hover")
    expect(hover).to_be_hidden()
    area = plot.locator(".u-over").bounding_box()
    app.page.mouse.move(area["x"] + tips["v"][0] + 3, area["y"] + tips["v"][1] + 3)
    expect(hover).to_be_visible()
    expect(hover.locator(".plot-tooltip-label")).to_have_text("v")
    expect(hover.locator("dt")).to_have_text(["X", "Y", "Magnitude", "Angle"])
    expect(hover.locator("dd")).to_have_text(["3.0000", "4.0000", "5.0000", "53.130°"])
    app.page.mouse.move(0, 0)
    expect(hover).to_be_hidden()

    # Trails on from the start, and off
    expect(plot).to_have_attribute("data-trails", "true")
    graph.locator(".graph-title").click()  # opens the settings
    trails = graph.get_by_role("group", name="Trails").get_by_role("button")
    expect(trails).to_have_attribute("aria-pressed", "true")
    trails.click()
    expect(trails).to_have_attribute("aria-pressed", "false")
    expect(plot).to_have_attribute("data-trails", "false")
    graph.locator(".graph-title").click()  # closes them

    # Each turned off in the legend, and on, as it stays so
    graph.locator(".u-legend .u-series th", has_text="w").click()
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["w"])
    app.page.wait_for_function(
        "plot => JSON.parse(plot.dataset.arrows ?? '[]').map(([label]) => label).join() === 'v'",
        arg=plot.element_handle(),
    )
    app.tab("Console")
    app.tab("Graph")
    graph = app.graphs().last
    plot = graph.locator(".graph-plot")
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["w"])
    expect(plot).to_have_attribute("data-trails", "false")
    app.page.wait_for_function(
        "plot => JSON.parse(plot.dataset.arrows ?? '[]').map(([label]) => label).join() === 'v'",
        arg=plot.element_handle(),
    )
    graph.locator(".u-legend .u-series th", has_text="w").click()
    app.page.wait_for_function(
        "plot => JSON.parse(plot.dataset.arrows ?? '[]').length === 2",
        arg=plot.element_handle(),
    )
    assert sorted(shown()) == ["v", "w"]


def test_arrows_in_space_turn_as_they_are_dragged(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_vector_match_map(
        "3D", r"({word})=({number}),({number}),({number})", "$1", ["$2", "$3", "$4"]
    )
    app.mock("receive('up=0,0,1\\nup=0,0,2\\nside=3,0,0\\n')")
    app.wait_for_labels("up", "side")
    app.tab("Graph")
    bar = app.page.locator(".add-graph-bar")
    bar.get_by_role("button", name="Vector").hover()
    bar.get_by_role("option", name="Arrows (3D)", exact=True).click()
    graph = app.graphs().last
    plot = graph.locator(".graph-plot")
    canvas = plot.locator("canvas")
    arrows = lambda: {  # noqa: E731
        label: (x, y)
        for label, x, y in json.loads(plot.get_attribute("data-arrows") or "[]")
    }

    # Each 3D vector label the data list shows, in a legend as the others'
    expect(graph.locator(".u-legend .u-series th")).to_have_text(["side", "up"])
    app.page.wait_for_function(
        "plot => JSON.parse(plot.dataset.arrows ?? '[]').length === 2"
        " && plot.dataset.settled === 'true'",
        arg=plot.element_handle(),
    )
    # The origin in the middle; Z up from it, straight
    box = canvas.bounding_box()
    origin = json.loads(plot.get_attribute("data-origin"))
    assert origin == pytest.approx([box["width"] / 2, box["height"] / 2], abs=1)
    tips = arrows()
    assert tips["up"][0] == pytest.approx(origin[0], abs=1)
    assert tips["up"][1] < origin[1] - 20
    # The axes' names and ticks over it
    expect(plot.locator(".arrows-3d-axis")).to_have_text(["X", "Y", "Z"])
    expect(plot.locator(".arrows-3d-tick").first).to_be_visible()

    hover = plot.locator(".arrows-hover")
    expect(hover).to_be_hidden()
    app.page.mouse.move(box["x"] + tips["up"][0] + 3, box["y"] + tips["up"][1] + 3)
    expect(hover).to_be_visible()
    expect(hover.locator(".plot-tooltip-label")).to_have_text("up")
    expect(hover.locator("dt")).to_have_text(["X", "Y", "Z", "Magnitude"])
    expect(hover.locator("dd")).to_have_text(["0.0000", "0.0000", "2.0000", "2.0000"])

    # Turned by dragging: the origin stays, the arrows go round it
    app.page.mouse.move(box["x"] + 40, box["y"] + 40)
    app.page.mouse.down()
    app.page.mouse.move(box["x"] + 140, box["y"] + 60, steps=8)
    app.page.mouse.up()
    expect(hover).to_be_hidden()
    turned = arrows()
    assert turned["side"] != pytest.approx(tips["side"], abs=5)
    assert json.loads(plot.get_attribute("data-origin")) == origin

    # Trails on from the start, and off
    expect(plot).to_have_attribute("data-trails", "true")
    graph.locator(".graph-title").click()  # opens the settings
    trails = graph.get_by_role("group", name="Trails").get_by_role("button")
    expect(trails).to_have_attribute("aria-pressed", "true")
    trails.click()
    expect(plot).to_have_attribute("data-trails", "false")
    graph.locator(".graph-title").click()  # closes them

    # Each turned off in the legend, as it stays so
    graph.locator(".u-legend .u-series th", has_text="side").click()
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["side"])
    app.page.wait_for_function(
        "plot => JSON.parse(plot.dataset.arrows ?? '[]').map(([label]) => label).join() === 'up'",
        arg=plot.element_handle(),
    )
    app.tab("Console")
    app.tab("Graph")
    graph = app.graphs().last
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["side"])
    expect(graph.locator(".graph-plot")).to_have_attribute("data-trails", "false")


def test_zooming_into_a_graph_pauses_them(app: App):
    open_graphs_with_numbers(app)
    app.wait_for_graph_legends([["temp", "volt"]])
    graph = app.graphs().first
    plot = graph.locator(".graph-plot")
    bar = app.page.locator(".graph-bar")
    span = lambda: float(plot.get_attribute("data-time-span"))  # noqa: E731
    values = lambda: float(plot.get_attribute("data-value-span"))  # noqa: E731
    app.page.wait_for_timeout(1000)
    all_values = values()

    # A rectangle dragged over it: zoomed into, times and values, and paused
    area = plot.locator(".u-over").bounding_box()
    y = area["y"] + area["height"] / 2
    app.page.mouse.move(
        area["x"] + area["width"] * 0.3, area["y"] + area["height"] * 0.3
    )
    app.page.mouse.down()
    app.page.mouse.move(
        area["x"] + area["width"] * 0.6, area["y"] + area["height"] * 0.6, steps=5
    )
    app.page.mouse.up()
    expect(plot).to_have_attribute("data-zoomed", "true")
    zoomed_values = values()
    # About 0.3 of them, the range moving with the data until then
    assert zoomed_values < all_values * 0.6
    expect(bar.get_by_role("button", name="Play graphs")).to_have_attribute(
        "aria-pressed", "true"
    )
    zoomed = span()

    # As zoomed, as data comes and its settings change
    app.page.wait_for_timeout(500)
    assert span() == pytest.approx(zoomed)
    assert values() == pytest.approx(zoomed_values)
    graph.locator(".graph-title").click()  # opens the settings
    graph.get_by_role("group", name="Value axis").get_by_role(
        "button", name="Log"
    ).click()
    expect(plot).to_have_attribute("data-value-scale", "log")
    app.page.wait_for_timeout(300)
    # The times as zoomed to; the values, not of another scale
    assert span() == pytest.approx(zoomed)

    # Unzoomed by a double click, paused still; then played
    app.page.mouse.dblclick(area["x"] + area["width"] / 2, y)
    expect(plot).to_have_attribute("data-zoomed", "false")
    app.page.wait_for_function(
        "([plot, zoomed]) => Number(plot.dataset.timeSpan) > zoomed * 1.2",
        arg=[plot.element_handle(), zoomed],
    )
    play = bar.get_by_role("button", name="Play graphs")
    expect(play).to_be_visible()
    play.click()
    expect(bar.get_by_role("button", name="Pause graphs")).to_be_visible()

    # A scatter plot too
    scatter = app.add_scatter()
    scatter.locator(".graph-title").click()  # opens the settings
    scatter.get_by_label("X label").fill("temp")
    scatter.get_by_label("Y label").fill("volt")
    points = scatter.locator(".graph-plot")
    app.page.wait_for_function(
        "plot => Number(plot.dataset.points) > 2", arg=points.element_handle()
    )
    area = points.locator(".u-over").bounding_box()
    app.page.mouse.move(
        area["x"] + area["width"] * 0.2, area["y"] + area["height"] * 0.2
    )
    app.page.mouse.down()
    app.page.mouse.move(
        area["x"] + area["width"] * 0.7, area["y"] + area["height"] * 0.7, steps=5
    )
    app.page.mouse.up()
    expect(points).to_have_attribute("data-zoomed", "true")
    expect(bar.get_by_role("button", name="Play graphs")).to_be_visible()


def test_box_plot_takes_the_values_of_its_time_range(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_match_map()
    app.stop_clock()
    # An outlier (100) among the first values, none among those 15 s later
    app.mock("receive('a:1\\na:2\\na:3\\na:4\\na:100\\n')")
    app.run_clock_until(lambda: "a" in app.data_rows())
    app.page.clock.run_for(15_000)
    app.mock("receive('a:1\\na:2\\na:3\\na:4\\n')")
    app.tab("Graph")
    bar = app.page.locator(".add-graph-bar")
    bar.get_by_role("button", name="Statistics").hover()
    bar.get_by_role("option", name="Box plot", exact=True).click()
    graph = app.graphs().last
    plot = graph.locator(".graph-plot")

    def outliers_of_a():
        boxes = json.loads(plot.get_attribute("data-boxes") or "[]")
        return next((outliers for label, outliers, *_ in boxes if label == "a"), None)

    # 10 s back from the newest at first: those after the outlier alone
    graph.locator(".graph-title").click()  # opens the settings
    slider = graph.get_by_role("slider", name="Time range")
    expect(slider).to_have_attribute("aria-valuenow", "10")
    expect(graph.locator(".graph-time-range-value")).to_have_text("10 s")
    app.run_clock_until(lambda: outliers_of_a() == 0)

    # 20 s: the outlier back
    slider.focus()
    app.page.keyboard.press("ArrowRight")
    expect(slider).to_have_attribute("aria-valuenow", "20")
    app.run_clock_until(lambda: outliers_of_a() == 1)
    app.page.keyboard.press("ArrowLeft")
    app.run_clock_until(lambda: outliers_of_a() == 0)

    # Back from the newest: no newer values, none leaves it as time goes on
    app.page.clock.run_for(11_000)
    app.run_clock_until(lambda: outliers_of_a() == 0)

    # All the time: all the values kept, the slider off
    graph.get_by_role("button", name="All").click()
    expect(slider).to_be_disabled()
    app.run_clock_until(lambda: outliers_of_a() == 1)


def test_scatter_shows_the_points_of_its_time_range(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_match_map()
    app.stop_clock()
    app.mock("receive('a:1\\nb:10\\na:2\\nb:20\\n')")
    app.run_clock_until(lambda: "b" in app.data_rows())
    app.page.clock.run_for(15_000)
    app.mock("receive('a:3\\nb:30\\n')")
    app.tab("Graph")
    graph = app.add_scatter()
    graph.locator(".graph-title").click()  # opens the settings
    graph.get_by_label("X label").fill("a")
    graph.get_by_label("Y label").fill("b")
    plot = graph.locator(".graph-plot")

    def points():
        return int(plot.get_attribute("data-points") or 0)

    # 10 s back from the newest at first: the last alone
    slider = graph.get_by_role("slider", name="Time range")
    expect(slider).to_have_attribute("aria-valuenow", "10")
    app.run_clock_until(lambda: points() == 1)

    # 20 s: those before come back
    slider.focus()
    app.page.keyboard.press("ArrowRight")
    app.run_clock_until(lambda: points() == 3)
    app.page.keyboard.press("ArrowLeft")
    app.run_clock_until(lambda: points() == 1)

    # All the time: every one
    graph.get_by_role("button", name="All").click()
    app.run_clock_until(lambda: points() == 3)


def test_scatter_settings_open_and_close_in_any_row(app: App):
    app.open_port()
    app.tab("Data")
    app.load_layout(
        [name_value_map()],
        [
            {"kind": "Time series"},
            {"kind": "Box plot"},
            {"kind": "Scatter", "settings": {"x_label": "temp", "y_label": "volt"}},
        ],
    )
    app.tab("Graph")
    # In the second row
    scatter = app.graphs().nth(2)
    assert scatter.bounding_box()["y"] > app.graphs().first.bounding_box()["y"]
    title = scatter.locator(".graph-title")
    body = scatter.locator(".graph-settings-body")

    title.click()
    expect(body).to_be_visible()
    # Its title at the top of the settings, taller than they show: not under the
    # plot, so it closes them again
    title.click()
    expect(body).to_be_hidden()


def test_scatter_redraws_ten_times_a_second_at_most(app: App):
    open_graphs_with_numbers(app)
    graph = app.add_scatter()
    graph.locator(".graph-title").click()  # opens the settings
    graph.get_by_label("X label").fill("temp")
    graph.get_by_label("Y label").fill("volt")
    # So none leaves it as they are counted
    graph.get_by_role("button", name="All").click()
    plot = graph.locator(".graph-plot")
    app.page.wait_for_function(
        "plot => Number(plot.dataset.points) > 2", arg=plot.element_handle()
    )
    draws = lambda: int(plot.get_attribute("data-draws"))  # noqa: E731
    # Pairs coming 20 times a second, and more at once
    app.mock("burst(50)")
    first, first_points = draws(), int(plot.get_attribute("data-points"))
    app.page.wait_for_timeout(1000)
    assert draws() - first <= 12
    # Each drawn all the same
    app.page.wait_for_function(
        "([plot, points]) => Number(plot.dataset.points) >= points",
        arg=[plot.element_handle(), first_points + 50],
    )


def test_graphs_redraw_less_as_the_page_is_busy(app: App):
    open_graphs_with_numbers(app)
    app.wait_for_graph_legends([["temp", "volt"]])
    interval = "() => window.servalDrawPacer?.interval()"
    app.page.wait_for_function(f"({interval})() !== undefined")

    # Frames late, as the page is busy: fewer redraws (more again as they come
    # on time, as the unit tests tell: the page of a busy machine may not)
    app.page.evaluate("""() => {
        window.busy = setInterval(() => {
            const start = performance.now();
            while (performance.now() - start < 100) {}
        }, 10);
    }""")
    app.page.wait_for_function(f"({interval})() >= 50", timeout=20000)
    app.page.evaluate("() => clearInterval(window.busy)")


def test_log_table_shows_the_logs_as_they_come(app: App):
    app.open_port()
    app.mock("mute()")
    app.tab("Data")
    app.add_log_match(r"^sensor (\w+) (.*)", "$1", "$2", "sensor", "log")
    levels = ["info", "warn", "error", "debug", "info", "trace"] * 10
    lines = "".join(
        f"[{level}] disk {i}\\nsensor {level} reading {i}\\n"
        for i, level in enumerate(levels)
    )
    app.mock(f"receive('{lines}')")
    app.wait_for_labels("log", "sensor")
    app.tab("Graph")
    bar = app.page.locator(".add-graph-bar")
    bar.get_by_role("button", name="Text").hover()
    expect(bar.get_by_role("option")).to_have_text(["Log"])
    bar.get_by_role("option", name="Log", exact=True).click()
    graph = app.graphs().last
    table = graph.locator(".log-table")
    body = table.locator(".log-table-body")
    rows = body.locator("[data-virtual-index]")
    count = lambda: int(rows.first.get_attribute("aria-setsize"))  # noqa: E731
    first = lambda: rows.first.locator("[role=cell]").all_text_contents()  # noqa: E731
    # The message of the row at the top of the view
    in_view = (  # noqa: E731
        'list => list.querySelector(`[data-virtual-index="${Math.round(list.scrollTop / 24)}"]'
        " .log-table-message`).textContent"
    )
    counts = table.get_by_role("group", name="Logs of each level")
    values = counts.locator(".log-table-count-value")
    # Of each level: ERROR, WARN, INFO, DEBUG, TRACE, then of none (the sensor's
    # lines in the log, and any read before it was muted)
    numbers = lambda: [int(value) for value in values.all_text_contents()]  # noqa: E731

    expect(table.locator(".log-table-head").get_by_role("columnheader")).to_have_text(
        ["Time", "Label", "Level", "Message"]
    )
    # The weakest level shown a setting: INFO and those that matter more, from
    # the start (4 of each 6, of each label)
    graph.locator(".graph-title").click()  # opens the settings
    level = graph.get_by_role("group", name="Weakest level")
    expect(level.get_by_role("button")).to_have_text(
        ["ERROR", "WARN", "INFO", "DEBUG", "TRACE", "ALL"]
    )
    expect(level.get_by_role("button", name="INFO")).to_have_attribute(
        "aria-pressed", "true"
    )
    expect(rows.first).to_have_attribute("aria-setsize", "80")
    # Above the table, how many of each level: those left out too, fainter
    expect(values.nth(4)).to_have_text("20")
    assert numbers()[:5] == [20, 20, 40, 20, 20] and numbers()[5] >= 60
    shown = [
        each.get_attribute("data-shown")
        for each in counts.locator(".log-table-count").all()
    ]
    assert shown == ["true"] * 3 + ["false"] * 3
    counted, table_box = counts.bounding_box(), body.bounding_box()
    assert counted["y"] + counted["height"] <= table_box["y"]
    levels_shown = set(body.locator(".log-level-tag").all_text_contents())
    assert levels_shown <= {"ERROR", "WARN", "INFO"}
    # Only the rows in view made
    assert rows.count() < 60
    # Newest first: of one time, the last label's first
    expect(rows.first.locator(".log-table-message")).to_have_text("reading 58")
    assert first()[1] == "sensor"
    assert re.fullmatch(r"\d{2}:\d{2}:\d{2}\.\d{3}", first()[0])
    assert body.evaluate("list => list.scrollTop") == 0

    # Followed as they come, on top
    app.mock("receive('[warn] newest one\\n')")
    expect(rows.first.locator(".log-table-message")).to_have_text("newest one")
    assert first()[1:] == ["log", "WARN", "newest one"]
    assert body.evaluate("list => list.scrollTop") == 0
    # Not once scrolled down to older ones: those in view kept where they are
    body.evaluate("list => list.scrollTop = 240")
    app.page.wait_for_timeout(300)
    reading = body.evaluate(in_view)
    app.mock("receive('[error] newer still\\n')")
    expect(rows.first).to_have_attribute("aria-setsize", "82")
    app.page.wait_for_function(
        "list => list.scrollTop === 264", arg=body.element_handle()
    )
    assert body.evaluate(in_view) == reading
    expect(values.first).to_have_text("21")
    assert numbers()[:3] == [21, 21, 40]

    # All levels, as chosen: the lines of no level too (those of the sensor in
    # the log as well; and any read before it was muted)
    level.get_by_role("button", name="ALL").click()
    app.page.wait_for_function(
        "row => Number(row.getAttribute('aria-setsize')) >= 182",
        arg=rows.first.element_handle(),
    )
    all_count = count()
    # A label turned off in the legend: its logs left out
    graph.locator(".u-legend .u-series th", has_text="sensor").click()
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["sensor"])
    expect(rows.first).to_have_attribute("aria-setsize", str(all_count - 60))
    assert "sensor" not in body.locator(".log-table-label").all_text_contents()
    # Counted of the labels shown
    expect(values.first).to_have_text("11")
    assert numbers()[:5] == [11, 11, 20, 10, 10]

    # As they were, after the board is made anew
    app.tab("Console")
    app.tab("Graph")
    graph = app.graphs().last
    expect(graph.locator(".u-legend .u-series.u-off th")).to_have_text(["sensor"])
    graph.locator(".graph-title").click()  # opens the settings
    expect(
        graph.get_by_role("group", name="Weakest level").get_by_role(
            "button", name="ALL"
        )
    ).to_have_attribute("aria-pressed", "true")
    expect(graph.locator("[data-virtual-index]").first).to_have_attribute(
        "aria-setsize", str(all_count - 60)
    )
