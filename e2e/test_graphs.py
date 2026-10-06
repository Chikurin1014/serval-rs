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
    add = app.page.get_by_role("button", name="Add graph")
    add.click()
    add.click()
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
    add = app.page.get_by_role("button", name="Add graph")
    add.click()
    add.click()
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
    app.page.locator(".graph-board").get_by_role("button", name="Clear all").click()

    app.wait_for_labels("temp")
    app.wait_for_graph_legends([["temp", "volt"]])
