# CLAUDE.md

Guidance for future Claude sessions working on this repo. Humans reading this
should skim it too — it's the shortest path to making a correct change.

## What this app is

Cross-platform Tauri 2 desktop app that flashes Walksnail Avatar HD firmware
onto an SD card. Rust workspace + React/TS frontend.

**Unofficial.** Not affiliated with Walksnail or CADDXFPV. Do not add their
logos, do not imply endorsement, do not ship binaries from unverified sources.
Firmware downloads must come from the existing two sources (D3VL mirror +
CADDXFPV download center).

## Repo layout

```
crates/
  hardware/         pure lib: Hardware enum, Version, filename regex, safety gates
  firmware-index/   fetch + normalize D3VL JSON, scrape CADDXFPV fallback
  sdcard/           removable-drive watcher, multi-signal scan, stage+SHA1+copy
  manuals/          build-time: download PDFs, extract steps -> instructions.json
src-tauri/          Tauri app shell, #[command] handlers in src/commands.rs
src/                React frontend (views/, components/, lib/ipc.ts, state/store.ts)
src-tauri/resources/instructions.json   bundled step-by-step per hardware
crates/manuals/manifest.toml            source of truth for PDF URLs per hardware
```

Business logic lives in the crates so it stays unit-testable. `commands.rs`
is a thin IPC layer — don't put logic there.

## Always run

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --exclude walksnail-updater
npm run build
```

These are the gates CI enforces. Clippy is `-D warnings`, so a new warning
fails the build.

Tauri dev: `npm run tauri:dev` (`target/` is workspace root).
Tauri build: `npm run tauri:build`.

## Keeping upstream in sync

This is the section the project exists to make easy. Three kinds of upstream
change matter: **new hardware**, **new safety notice**, **new/updated manual
instructions**. Each has a narrow recipe.

### Firmware index refresh

The runtime fetches `firmwares.json` on every launch — no action needed when
a new firmware version ships. If the schema changes, adjust
`crates/firmware-index/src/d3vl.rs::RawRelease`/`RawDownload` + their tests.

### New hardware variant

Touch exactly these four places, in this order:

1. `crates/hardware/src/lib.rs`
   - Add a variant to `Hardware`.
   - Add its prefix to `filename_prefix()`, `display_name()`, `is_ground()`
     as needed, and to `Hardware::all()`.
   - Add the prefix to the `FILENAME_RE` alternation. **Order matters:** more
     specific prefixes (e.g. `AvatarMoonlight_Sky`) must appear *before*
     `Avatar_Sky` or the regex will match the wrong one.
   - Add the lowercase prefix branch to `parse_filename()`.
   - Add at least one row to the filename-regex test table and one to
     `canonical_filename_roundtrip`. Both tests are table-driven — one line
     each.
2. `crates/manuals/src/lib.rs::parse_hardware_key` — add the match arm.
3. `crates/manuals/manifest.toml` — add the upstream PDF URL. Then run
   `cargo run -p manuals --bin refresh` locally to regenerate
   `src-tauri/resources/instructions.json`. Commit both.
4. `src/lib/ipc.ts` — add the variant to the `Hardware` union, to
   `HARDWARE_LIST`, and to `HARDWARE_LABELS`.

The hardware tests will fail fast if the canonical filename regex doesn't
round-trip. If you see `parse_filename_table` red, you forgot step 1.

### New safety notice (brick advisory)

Encode it as a floor in `hardware::safety_check`, with a citation in the
reason string — PSA URL, wiki revision date, or forum thread. The `reason`
is what the UI shows the user, so spell out *why* they can't downgrade.

Pattern:

```rust
pub const SOMETHING_FLOOR: Version = Version::new(X, Y, Z);

// inside safety_check:
if hardware == Hardware::FooBar && target < SOMETHING_FLOOR {
    return SafetyVerdict::Block {  // or Warn, see below
        reason: format!(
            "FooBar units shipped after <date> require at least {SOMETHING_FLOOR}. \
             Source: <URL>. Refusing to stage this firmware."
        ),
    };
}
```

**Block vs Warn:**
- `Block` = permanent brick or unrecoverable state. Hardcoded — the UI
  won't even let you past the confirmation screen. Reserve for documented
  brick scenarios.
- `Warn` = likely to work but deserves prominence. The UI gates it behind a
  typed-string confirmation. Use for generic downgrades or when the advisory
  is batch-specific and we can't distinguish batches from software.

Add a test per verdict branch — the existing `safety_*` tests in
`crates/hardware/src/lib.rs` are the template.

**Mirror the gate in the frontend** (`src/views/UpdateWizard.tsx`). The Rust
gate is authoritative, but the wizard re-checks client-side for immediate
feedback — the `GOGGLES_X_FLOOR` constant at the bottom of that file is the
pattern. If you don't mirror it, the user sees "Stage" enabled and only
discovers the block after clicking.

### New / updated manual instructions

Two paths:

1. **Preferred — from upstream PDF.** Bump the URL in
   `crates/manuals/manifest.toml` if CADDXFPV moved the file, then run:
   ```bash
   cargo run -p manuals --bin refresh
   ```
   Review the diff in `src-tauri/resources/instructions.json` — PDF
   extraction is heuristic, so eyeball the steps before committing. The
   weekly `.github/workflows/refresh-manuals.yml` job does this automatically
   and opens a PR; you are reviewing that PR like any other.

2. **Manual override.** If the PDF extractor produces garbage (section
   heading changed, columns scrambled, etc.), edit the affected entry in
   `src-tauri/resources/instructions.json` by hand. Keep `source_url`
   pointing at the upstream manual; set `manual_sha256` to a descriptive
   marker like `"manual-override-YYYY-MM-DD"`. Future refreshes will *not*
   overwrite manually-edited entries as long as the PDF text fails to match
   the section regex — but add a test PDF or a comment noting the override.

Where users get the instruction they see in-app: the frontend loads
`instructions.json` once at startup (`load_instructions` command) and
`InstructionCard.tsx` renders the matching entry after a successful stage.

### Pulling in news

Watch these for safety advisories and new-hardware announcements:

- <https://walksnail.wiki/en/PSA> — community PSA page; source of the
  Goggles X / Avatar GT floors.
- <https://walksnail.wiki/en/firmware> — authoritative-ish changelog.
- <https://www.caddxfpv.com/pages/download-center> — official downloads +
  product launches.
- <https://intofpv.com> forum threads tagged Walksnail — early signal on
  bricks, often before official PSA.
- <https://github.com/D3VL/Avatar-Firmware-Updates> releases — first place
  new firmware JSON appears.

When you encode a new floor or hardware entry, put the source URL in the
commit message *and* in the Rust `reason` string.

## Invariants

Don't break these without explicit discussion:

- **Filename parser is the hardware identifier.** The app trusts the
  filename of the `.img` on an SD card to determine which device it's for.
  Weakening the regex (e.g. accepting partial matches) can cross-flash
  devices and brick them. If in doubt, add a test, don't relax the regex.
- **Safety gates are hardcoded, never fetched.** A `Block` verdict must
  survive an offline launch. The gates live in Rust source, not JSON.
- **SHA1 verification is mandatory when the upstream publishes a hash.**
  D3VL does; CADDXFPV doesn't. Do not silently accept a download with no
  hash when a hash was advertised — that's how supply-chain attacks land.
- **We never wildcard-delete on the SD.** `sdcard::stage` removes files
  only when `hardware::parse_filename` matches them. DVR recordings and
  user files stay untouched.
- **The app is unofficial.** Don't add branding that implies otherwise.
  The "Unofficial" banner, footer, and onboarding acknowledgement must
  stay.

## When things break

- Clippy fails on a new lint version: fix the lint; do not disable it.
- `parse_filename_table` red: you changed `FILENAME_RE` order — more-specific
  prefixes must come first.
- Tauri panics with "there is no reactor running": you spawned into Tokio
  from Tauri's `setup` closure. Use `tauri::async_runtime::spawn` instead,
  or return a future from the crate and let the app spawn it (see the
  `spawn_watcher` / `run_watcher` split in `crates/sdcard/src/watcher.rs`).
- `npm run build` fails on a type error from `@/lib/ipc`: the Rust response
  struct for a `#[command]` changed — keep `src/lib/ipc.ts` in lockstep.
  (The IPC types are duplicated by hand; there is no codegen. Yet.)

## Disclaimer

Not affiliated with Walksnail, CADDXFPV, or any related entity. All
trademarks belong to their owners. The app mirrors firmware from public
sources and does not modify it. Users accept the risk of flashing.
