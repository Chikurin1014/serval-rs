# CSV Export

The **Export as CSV** button of the data list.

|           |                                                         |
| --------- | ------------------------------------------------------- |
| File name | `serval-<time>.csv` (`<time>`: ms since the Unix epoch) |
| Labels    | Those the data list shows, sorted by name               |
| Rows      | All the values each label keeps, oldest first           |

## Columns

```csv
timestamp-temp,value-temp,timestamp-volt,value-volt
1761450000123,22.5,1761450000123,3.31
1761450000223,22.6,,
```

- Two columns per label: `timestamp-<label>` and `value-<label>`.
- Row _n_ holds the _n_-th value of each label: the rows do not line up by
  time.
- A label with fewer values leaves its columns empty below them.
- Values are written as text, as in [Data / Types](data.md#types), numbers
  exact.
- A field with `,`, `"`, or a line break is quoted, its `"` doubled.

## Timestamps

Ms since the Unix epoch: `1761450000123`.
