# Graphs

| Graph                       | Menu                                    | Of         | Settings                                                 |
| --------------------------- | --------------------------------------- | ---------- | -------------------------------------------------------- |
| [Time series](#time-series) | Time series › Points / Linear / Stepped | Number     | Value axis, Draw, Time range                             |
| [Box plot](#box-plot)       | Statistics                              | Number     | Value axis, Time range                                   |
| [Scatter](#scatter)         | Statistics                              | Number × 2 | X label, Y label, X axis, Y axis, Regression, Time range |
| [Arrows (2D)](#arrows)      | Vector                                  | Vector2d   | Trails                                                   |
| [Arrows (3D)](#arrows)      | Vector                                  | Vector3d   | Trails                                                   |
| [Log](#log)                 | Text                                    | Log        | Weakest level                                            |

## Common settings

- A graph plots the labels of its type that the data list shows (see
  [label filters](patterns.md#label-filters)).
- **Title**: empty for a numbered default.
- **Legend**: click a label to hide or show it.
- Click the title bar to open the settings.
- With two or more graphs, drag the handle to move a graph, and click the button
  at its bottom right to show it alone or bring the others back.
- A graph added from the menu starts with settings of its own: those below
  marked "from the menu". One of a [layout file](layout-file.md#graphs) takes
  the defaults there for the settings it leaves out.

| Axis scale |                                  |
| ---------- | -------------------------------- |
| Linear     | Default                          |
| Log        | Values of 0 or less are left out |

## Time series

| Setting    | Values                     | Default                 |
| ---------- | -------------------------- | ----------------------- |
| Value axis | Linear, Log                | Linear                  |
| Draw       | Points, Linear, Stepped    | As picked from the menu |
| Time range | 10–300 s, in steps of 10 s | 10 s                    |
| All        | On, off                    | Off                     |

- The time axis fits the data, up to the time range: back from the newest
  value.
- **All**: all the data kept, the time range off.
- Drag a rectangle to zoom in, which [pauses](#pausing) the graphs; double click
  to zoom out.

## Statistics time range

Box plot and Scatter are of the values of their **Time range**:

|           |                                                                                                                                |
| --------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Values    | Those that arrived in the 10–300 s (in steps of 10 s) back from the newest                                                     |
| Default   | 10 s                                                                                                                           |
| Back from | The newest value the graph shows (as of the pause, while paused): of all its labels for Box plot, the newest point for Scatter |
| All       | All the values kept, the time range off. Off by default                                                                        |

- Values leave it only as newer ones arrive, not as time goes by.
- A longer time range, or **All**, brings back the older values still kept.

## Box plot

A box per label, of its values in the [time range](#statistics-time-range).

| Setting    | Values                     | Default |
| ---------- | -------------------------- | ------- |
| Value axis | Linear, Log                | Linear  |
| Time range | 10–300 s, in steps of 10 s | 10 s    |
| All        | On, off                    | Off     |

|           |                                                         |
| --------- | ------------------------------------------------------- |
| Box       | From Q1 to Q3, with the median                          |
| Whiskers  | The furthest values within 1.5 × IQR of the box (Tukey) |
| Outliers  | The values past the whiskers, as points                 |
| Quartiles | By linear interpolation, as `QUARTILE.INC`              |
| Hover     | Mean, Variance (over n − 1), Median, Max, Min, Q3, Q1   |

## Scatter

| Setting           | Values                                        | Default           |
| ----------------- | --------------------------------------------- | ----------------- |
| X label / Y label | A Number label each                           | Empty             |
| X axis / Y axis   | Linear, Log                                   | Linear            |
| Regression        | Shows the least-squares line and its equation | On, from the menu |
| Time range        | 10–300 s, in steps of 10 s                    | 10 s              |
| All               | On, off                                       | Off               |

- A point pairs the newest X with the newest Y, once both have a new value,
  at the time of the later.
- Shows the points of the [time range](#statistics-time-range), of the 10,000
  newest it keeps.
- From 1,000 points, each is drawn as a square of one color, without its
  outline, to draw them faster.
- Shows the covariance (over n − 1) and Pearson's correlation coefficient, to
  two decimal places.
- The regression line and the statistics are of the points shown alone.

## Arrows

- An arrow per label, from the origin to its newest vector.
- **Trails**: the tips of its 50 values before. On, from the menu.

|         | Arrows (2D)                              | Arrows (3D)        |
| ------- | ---------------------------------------- | ------------------ |
| Hover   | X, Y, Magnitude, Angle (from X toward Y) | X, Y, Z, Magnitude |
| Turning |                                          | Drag               |

## Log

- The logs of the labels shown, newest first.
- **Weakest level**: leaves out the logs of weaker levels. `INFO`, from the
  menu.
- Counts the logs of each level above the table (those left out, fainter).
- In a narrow graph, the levels show only their first letter (e.g. `E` for
  `ERROR`). In a narrower one, the Level column is hidden.

## Pausing

|              |                                             |
| ------------ | ------------------------------------------- |
| Pause button | Stops all the graphs at the data up to then |
| Play button  | Brings them up to date                      |
| While paused | The data keep coming; settings still apply  |
