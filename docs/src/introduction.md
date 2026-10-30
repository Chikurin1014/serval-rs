# Introduction

> [!IMPORTANT]
> **This project is still in early development!** \
> It is not yet stable, and the API may change in the future.
>
> If you find a bug or have a feature request,
> feel free to open an issue on [GitHub](https://github.com/Chikurin1014/serval-rs).

**Serval** is a multi-platform serial monitor.

<video src="media/demo.webm" poster="media/demo.png" autoplay loop muted playsinline controls width="100%"></video>

## Concept

To extract and plot data, most existing serial monitors
require you to send it in a specific format, such as `label:data`, `>label:data`, and so on. \
**Why do I have to change my communication format just for debugging?**

That's why I made **Serval**.

## Features

### Multi-platform

Serval is designed to run on Windows, Linux, macOS, and the web.

Use it [on the web](https://serval-5qv.pages.dev), or [install the desktop app](reference/index.md#install).

### Flexible data format

You can use [regular expressions](reference/patterns.md) to extract data from serial input.

### Typed data

Serval stores data with types,
and provides many [Maps](concept/maps.md) and [Graphs](usage/graph.md) for
each type.

### Plugin system

You can add your own logic to Serval by writing a [plugin](concept/tasks.md),
which works like Tera Term's MACRO.
