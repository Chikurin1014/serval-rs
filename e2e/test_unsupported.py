from playwright.sync_api import Browser, expect

from app import App

TITLE = "This browser cannot open serial ports"


def test_says_when_the_browser_has_no_web_serial(browser: Browser, base_url: str):
    page = browser.new_page(viewport={"width": 1280, "height": 800})
    page.add_init_script("delete Navigator.prototype.serial")
    app = App(page)
    page.goto(base_url)

    dialog = page.get_by_role("dialog", name=TITLE)
    expect(dialog).to_be_visible()
    expect(dialog).to_contain_text("Chrome, Edge")
    link = dialog.get_by_role("link")
    expect(link).to_have_attribute(
        "href", "https://developer.mozilla.org/docs/Web/API/Web_Serial_API"
    )
    expect(link).to_have_attribute("target", "_blank")
    dialog.get_by_role("button", name="Close").click()
    expect(dialog).to_be_hidden()
    expect(page.locator(".toolbar-tabs")).to_be_visible()
    page.close()
    assert app.errors == [], "uncaught errors in the page"


def test_says_nothing_where_web_serial_works(app: App):
    app.page.wait_for_timeout(300)
    expect(app.page.get_by_role("dialog", name=TITLE)).to_have_count(0)
