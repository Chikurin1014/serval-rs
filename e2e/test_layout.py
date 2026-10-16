import json
import re
from pathlib import Path

from playwright.sync_api import expect

from app import App
from test_graphs import add_scatter

# Serval's, as `CARGO_PKG_VERSION` (its unit tests check that; the e2e tests
# run with no Cargo.toml)
VERSION = re.compile(r"^\d+\.\d+\.\d+")


def layout_menu(app: App, item: str):
    app.page.get_by_role("button", name="Layout").click()
    app.page.get_by_role("option", name=item).click()


def export_layout(app: App, path: Path) -> dict:
    with app.page.expect_download() as download:
        layout_menu(app, "Export to JSON")
    assert download.value.suggested_filename == "serval-layout.json"
    download.value.save_as(path)
    return json.loads(path.read_text())


def import_layout(app: App, path: Path):
    with app.page.expect_file_chooser() as chooser:
        layout_menu(app, "Import from JSON")
    chooser.value.set_files(path)


def test_layout_menu_is_beside_the_theme_switch(app: App):
    trigger = app.page.get_by_role("button", name="Layout")
    trigger.click()
    expect(app.page.get_by_role("option")).to_have_text(
        ["Import from JSON", "Export to JSON"]
    )
    theme = app.page.get_by_role("button", name="Toggle theme")
    assert trigger.bounding_box()["x"] < theme.bounding_box()["x"]


def test_an_exported_layout_is_imported_in_place_of_the_one_there_is(
    app: App, tmp_path: Path
):
    app.tab("Graph")
    app.set_up_graph(0, "Readings")
    add_scatter(app)
    app.tab("Data")
    card = app.map_cards().nth(1)
    card.get_by_role("switch").click()  # off
    expect(card.get_by_role("switch")).not_to_be_checked()

    layout = export_layout(app, tmp_path / "layout.json")
    assert VERSION.match(layout["version"])
    assert [map["kind"] for map in layout["maps"]] == ["Decode", "Match", "Match"]
    assert [map["enabled"] for map in layout["maps"]] == [True, False, True]
    assert layout["maps"][1]["settings"]["pattern"] == "^{number}"
    assert [graph["kind"] for graph in layout["graphs"]] == ["Time series", "Scatter"]
    assert layout["graphs"][0]["settings"]["title"] == "Readings"
    assert layout["graphs"][1]["settings"]["regression"] is True

    # Changed, then put back as exported
    app.map_cards().nth(2).get_by_role("button", name="Delete map").click()
    expect(app.map_cards()).to_have_count(2)
    app.tab("Graph")
    app.add_graph()
    expect(app.graphs()).to_have_count(3)

    import_layout(app, tmp_path / "layout.json")
    app.wait_for_toast("Layout imported")
    assert app.toasts()[0] == ("success", "Layout imported", "layout.json")
    expect(app.graphs()).to_have_count(2)
    expect(app.graphs().locator(".graph-title-text")).to_have_text(
        ["Readings", "Graph 2"]
    )
    app.tab("Data")
    expect(app.map_cards()).to_have_count(3)
    expect(app.map_cards().get_by_role("switch")).to_have_count(3)
    assert [
        app.map_cards().nth(index).get_by_role("switch").is_checked()
        for index in range(3)
    ] == [True, False, True]
    assert export_layout(app, tmp_path / "again.json") == layout


def test_a_wrong_layout_changes_nothing(app: App, tmp_path: Path):
    app.tab("Graph")
    layout = export_layout(app, tmp_path / "layout.json")
    layout["graphs"].append({"kind": "Pie chart"})
    (tmp_path / "wrong.json").write_text(json.dumps(layout))

    import_layout(app, tmp_path / "wrong.json")
    app.wait_for_toast("Failed to import the layout")
    assert app.toasts()[0] == (
        "error",
        "Failed to import the layout",
        "No graph Pie chart",
    )
    expect(app.graphs()).to_have_count(1)

    assert export_layout(app, tmp_path / "after.json") == layout | {
        "graphs": layout["graphs"][:1]
    }


def test_a_file_not_of_a_layout_changes_nothing(app: App, tmp_path: Path):
    app.tab("Graph")
    (tmp_path / "broken.json").write_text("{")
    import_layout(app, tmp_path / "broken.json")
    app.wait_for_toast("Failed to import the layout")
    assert app.toasts()[0][2].startswith("Not a layout file")
    expect(app.graphs()).to_have_count(1)
