import pytest
from playwright.sync_api import expect

from app import App


def test_licenses_open_in_a_new_tab(app: App):
    app.open_port()
    link = app.page.get_by_role("navigation", name="Pages").get_by_role(
        "menuitem", name="Licenses"
    )
    expect(link).to_have_attribute("href", "/third-party-licenses.html")
    with app.page.context.expect_page() as opened:
        link.click()
    licenses = opened.value
    licenses.wait_for_load_state()
    expect(licenses).to_have_title("Serval: third-party licenses")
    expect(licenses.get_by_role("heading", name="uPlot")).to_be_visible()

    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()


def test_documents_open_in_a_new_tab(app: App):
    app.open_port()
    with app.page.context.expect_page() as opened:
        app.page.get_by_role("navigation", name="Pages").get_by_role(
            "menuitem", name="Documents"
        ).click()
    documents = opened.value
    documents.wait_for_load_state()
    expect(documents).to_have_title("Introduction - Serval")

    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()


def github_link(app: App):
    return app.page.get_by_role("link", name="Serval on GitHub")


def test_the_pages_and_the_tools_are_in_their_places(app: App):
    pages = app.page.get_by_role("navigation", name="Pages")
    title = pages.get_by_role("menuitem", name="Serval").bounding_box()
    documents = pages.get_by_role("menuitem", name="Documents")
    licenses = pages.get_by_role("menuitem", name="Licenses").bounding_box()
    expect(documents).to_have_attribute("href", "/docs/")
    expect(documents).to_have_attribute("target", "_blank")
    box = documents.bounding_box()
    assert title["x"] + title["width"] <= box["x"]
    assert box["x"] + box["width"] <= licenses["x"]

    # The repository right of the licenses, as an icon button (as the theme
    # switch), in a new tab so the open port stays
    link = github_link(app)
    expect(link).to_have_attribute("href", "https://github.com/Chikurin1014/serval-rs")
    expect(link).to_have_attribute("target", "_blank")
    layout = app.page.get_by_role("button", name="Layout").bounding_box()
    theme = app.page.get_by_role("button", name="Toggle theme").bounding_box()
    box = link.bounding_box()
    assert (box["width"], box["height"]) == (theme["width"], theme["height"])
    assert licenses["x"] + licenses["width"] <= box["x"] < layout["x"]

    # The tools: the layout menu, the theme switch, then plugins
    plugin = app.page.get_by_role("button", name="Import plugin").bounding_box()
    assert layout["x"] < theme["x"] < plugin["x"]
    # No tab for tasks until a plugin gives some
    expect(app.page.locator(".toolbar-tabs").get_by_role("tab")).to_have_text(
        ["Console", "Data", "Graph"]
    )

    # The layout menu opened to the left, within the window: once it has grown
    # to its size
    trigger = app.page.get_by_role("button", name="Layout")
    trigger.click()
    expect(app.page.get_by_role("option")).to_have_text(
        ["Import from JSON", "Export to JSON"]
    )
    content = app.page.locator(".layout-menu-content")
    content.evaluate("m => Promise.all(m.getAnimations().map(a => a.finished))")
    menu = content.bounding_box()
    button = trigger.bounding_box()
    assert menu["x"] + menu["width"] == pytest.approx(
        button["x"] + button["width"], abs=1
    )
    assert menu["x"] + menu["width"] <= app.page.viewport_size["width"]
