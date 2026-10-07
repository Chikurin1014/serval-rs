# Development

The web crate defines the entrypoint for the web app along with any assets, components and dependencies that are specific to web builds. The web crate starts out something like this:

```
web/
├─ assets/ # Assets used by the web app - Any platform specific assets should go in this folder
├─ src/
│  ├─ main.rs # The entrypoint for the web app: its providers, routes and navbar
│  ├─ serial.rs, serial.js # The serial port, by Web Serial
│  ├─ time.rs # The clock, by the browser's
├─ Cargo.toml # The web crate's Cargo.toml - This should include all web specific dependencies
```

## Dependencies

This crate will only be included in the web build, so you should add all web specific dependencies to this crate's [Cargo.toml](../Cargo.toml) file instead of the shared [ui](../ui/Cargo.toml) crate.

### Serving Your Web App

You can start your web app with the following command:

```bash
dx serve
```
