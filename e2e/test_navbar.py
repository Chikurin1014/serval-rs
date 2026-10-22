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


def github_link(app: App):
    return app.page.get_by_role("link", name="Serval on GitHub")


def test_the_repository_is_right_of_the_licenses(app: App):
    link = github_link(app)
    expect(link).to_have_attribute("href", "https://github.com/Chikurin1014/serval-rs")
    # In a new tab, so the open port stays
    expect(link).to_have_attribute("target", "_blank")
    licenses = (
        app.page.get_by_role("navigation", name="Pages")
        .get_by_role("menuitem", name="Licenses")
        .bounding_box()
    )
    layout = app.page.get_by_role("button", name="Layout").bounding_box()
    box = link.bounding_box()
    # An icon button, as the theme switch is
    theme = app.page.get_by_role("button", name="Toggle theme").bounding_box()
    assert (box["width"], box["height"]) == (theme["width"], theme["height"])
    assert licenses["x"] + licenses["width"] <= box["x"] < layout["x"]


def test_documents_are_right_of_the_title(app: App):
    pages = app.page.get_by_role("navigation", name="Pages")
    documents = pages.get_by_role("menuitem", name="Documents")
    expect(documents).to_have_attribute("href", "/docs")
    expect(documents).to_have_attribute("target", "_blank")
    title = pages.get_by_role("menuitem", name="Serval").bounding_box()
    licenses = pages.get_by_role("menuitem", name="Licenses").bounding_box()
    box = documents.bounding_box()
    assert title["x"] + title["width"] <= box["x"]
    assert box["x"] + box["width"] <= licenses["x"]
