from playwright.sync_api import expect

from app import App

GRID_COLUMNS = (
    "getComputedStyle(document.querySelector('.graph-grid'))"
    ".gridTemplateColumns.split(' ').length"
)


def open_graphs_with_numbers(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("temp", "volt")
    app.tab("Graph")


def test_plots_the_labels_the_data_list_shows(app: App):
    open_graphs_with_numbers(app)
    app.set_up_graph(0, "Readings")
    app.wait_for_graph_legends([["temp", "volt"]])
    expect(app.graphs().first.locator(".graph-title-text")).to_have_text("Readings")
    # No labels to choose in the settings any more
    app.graphs().first.locator(".graph-title").click()
    expect(app.graphs().first.locator(".graph-settings-body [role=grid]")).to_have_count(0)

    # Filtered in the data list beside the graphs, as in the Data tab
    app.page.get_by_role("button", name="Toggle data list").click()
    board = app.page.locator(".graph-board")
    board.get_by_placeholder("Label filter").fill("temp")
    board.get_by_role("button", name="Add filter").click()
    app.wait_for_graph_legends([["temp"]])

    # Nothing left to plot: says why
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

    # Kept when the graph is shown again
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
    expect(draw.get_by_role("button", name="Linear")).to_have_attribute("aria-pressed", "true")
    for name, style in [("Points", "points"), ("Stepped", "stepped"), ("Linear", "linear")]:
        draw.get_by_role("button", name=name).click()
        expect(draw.get_by_role("button", name=name)).to_have_attribute("aria-pressed", "true")
        expect(plot).to_have_attribute("data-draw-style", style)
        app.wait_for_graph_legends([["temp", "volt"]])


def test_cursor_moves_together_across_graphs(app: App):
    open_graphs_with_numbers(app)
    app.add_graph()
    app.wait_for_graph_legends([["temp", "volt"]] * 2)
    first, second = app.graphs().nth(0), app.graphs().nth(1)

    def legend_values(graph):
        return graph.locator(".u-legend .u-value").all_text_contents()

    def cursor_offset(graph):
        """How far into its plot the graph's cursor line is, in pixels."""
        return graph.locator(".u-over").evaluate(
            "over => over.parentElement.querySelector('.u-cursor-x').getBoundingClientRect().left"
            " - over.getBoundingClientRect().left"
        )

    # Pointing at the first graph points at the same time in the second
    over = first.locator(".u-over")
    box = over.bounding_box()
    app.page.mouse.move(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
    expect(second.locator(".u-legend .u-value").first).not_to_have_text("--")
    assert all(value != "--" for value in legend_values(second))
    # At the same place in each, as both show the same span of time
    assert abs(cursor_offset(first) - cursor_offset(second)) < 3


def test_settings_and_legend_toggles_stay(app: App):
    open_graphs_with_numbers(app)
    app.add_graph()
    app.wait_for_graph_legends([["temp", "volt"]] * 2)

    def turned_off(graph):
        """The labels turned off in the graph's legend."""
        return graph.locator(".u-legend .u-series.u-off th").all_text_contents()

    def check():
        first, second = app.graphs().nth(0), app.graphs().nth(1)
        expect(first.locator(".u-legend .u-series.u-off")).to_have_count(1)
        assert [label.strip() for label in turned_off(first)] == ["temp"]
        # The other graph has its own
        expect(second.locator(".u-legend .u-series.u-off")).to_have_count(0)
        expect(first.locator(".graph-title-text")).to_have_text("Readings")
        plot = first.locator(".graph-plot")
        expect(plot).to_have_attribute("data-value-scale", "log")
        expect(plot).to_have_attribute("data-draw-style", "points")

    first = app.graphs().first
    app.set_up_graph(0, "Readings")
    first.locator(".graph-title").click()  # opens the settings
    first.get_by_role("group", name="Value axis").get_by_role("button", name="Log").click()
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
