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


def test_plots_the_selected_labels(app: App):
    open_graphs_with_numbers(app)
    app.set_up_graph(0, "Temp", ["temp"])
    app.wait_for_graph_legends([["temp"]])
    expect(app.graphs().first.locator(".graph-title-text")).to_have_text("Temp")


def test_graphs_survive_switching_tabs(app: App):
    open_graphs_with_numbers(app)
    add = app.page.get_by_role("button", name="Add graph")
    add.click()
    add.click()
    setup = [("Temp", ["temp"]), ("Volt", ["volt"]), ("Both", ["temp", "volt"])]
    for index, (title, labels) in enumerate(setup):
        app.set_up_graph(index, title, labels)
    legends = [labels for _, labels in setup]
    app.wait_for_graph_legends(legends)
    assert app.page.evaluate(GRID_COLUMNS) == 2

    app.tab("Console")
    app.tab("Graph")
    app.wait_for_graph_legends(legends)
    titles = app.graphs().locator(".graph-title-text")
    expect(titles).to_have_text(["Temp", "Volt", "Both"])


def test_removing_a_graph_keeps_the_others(app: App):
    open_graphs_with_numbers(app)
    add = app.page.get_by_role("button", name="Add graph")
    add.click()
    add.click()
    for index, (title, labels) in enumerate(
        [("Temp", ["temp"]), ("Volt", ["volt"]), ("Both", ["temp", "volt"])]
    ):
        app.set_up_graph(index, title, labels)

    middle = app.graphs().nth(1)
    middle.hover()  # the button shows on hover
    middle.get_by_role("button", name="Remove graph").click()

    expect(app.graphs().locator(".graph-title-text")).to_have_text(["Temp", "Both"])
    app.wait_for_graph_legends([["temp"], ["temp", "volt"]])
    expect(app.page.locator(".uplot")).to_have_count(2)


def test_clearing_data_from_the_sidebar_keeps_plotting(app: App):
    open_graphs_with_numbers(app)
    app.set_up_graph(0, "Temp", ["temp"])
    app.wait_for_graph_legends([["temp"]])

    app.page.get_by_role("button", name="Toggle data list").click()
    app.page.locator(".graph-board").get_by_role("button", name="Clear all").click()

    app.wait_for_labels("temp")
    app.wait_for_graph_legends([["temp"]])
