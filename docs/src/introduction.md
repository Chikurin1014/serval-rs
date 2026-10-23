# Introduction

> [!WARNING]
> **This project is still in early development!** \
> It is not yet stable, and the API may change in the future.
>
> If you find a bug, or have a feature request,
> feel free to open an issue on [GitHub](https://github.com/Chikurin1014/serval-rs).

**Serval** is a multi-platform serial monitor.

<video src="media/demo.webm" poster="media/demo.png" autoplay loop muted playsinline controls width="100%"></video>

## Concept

Most of the previous serial monitors requires you to use a specific protocol
to communicate with your device and to visualize the data.
I've always been frustrated by this. \
**Why do I have to implement communication in a specific protocol just for debugging?**

That's why I created **Serval**.

## Features

### Multi platform

You can use Serval on Windows, Linux, macOS, and Web.

### Flexible data visualization

You can use regular expressions.

Serval contains data with types.
And it supports many Maps and Graphs for each type.

### Plugin system

You can add your own logic to Serval by writing a plugin,
which works like MACRO in Tera Term.
