# Data Tab

![The Data tab](../media/getting-started-data.png)

## Data list

Each row corresponds to a label and shows its [type](../concept/data-types.md), its newest value, and when that value arrived.

Click a row to see its earlier values.

| Button | |
|---|---|
| **Clear all** (top) | Forgets all the data |
| **Export as CSV** (bottom) | Saves the labels shown as a CSV file: a time and a value column for each |
| Delete (on a row) | Forgets the label's data |

### Label filters

A filter, in the bar below the list, is a regular expression matching the labels to **show** or **hide**.

The data list shows the labels that match a **Show** filter (all labels, if there are none) and match no **Hide** filter.

At first, `raw_bytes` is hidden.

> [!NOTE]
> The graphs plot only the labels shown in the data list, so the filters also
> choose what they plot.

## Maps

![A map's card](../media/data-map.png)

Each map is shown as a card. Turn it on with its switch, and click its title to see its
settings and the latest value it made.

> [!NOTE]
> Turn a map off to edit its settings.

> [!TIP]
> You can reorder maps by dragging their cards.

### Adding a map

Hover over the output type in the bar at the top, then pick a map.
Some come with presets, ready-made settings for common formats.

![Adding a map](../media/data-add-map.png)

See [Reference / Maps](../reference/maps.md) for each map.
