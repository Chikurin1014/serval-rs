# Introduction

**Serval** is a multi-platform serial monitor.

> [!WARNING]
> **This project is still in early development!** \
> It is not yet stable, and the API may change in the future.
>
> If you find a bug, or have a feature request,
> feel free to open an issue on [GitHub](https://github.com/Chikurin1014/serval-rs).

## Concept

Most of the previous serial monitors requires you to use a specific protocol
to communicate with your device and to visualize the data.
I've always been frustrated by this. \
**Why do I have to implement communication in a specific protocol just for debugging?**

That's why I created **Serval**.


## Features

Multi platform
: Windows, Linux, macOS, and Web.

Flexible data visualization
: Using regular expressions. \
  Contains data with types, supports many Maps and Graphs for each type.

Plugin system
: You can add your own logic to Serval by writing a plugin. \
  It works like macro in Tera Term.
