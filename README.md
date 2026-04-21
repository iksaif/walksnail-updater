<p align="center">
  <img src="assets/logo.png" width="128" alt="Walksnail Updater logo"/>
</p>

<h1 align="center">Walksnail Updater</h1>

<p align="center">
  A small desktop app that flashes Walksnail Avatar HD firmware without the usual faff.
</p>

<p align="center">
  <a href="https://github.com/iksaif/walksnail-updater/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/iksaif/walksnail-updater/ci.yml?branch=main&label=CI" alt="CI"/></a>
  <a href="https://github.com/iksaif/walksnail-updater/releases"><img src="https://img.shields.io/github/v/release/iksaif/walksnail-updater?include_prereleases" alt="Latest release"/></a>
  <img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-0F172A" alt="Platforms"/>
  <img src="https://img.shields.io/badge/built%20with-Tauri%202-F59E0B" alt="Tauri"/>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License"/></a>
  <img src="https://img.shields.io/badge/unofficial-not%20affiliated-red" alt="Unofficial"/>
</p>

---

> **Unofficial.** Not affiliated with Walksnail or CADDXFPV. Trademarks belong
> to their owners. Flash at your own risk — bricks are not on me.

## What it does

Plug in your SD card. The app figures out which Walksnail you own, grabs the
latest firmware, drops it on the card with the right filename, and tells you
which buttons to hold. No SD plugged in? Download firmware anyway and drag
it across later.

- Recognises every variant by filename: `Avatar_Sky`, `AvatarX_Gnd`,
  `AvatarMini_Gnd`, `AvatarMoonlight_Sky`, etc.
- Reads DVR `.srt` sidecars to find the firmware actually running on the
  device (not just what's staged).
- Blocks the known bricks (Goggles X below `38.44.13`, Avatar GT below
  `39.44.2`) and makes you type out downgrade confirmations.
- Ships every release's changelog so you can pick a past version.
- Instructions come from the upstream product PDFs, parsed at build time.

## Screenshot

<!-- Replace with a real screenshot once the app has run on a machine. -->
<p align="center">
  <img src="assets/screenshot.png" width="720" alt="App screenshot (placeholder)"/>
</p>

## Install

Grab the file for your OS from the [Releases](../../releases) page:

- macOS: `.dmg`
- Windows: `.msi`
- Linux: `.AppImage` or `.deb`

Verify the SHA against the release asset, then run the installer.

## Build from source

```bash
npm ci
npm run tauri:dev      # run with HMR
npm run tauri:build    # produce installers in src-tauri/target/release/bundle
```

Linux also needs: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev`.

Rust tests:

```bash
cargo test --workspace --exclude walksnail-updater
```

Refresh instructions from upstream PDFs:

```bash
cargo run -p manuals --bin refresh
```

## Sources

- Firmware index: [D3VL/Avatar-Firmware-Updates](https://github.com/D3VL/Avatar-Firmware-Updates)
- Official downloads: [caddxfpv.com/pages/download-center](https://www.caddxfpv.com/pages/download-center)
- Wiki: [walksnail.wiki](https://walksnail.wiki/en/firmware)

## License

[MIT](LICENSE). No warranty.
