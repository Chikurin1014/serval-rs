from playwright.sync_api import expect

from app import App


def test_licenses_open_in_a_new_tab(app: App):
    app.open_port()
    # The navbar's items are menu items (links underneath)
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

    # The app stays as it was, its port still open
    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()
