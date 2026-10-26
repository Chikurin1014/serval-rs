# Runtime

## Running a task

| | |
|---|---|
| Task | Each top-level `pub async fn` that takes no arguments |
| Start | Turning its switch on: the function is called |
| End | It returns, or an error stops it; the switch turns off |
| Stop | Turning its switch off, or deleting the task |
| Where | In the page, alongside the app |

Each task runs alone: turning one on again starts it anew.

## Output

| | |
|---|---|
| `println!` | To **STDOUT**, as `<file>::<function> (HH:MM:SS.mmm)` and the line |
| Card | The last line it printed, while it runs |
| STDOUT | The last 10,000 lines of all tasks; the brush button clears it |

## Return values

| Returned | Card |
|---|---|
| A value | Formatted with `{}`, or `{:?}` if it has no `Display` |
| `()` | The last line it printed |
| An error (e.g. of `?`) | The error |

## Limits

| | |
|---|---|
| Instructions between two `.await`s | 10,000,000; past it, the task is stopped |
| Waiting (`time::sleep`, streams, `serial::flush`, …) | Checked every 10 ms |
| `serial::wait_for` | Keeps the last 1,000 bytes unless given a limit |

A loop with no `.await` in it would freeze the page: the limit stops it.

## Importing a plugin

| Result | Toast |
|---|---|
| Imported | `Plugin imported`: how many tasks |
| Imported again (same file name) | `Plugin reimported`: its tasks replace all those of that file |
| Functions with arguments | Listed as not taken (`not of …: taking arguments`) |
| No `pub async fn` | `Failed to import the plugin` |
| A compile error | A dialog with each error, its line, and the code there |

New tasks start off.
