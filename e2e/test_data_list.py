import re

from playwright.sync_api import expect

from app import App


def test_empty_until_a_port_is_open(app: App):
    app.tab("Data")
    expect(app.page.locator(".data-grid .data-table")).to_be_visible()
    assert app.data_rows() == {}


def add_filter(app: App, pattern: str, by_enter: bool = False, kind: str = "Show"):
    toggle = app.page.locator(".label-filter-kind")
    if toggle.text_content() != kind:
        toggle.click()
    expect(toggle).to_have_text(kind)
    field = app.page.get_by_placeholder("Label filter")
    field.fill(pattern)
    if by_enter:
        field.press("Enter")
    else:
        app.page.get_by_role("button", name="Add filter").click()


def remove_filter(app: App, kind: str, pattern: str):
    tag = filter_tags(app).filter(has_text=re.compile(f"^{re.escape(pattern)}$"))
    tag.hover()
    tag.get_by_role("button", name=f"Remove {kind} filter {pattern}").click()


def filter_tags(app: App):
    return app.page.locator(".label-filter-tags [role=row]")


def wait_for_rows(app: App, *labels: str):
    """Waits until the data list shows `labels` and no others."""
    app.page.wait_for_function(
        """labels => {
            const shown = [...document.querySelectorAll('.data-table .data-row th')]
                .map(th => th.textContent);
            return shown.length === labels.length && labels.every(l => shown.includes(l));
        }""",
        arg=list(labels),
    )


def test_raw_bytes_are_hidden_by_default(app: App):
    app.open_port()
    app.tab("Data")
    expect(filter_tags(app)).to_have_text(["raw_bytes"])
    expect(filter_tags(app)).to_have_attribute("data-kind", "Hide")
    app.wait_for_labels("message")
    assert "raw_bytes" not in app.data_rows()


def test_shows_each_label_with_its_latest_value(app: App):
    app.open_port()
    app.tab("Data")
    remove_filter(app, "Hide", "raw_bytes")
    app.add_regex_map()
    app.wait_for_labels("raw_bytes", "message", "temp", "volt")
    rows = app.data_rows()
    # The time of day as the browser shows it, over the last few seconds
    recent = app.page.evaluate(
        "[0, 1, 2, 3].map(s => new Date(Date.now() - s * 1000).toLocaleTimeString('default'))"
    )

    assert {label: row[0] for label, row in rows.items()} == {
        "raw_bytes": "Bytes",
        "message": "String",
        "temp": "Number",
        "volt": "Number",
    }
    assert re.fullmatch(r"(temp|volt):[\d.]+", rows["message"][1])
    assert re.fullmatch(r"[\d.]+", rows["temp"][1])
    for label, (_, _, time) in rows.items():
        # With its milliseconds, as the locale writes them
        assert re.search(r":\d{2}[.,]\d{3}", time), f"{label} at {time} has no milliseconds"
        assert re.sub(r"([.,])\d{3}", "", time, count=1) in recent, f"{label} at {time} is not recent"


def test_deletes_a_label(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("temp", "volt")
    # Stop the stream first, or the next reading brings the label back
    app.page.get_by_role("button", name="Close port").click()
    expect(app.page.get_by_role("button", name="Open port")).to_be_visible()

    rows = app.page.locator(".data-grid .data-table tr.data-row")
    rows.filter(has=app.page.locator("th", has_text="volt")).get_by_role(
        "button", name="Delete label"
    ).click()

    expect(rows).to_have_count(2)
    assert set(app.data_rows()) == {"message", "temp"}


def test_filters_show_the_labels_any_of_them_match(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("message", "temp", "volt")

    add_filter(app, "temp")
    expect(filter_tags(app)).to_have_text(["raw_bytes", "temp"])
    expect(app.page.get_by_placeholder("Label filter")).to_have_value("")
    wait_for_rows(app, "temp")

    # Still not raw_bytes, which the default filter hides
    add_filter(app, "raw_.*|message", by_enter=True)
    expect(filter_tags(app)).to_have_text(["raw_bytes", "temp", "raw_.*|message"])
    wait_for_rows(app, "temp", "message")


def test_filters_match_whole_labels(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("message", "temp", "volt")
    # `e` is in message and temp, but neither is just `e`
    add_filter(app, "e")
    wait_for_rows(app)
    add_filter(app, ".*e.*")
    wait_for_rows(app, "message", "temp")


def test_a_filter_is_removed_from_its_tag(app: App):
    app.tab("Data")
    add_filter(app, "volt")
    tag = filter_tags(app).last
    remove = tag.get_by_role("button", name="Remove Show filter volt")
    app.page.mouse.move(0, 0)
    expect(remove).to_have_css("opacity", "0")
    tag.hover()
    expect(remove).to_have_css("opacity", "1")
    remove.click()
    expect(filter_tags(app)).to_have_text(["raw_bytes"])


def test_an_invalid_filter_shows_an_error(app: App):
    app.tab("Data")
    add_filter(app, "(")
    expect(app.page.locator(".label-filter-error")).to_contain_text("Invalid regex")
    expect(filter_tags(app)).to_have_text(["raw_bytes"])


def test_filters_stay_across_tabs(app: App):
    app.tab("Data")
    add_filter(app, "temp")
    app.tab("Console")
    app.tab("Data")
    expect(filter_tags(app)).to_have_text(["raw_bytes", "temp"])


def test_hide_filters_hide_what_they_match(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("message", "temp", "volt")

    add_filter(app, "message", kind="Hide")
    expect(filter_tags(app).last).to_have_attribute("data-kind", "Hide")
    wait_for_rows(app, "temp", "volt")

    # Shown if a Show filter matches, and no Hide filter does
    add_filter(app, "temp|message")
    wait_for_rows(app, "temp")

    remove_filter(app, "Hide", "message")
    wait_for_rows(app, "temp", "message")


def test_columns_and_rows_stay_put_as_values_change(app: App):
    app.open_port()
    app.tab("Data")
    remove_filter(app, "Hide", "raw_bytes")
    app.add_regex_map()
    app.wait_for_labels("raw_bytes", "message", "temp", "volt")
    layouts = set()
    for _ in range(20):
        layouts.add(
            app.page.evaluate(
                """() => JSON.stringify({
                    columns: [...document.querySelectorAll('.data-table thead th')]
                        .map(th => Math.round(th.getBoundingClientRect().left)),
                    rows: [...document.querySelectorAll('.data-table tbody tr')]
                        .map(tr => Math.round(tr.getBoundingClientRect().height)),
                })"""
            )
        )
        app.page.wait_for_timeout(50)
    assert len(layouts) == 1, layouts


def history_height(group) -> float:
    """How tall `group`'s history rows are, all together."""
    return group.evaluate(
        "g => [...g.querySelectorAll('.data-history')]"
        ".reduce((sum, tr) => sum + tr.getBoundingClientRect().height, 0)"
    )


def test_a_row_opens_to_its_recent_values(app: App):
    app.open_port()
    app.tab("Data")
    app.add_regex_map()
    app.wait_for_labels("temp")
    label = app.page.locator("th", has_text=re.compile("^temp$"))
    group = app.page.locator(".data-table .data-group").filter(has=label)
    row = group.locator(".data-row")
    toggle = row.locator(".data-row-toggle")
    # The values before the latest, there but of no height until expanded
    history = group.locator(".data-history")
    expect(toggle).to_have_attribute("aria-expanded", "false")
    expect(history).to_have_count(4)
    # At most the half of the row's border its first one shares
    assert history_height(group) < 1

    row.click()
    expect(toggle).to_have_attribute("aria-expanded", "true")
    expect(row).to_have_attribute("data-expanded", "true")
    expect(history.last).to_be_visible()

    # Newest first, from the row: each one's time no later than the one above it
    times = app.page.evaluate(
        """group => [...group.querySelectorAll('tr td:nth-child(4)')]
            .map(td => td.textContent)""",
        group.element_handle(),
    )
    assert len(times) == 5
    assert times == sorted(times, reverse=True), times
    for value in history.locator("td:nth-child(3)").all_text_contents():
        assert re.fullmatch(r"[\d.]+", value), value

    toggle.press("Enter")
    expect(toggle).to_have_attribute("aria-expanded", "false")
    app.page.wait_for_function(
        "g => [...g.querySelectorAll('.data-history')]"
        ".every(tr => tr.getBoundingClientRect().height < 1)",
        arg=group.element_handle(),
    )


def test_delete_button_shows_on_hover(app: App):
    app.open_port()
    app.tab("Data")
    app.wait_for_labels("message")
    row = app.page.locator(".data-table .data-row").filter(
        has=app.page.locator("th", has_text=re.compile("^message$"))
    )
    delete = row.get_by_role("button", name="Delete label")
    app.page.mouse.move(0, 0)
    expect(delete).to_have_css("opacity", "0")
    row.hover()
    expect(delete).to_have_css("opacity", "1")
