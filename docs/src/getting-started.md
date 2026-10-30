# Getting Started

You can try **Serval** [on the web](https://serval-5qv.pages.dev).

This page shows Serval as it starts, with its default layout:
a device that prints lines such as `temp: 22.5` is shown as text,
as data, and as a graph without setting anything up.

> [!WARNING]
> Serval on the web runs in the browser, through the [Web Serial API].
> Use a browser that supports it, such as Chrome, Edge, or any other Chromium-based one.

[Web Serial API]: https://developer.mozilla.org/en-US/docs/Web/API/Web_Serial_API

## 1. Open a port

0. Click the plug icon button, and choose the device to allow Serval to access it.
1. Click the port selector, next to the plug icon button, and pick your device.
2. Type the baud rate the device uses (e.g. `115200`).
3. Click the open button beside it.

## 2. See what it sends

The **Console** tab shows what the device sends, as a terminal does.
What you type in it is sent to the device.

![The Console tab](media/getting-started-console.png)

## 3. See it as data

The **Data** tab lists the data Serval extracts from it, each under its own label.
With the default layout:

* each line goes to `log`,
* a line of `name: value` (e.g. `temp: 22.5`) goes to the label `name` as a
  number,
* a line starting with a number goes to `anonymous data`.

![The Data tab](media/getting-started-data.png)

> [!TIP]
> How the lines become data is set by the **maps** on the right of the tab. \
> See [Concept / Maps](concept/maps.md) for what the default ones do.

## 4. Plot it

The **Graph** tab plots the numbers against time.

![The Graph tab](media/getting-started-graph.png)

## Next steps

* [Usage](usage/index.md): each part of the app.
* [Concept](concept/index.md): how Serval turns what it receives into data.
