from pathlib import Path

from playwright.sync_api import expect

from app import App

PLUGIN = """// A plugin of two tasks
const INTERVAL = 1000;

pub async fn blink(led) {
    helper().await;
    led + INTERVAL
}

// Not public: a helper, not a task
async fn helper() {}

pub async fn poll() {
    #{ ok: true }
}
"""


def import_plugin(app: App, path: Path):
    with app.page.expect_file_chooser() as chooser:
        app.page.get_by_role("button", name="Import plugin").click()
    chooser.value.set_files(path)


def home_tabs(app: App):
    return app.page.locator(".toolbar-tabs").get_by_role("tab")


def task_cards(app: App):
    return app.page.locator(".task-list .task-card")


def test_plugin_button_is_right_of_the_theme_switch(app: App):
    plugin = app.page.get_by_role("button", name="Import plugin")
    theme = app.page.get_by_role("button", name="Toggle theme")
    assert plugin.bounding_box()["x"] > theme.bounding_box()["x"]
    # No tab for tasks until a plugin gives some
    expect(home_tabs(app)).to_have_text(["Console", "Data", "Graph"])


def test_a_plugin_gives_a_task_of_each_public_async_function(app: App, tmp_path: Path):
    (tmp_path / "blinker.rn").write_text(PLUGIN)
    import_plugin(app, tmp_path / "blinker.rn")
    app.wait_for_toast("Plugin imported")
    assert app.toasts()[0] == ("success", "Plugin imported", "2 tasks of blinker.rn")
    # Its tab, the rightmost
    expect(home_tabs(app)).to_have_text(["Console", "Data", "Graph", "Task"])
    app.tab("Task")
    expect(task_cards(app).locator(".map-title-text")).to_have_text(["blink", "poll"])
    expect(task_cards(app).locator(".task-plugin")).to_have_text(["blinker.rn"] * 2)

    # Off at first: not run until turned on
    switch = task_cards(app).first.get_by_role("switch", name="Toggle task")
    expect(switch).not_to_be_checked()

    # Opened to its function's definition, scrolled within the card
    task_cards(app).nth(1).locator(".map-title").click()
    definition = task_cards(app).nth(1).locator(".task-definition")
    expect(definition).to_have_text("pub async fn poll() {\n    #{ ok: true }\n}")

    task_cards(app).first.hover()
    task_cards(app).first.get_by_role("button", name="Delete task").click()
    expect(task_cards(app).locator(".map-title-text")).to_have_text(["poll"])


def test_a_plugin_of_no_public_async_function_gives_none(app: App, tmp_path: Path):
    (tmp_path / "plain.rn").write_text("async fn private() {}\npub fn sync() {}\n")
    import_plugin(app, tmp_path / "plain.rn")
    app.wait_for_toast("Failed to import the plugin")
    assert app.toasts()[0][2] == "No `pub async fn` in plain.rn"
    expect(home_tabs(app)).to_have_count(3)


def compile_error_dialog(app: App):
    return app.page.get_by_role("dialog")


def test_a_file_not_of_rune_is_told_where_in_a_dialog(app: App, tmp_path: Path):
    (tmp_path / "broken.rn").write_text(
        "pub async fn ok() {}\npub async fn broken( {\n"
    )
    import_plugin(app, tmp_path / "broken.rn")
    dialog = compile_error_dialog(app)
    expect(dialog).to_be_visible()
    expect(dialog.get_by_role("heading")).to_have_text("Failed to compile broken.rn")
    expect(dialog.locator(".compile-error")).to_contain_text("broken.rn:2:")
    dialog.get_by_role("button", name="Close").click()
    expect(dialog).to_have_count(0)
    expect(home_tabs(app)).to_have_count(3)


def test_a_plugin_is_compiled_as_it_is_imported(app: App, tmp_path: Path):
    (tmp_path / "calls.rn").write_text(
        "pub async fn ok() {}\n\npub async fn calls() {\n    missing()\n}\n"
    )
    import_plugin(app, tmp_path / "calls.rn")
    told = compile_error_dialog(app).locator(".compile-error")
    # Each error, its line and the code there
    expect(told).to_contain_text("missing")
    expect(told).to_contain_text("calls.rn:4:")
    expect(told).to_contain_text("    missing()")
    # None of its tasks: it does not compile as a whole
    expect(home_tabs(app)).to_have_count(3)


def test_the_console_is_shown_as_the_last_task_goes(app: App, tmp_path: Path):
    (tmp_path / "one.rn").write_text("pub async fn only() {}\n")
    import_plugin(app, tmp_path / "one.rn")
    app.tab("Task")
    task_cards(app).first.hover()
    task_cards(app).first.get_by_role("button", name="Delete task").click()
    expect(home_tabs(app)).to_have_text(["Console", "Data", "Graph"])
    expect(home_tabs(app).first).to_have_attribute("data-state", "active")
    expect(app.page.locator(".console-output")).to_be_visible()


def test_a_function_of_a_plugin_is_imported_once(app: App, tmp_path: Path):
    (tmp_path / "blinker.rn").write_text(PLUGIN)
    import_plugin(app, tmp_path / "blinker.rn")
    app.wait_for_toast("Plugin imported")
    import_plugin(app, tmp_path / "blinker.rn")
    app.wait_for_toast("Plugin already imported")
    assert app.toasts()[0][2] == "All the tasks of blinker.rn"
    app.tab("Task")
    expect(task_cards(app).locator(".map-title-text")).to_have_text(["blink", "poll"])

    # Its new function alone, once more in it
    (tmp_path / "blinker.rn").write_text(PLUGIN + "pub async fn reset() {}\n")
    import_plugin(app, tmp_path / "blinker.rn")
    expect(task_cards(app).locator(".map-title-text")).to_have_text(
        ["blink", "poll", "reset"]
    )
    # Of another plugin, of the same names all the same
    (tmp_path / "other.rn").write_text(PLUGIN)
    import_plugin(app, tmp_path / "other.rn")
    expect(task_cards(app).locator(".task-plugin")).to_have_text(
        ["blinker.rn"] * 3 + ["other.rn"] * 2
    )


RUNNABLE = """pub async fn answer() { helper().await * 7 }
async fn helper() { 6 }
pub async fn greet() { `Hello, ${\"Serval\"}` }
pub async fn quiet() {}
pub async fn fails() { panic!(\"broken\") }
pub async fn spins() { loop {} }
pub async fn talks() { println!(\"first\"); println!(\"last {}\", 2); }
pub async fn reports() { println!(\"working\"); \"done\" }
"""


def run_task(app: App, index: int):
    """Turns the task on, to run: off again as it ends."""
    card = task_cards(app).nth(index)
    card.get_by_role("switch", name="Toggle task").click()
    expect(card.get_by_role("switch", name="Toggle task")).not_to_be_checked(
        timeout=30000
    )
    return card


def test_a_task_runs_as_it_is_turned_on_and_shows_what_it_returns(
    app: App, tmp_path: Path
):
    (tmp_path / "runnable.rn").write_text(RUNNABLE)
    import_plugin(app, tmp_path / "runnable.rn")
    app.tab("Task")

    expect(run_task(app, 0).locator(".task-latest")).to_have_text("42")
    expect(run_task(app, 1).locator(".task-latest")).to_have_text("Hello, Serval")
    # Nothing returned: nothing shown
    expect(run_task(app, 2).locator(".task-latest")).to_have_count(0)

    failed = run_task(app, 3).locator(".task-latest")
    expect(failed).to_have_attribute("data-failed", "true")
    expect(failed).to_contain_text("broken")
    # A loop of no await: stopped, not the page with it
    spun = run_task(app, 4).locator(".task-latest")
    expect(spun).to_have_text("Ran past 10000000 instructions without awaiting")

    # What it printed last, there as it returned nothing
    printed = run_task(app, 5).locator(".task-latest")
    expect(printed).to_have_text("last 2")
    expect(printed).to_have_attribute("data-printed", "true")
    # What it returned in place of what it printed
    returned = run_task(app, 6).locator(".task-latest")
    expect(returned).to_have_text("done")
    expect(returned).not_to_have_attribute("data-printed", "true")

    # Run again: what it returns anew
    expect(run_task(app, 0).locator(".task-latest")).to_have_text("42")


SERVAL = """use serval::{data, serial, time};

pub async fn waits() {
    let start = time::current_ms()?;
    time::wait_ms(300).await?;
    let until = time::current_ms()? + 200;
    time::wait_until(|| time::current_ms().map(|now| now >= until)).await?;
    time::current_ms()? - start
}

pub async fn keeps() {
    data::push("level", 1.5)?;
    data::push("level", 2)?;
    data::push("pos", (1, 2.5))?;
    data::push("acc", (0, 0, 9.8))?;
    data::push("note", "hello")?;
    data::push("alert", data::LogEntry::new(Some("WARN"), "hot"))?;
    let level = data::get("level");
    let first = level.next().await;
    // A stream of it: what comes after is in it
    data::push("level", 3)?;
    let rest = [level.next().await, level.next().await];
    let pos = data::get("pos").next().await;
    let alert = data::get("alert").next().await.unwrap();
    format!("{:?} {:?} {:?} {} {}", first, rest, pos, alert.level.unwrap(), alert.message)
}

pub async fn awaits_data() {
    // Not there yet: waited for
    data::get("ready").next().await.unwrap()
}

pub async fn talks() {
    serial::set_baudrate(115200)?;
    let baudrate = serial::get_baudrate()?;
    serial::send("ping\\n")?;
    serial::set_dtr(false).await?;
    serial::toggle_rts().await?;
    let lines = (serial::get_dtr()?, serial::get_rts()?);
    serial::close().await?;
    serial::open().await?;
    serial::restart("arduino").await?;
    format!("{:?} {:?}", baudrate, lines)
}

pub async fn sleeps() {
    time::wait_ms(60000).await?;
    "woke"
}

pub async fn follows() {
    data::push("count", 1)?;
    data::push("count", 2)?;
    let count = data::get_with_time("count");
    let first = count.next().await.unwrap();
    let second = count.next().await.unwrap();
    data::push("count", 3)?;
    let third = count.next().await.unwrap();
    format!("{} {} {} {}", first.1, second.1, third.1, first.0 <= third.0)
}

pub async fn collects() {
    let reading = data::get("reading");
    let sum = 0.0;
    while let Some(value) = reading.next().await {
        if value < 0.0 {
            break;
        }
        sum += value;
    }
    sum
}
"""


def test_tasks_reach_serval(app: App, tmp_path: Path):
    app.open_port()
    app.mock("mute()")
    (tmp_path / "serval.rn").write_text(SERVAL)
    import_plugin(app, tmp_path / "serval.rn")
    app.tab("Task")

    waited = run_task(app, 0).locator(".task-latest")
    expect(waited).not_to_have_attribute("data-failed", "true")
    assert int(waited.inner_text()) >= 500

    kept = run_task(app, 1).locator(".task-latest")
    expect(kept).to_have_text(
        "Some(1.5) [Some(2.0), Some(3.0)] Some((1.0, 2.5)) WARN hot"
    )
    app.tab("Data")
    app.wait_for_labels("level", "pos", "acc", "note", "alert")
    rows = app.data_rows()
    assert [rows[label][0] for label in ["level", "pos", "acc", "note"]] == [
        "Number",
        "Vector2d",
        "Vector3d",
        "Log",
    ]

    app.tab("Task")
    talked = run_task(app, 3).locator(".task-latest")
    expect(talked).to_have_text("Some(115200) (false, false)")
    assert "ping\n" in app.mock("written")
    # DTR off, RTS toggled; then open again, and the restart's steps
    assert app.mock("signals")[:2] == [
        {"dataTerminalReady": False},
        {"requestToSend": False},
    ]
    expect(app.page.get_by_role("button", name="Close port")).to_be_visible()
    # Arduino's restart: DTR and RTS off a moment, then on again
    assert app.mock("signals")[-2:] == [
        {"dataTerminalReady": False, "requestToSend": False},
        {"dataTerminalReady": True, "requestToSend": True},
    ]


def test_a_waiting_task_is_stopped_as_it_is_turned_off(app: App, tmp_path: Path):
    (tmp_path / "serval.rn").write_text(SERVAL)
    import_plugin(app, tmp_path / "serval.rn")
    app.tab("Task")
    card = task_cards(app).nth(4)
    switch = card.get_by_role("switch", name="Toggle task")
    switch.click()
    expect(switch).to_be_checked()
    app.page.wait_for_timeout(500)
    expect(switch).to_be_checked()
    switch.click()
    expect(switch).not_to_be_checked()
    app.page.wait_for_timeout(300)
    # Stopped: it never returned
    expect(card.locator(".task-latest")).to_have_count(0)


def test_a_task_waits_until_its_condition_is_met(app: App, tmp_path: Path):
    app.open_port()
    app.mock("mute()")
    (tmp_path / "serval.rn").write_text(SERVAL)
    import_plugin(app, tmp_path / "serval.rn")
    app.tab("Task")
    card = task_cards(app).nth(2)
    switch = card.get_by_role("switch", name="Toggle task")
    switch.click()
    # Waiting for the data, until it comes from the port
    app.page.wait_for_timeout(300)
    expect(switch).to_be_checked()
    app.mock("receive('ready: 7\\n')")
    expect(switch).not_to_be_checked()
    expect(card.locator(".task-latest")).to_have_text("7.0")


def test_a_stream_gives_each_entry_as_it_comes(app: App, tmp_path: Path):
    app.open_port()
    app.mock("mute()")
    (tmp_path / "serval.rn").write_text(SERVAL)
    import_plugin(app, tmp_path / "serval.rn")
    app.tab("Task")
    followed = run_task(app, 5).locator(".task-latest")
    expect(followed).to_have_text("1.0 2.0 3.0 true")

    card = task_cards(app).nth(6)
    switch = card.get_by_role("switch", name="Toggle task")
    switch.click()
    app.mock("receive('reading: 1\\n')")
    # Waiting for what comes next, the page going on meanwhile
    app.page.wait_for_timeout(300)
    expect(switch).to_be_checked()
    app.tab("Data")
    app.wait_for_labels("reading")
    app.tab("Task")
    app.mock("receive('reading: 2\\nreading: -1\\n')")
    expect(switch).not_to_be_checked()
    expect(card.locator(".task-latest")).to_have_text("3.0")
