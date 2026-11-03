<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="pictures/logo-dark.svg">
    <img src="pictures/logo-light.svg" width="88" alt="">
  </picture>
</p>

<h1 align="center">Monitor Switcher for Ectomatter</h1>

<p align="center">Brightness, resolution, HDR and more, set for each game.</p>

This is a plugin for [Ectomatter](https://github.com/Ectonomic/Ectomatter), one app for the devices on your desk. A plugin brings something into Ectomatter that isn't a device, such as another program, the games on the computer or the weather. It runs inside Ectomatter as a WebAssembly module and reaches only what its manifest (`core.json`) names.

## Adding it

In Ectomatter's Settings, under Plugins, choose **Add a plugin**, then pick Monitor Switcher and press **Add**. Ectomatter shows what the plugin can reach before you add it, and takes the package only if it matches the checksum in Ectonomic's list of reviewed packages, [ectomatter-packages](https://github.com/Ectonomic/ectomatter-packages).

## Building it

The plugin is built inside Ectomatter's repository, which links this one at `plugins/monitor-switcher` (a Git submodule) and holds what every package is built from: the library of the core protocol's names (`crates/ectomatter-protocol`) and the interface between a module and Ectomatter (`wit/core.wit`).

```sh
git clone --recurse-submodules https://github.com/Ectonomic/Ectomatter
cd Ectomatter
rustup target add wasm32-wasip2
tools/build-module.sh plugins/monitor-switcher/module/Cargo.toml plugins/monitor-switcher/core.wasm
```

Ectonomic builds its releases with Rust 1.98.1.

- `core.json`, `glyph.svg`: the plugin as Ectomatter installs it, its manifest and the files it carries
- `module/`: the module's source
- `pictures/`: the logo at the top of this README, the plugin's icon in GitHub's text colors, made in Ectomatter's repository by `tools/plugin-logos.py`, and not part of the plugin

## License

Copyright (C) 2026 Ectonomic LLC. Made by Grant Garrison and Claude.

This plugin is free software under the GNU General Public License, version 3 only (`LICENSE`): you may share and change it under its terms. It comes with no warranty. The files it carries from others are under their own licenses, named in `REUSE.toml`, with their texts in `LICENSES/`.
