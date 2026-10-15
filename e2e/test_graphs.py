from playwright.sync_api import expect

from app import App

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


def wait_for_time_span(graph, low: float, high: float):
    graph.page.wait_for_function(
        """([plot, low, high]) => {
            const span = Number(plot.dataset.timeSpan);
            return low <= span && span <= high;
        }""",
        arg=[graph.locator(".graph-plot").element_handle(), low, high],
    )


def test_time_axis_shows_a_window_of_the_newest_data(app: App):
    open_graphs_with_numbers(app)
    app.wait_for_graph_legends([["temp", "volt"]])
    graph = app.graphs().first

    # 10 s: fits the data
    wait_for_time_span(graph, 0.01, 9.99)

    graph.locator(".graph-title").click()  # opens the settings
    slider = graph.get_by_role("slider", name="Time axis")
    expect(slider).to_have_attribute("aria-valuemin", "10")
    expect(slider).to_have_attribute("aria-valuemax", "300")
    expect(slider).to_have_attribute("aria-valuenow", "10")
    value = graph.locator(".graph-time-window-value")
    expect(value).to_have_text("10 s")

    slider.focus()
    for _ in range(5):
        app.page.keyboard.press("ArrowRight")
    expect(slider).to_have_attribute("aria-valuenow", "60")
    expect(value).to_have_text("60 s")
    wait_for_time_span(graph, 59.99, 60.01)

    app.tab("Console")
    app.tab("Graph")
    wait_for_time_span(app.graphs().first, 59.99, 60.01)


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


def add_scatter(app: App):
    bar = app.page.locator(".add-graph-bar")
    bar.get_by_role("button", name="Statistics").hover()
    bar.get_by_role("option", name="Scatter", exact=True).click()
    return app.graphs().last


def test_statistics_menu_adds_a_scatter_plot(app: App):
    app.tab("Graph")
    bar = app.page.locator(".add-graph-bar")
    expect(bar.get_by_role("button")).to_have_text(["Time series", "Statistics"])
    bar.get_by_role("button", name="Statistics").hover()
    expect(bar.get_by_role("option")).to_have_text(["Scatter"])
    graph = add_scatter(app)
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
    graph = add_scatter(app)
    graph.locator(".graph-title").click()  # opens the settings
    expect(graph.locator(".graph-empty")).to_contain_text("Choose the Number labels")
    graph.get_by_label("X label").fill("a")
    expect(graph.locator(".graph-empty")).to_contain_text("Choose the Number labels")
    graph.get_by_label("Y label").fill("nothing")
    expect(graph.locator(".graph-empty")).to_have_text("No number data of these labels yet.")
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
    graph.get_by_role("group", name="X axis").get_by_role("button", name="Linear").click()
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
    graph = add_scatter(app)
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
    expect(formulas.locator(".katex-mathml annotation")).to_have_text(["Y ={}", "X +{}"])
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

