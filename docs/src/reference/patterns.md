# Patterns

Regular expressions, used by:

* the maps that read text: **Decode**, **Match**, and **Replace** (see
  [Maps](maps.md)),
* the [label filters](#label-filters) of the data list.

## Syntax

That of the Rust [`fancy-regex`](https://docs.rs/fancy-regex) crate: the
[`regex`](https://docs.rs/regex/latest/regex/#syntax) crate's, with
look-around and backreferences.

| | |
|---|---|
| Groups | `(…)`, numbered from 1; `(?<name>…)` named |
| Case-insensitive | `(?i)…` |
| Look-ahead / look-behind | `(?=…)`, `(?!…)`, `(?<=…)`, `(?<!…)` |

An invalid pattern is shown with its error beside the field.

## Aliases

| Alias | Matches | Regex |
|---|---|---|
| `{number}` | A number: `20`, `-0.5`, `.5`, `+1.`, `1.5e-3` | `[+-]?(?:\d+\.?\d*\|\.\d+)(?:[eE][+-]?\d+)?` |
| `{word}` | The shortest run of non-spaces: it ends at the first separator after it | `\S+?` |
| `{level}` | A [log level](data.md#log-levels), in any case | `(?i:error\|warn(?:ing)?\|info\|debug\|trace)` |

* Each is replaced by its regex as a non-capturing group: capture it with
  `(…)`, as `({number})`.
* Not replaced when escaped (`\{number}`) or in a character class
  (`[{number}]`).

## Label filters

* A filter must match the **whole** label: `temp` matches `temp`, not
  `temp_1`; `temp.*` matches both.
* The data list shows the labels that match a **Show** filter (all, with none)
  and no **Hide** filter.
* The graphs plot only the labels the data list shows.
