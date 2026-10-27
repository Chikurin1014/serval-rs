# Graphs

| Graph | Menu | Of | Settings |
|---|---|---|---|
| [Time series](#time-series) | Time series › Points / Linear / Stepped | Number | Value axis, Draw, Time axis |
| [Box plot](#box-plot) | Statistics | Number | Value axis |
| [Scatter](#scatter) | Statistics | Number × 2 | X label, Y label, X axis, Y axis, Regression |
| [Arrows (2D)](#arrows) | Vector | Vector2d | Trails |
| [Arrows (3D)](#arrows) | Vector | Vector3d | Trails |
| [Log](#log) | Text | Log | Weakest level |

## Common settings

* A graph plots the labels of its type that the data list shows (see
  [label filters](patterns.md#label-filters)).
* **Title**: empty for a numbered default.
* **Legend**: click a label to hide or show it.
* Click the title bar to open the settings; drag the handle to move the graph.
* A graph added from the menu starts with settings of its own: those below
  marked "from the menu". One of a [layout file](layout-file.md#graphs) takes
  the defaults there for the settings it leaves out.

| Axis scale | |
|---|---|
| Linear | Default |
| Log | Values of 0 or less are left out |

## Time series

| Setting | Values | Default |
|---|---|---|
| Value axis | Linear, Log | Linear |
| Draw | Points, Linear, Stepped | As picked from the menu |
| Time axis | 10–300 s, in steps of 10 s | 10 s |

* Time axis at 10 s fits the data instead, up to 10 s.
* Drag a rectangle to zoom in, which [pauses](#pausing) the graphs; double click
  to zoom out.

## Box plot

A box per label, of all the values it keeps.

| | |
|---|---|
| Box | From Q1 to Q3, with the median |
| Whiskers | The furthest values within 1.5 × IQR of the box (Tukey) |
| Outliers | The values past the whiskers, as points |
| Quartiles | By linear interpolation, as `QUARTILE.INC` |
| Hover | Mean, Variance (over n − 1), Median, Max, Min, Q3, Q1 |

## Scatter

| Setting | |
|---|---|
| X label / Y label | A Number label each |
| X axis / Y axis | Linear or Log |
| Regression | Shows the least-squares line and its equation. On, from the menu |

* A point pairs the newest X with the newest Y, once both have a new value.
* Up to 10,000 points, dropping the oldest.
* Shows the covariance (over n − 1) and Pearson's correlation coefficient, to
  two decimal places.

## Arrows

* An arrow per label, from the origin to its newest vector.
* **Trails**: the tips of its 50 values before. On, from the menu.

| | Arrows (2D) | Arrows (3D) |
|---|---|---|
| Hover | X, Y, Magnitude, Angle (from X toward Y) | X, Y, Z, Magnitude |
| Turning | | Drag |

## Log

* The logs of the labels shown, newest first.
* **Weakest level**: leaves out the logs of weaker levels. `INFO`, from the
  menu.
* Counts the logs of each level above the table (those left out, fainter).

## Pausing

| | |
|---|---|
| Pause button | Stops all the graphs at the data up to then |
| Play button | Brings them up to date |
| While paused | The data keep coming; settings still apply |
