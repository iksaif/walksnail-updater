# Summary

<!-- What does this PR change, and why? -->

## Checklist

- [ ] `cargo fmt --all` and `cargo clippy --workspace -- -D warnings` pass
- [ ] `cargo test --workspace --exclude walksnail-updater` passes
- [ ] `npm run build` passes
- [ ] If you touched the firmware safety gates or the filename/version parsers, unit tests were added
- [ ] If you touched `crates/manuals/manifest.toml`, you ran `cargo run -p manuals --bin refresh` and committed the regenerated `src-tauri/resources/instructions.json`

## Disclaimer reminder

This project is not affiliated with Walksnail or CADDXFPV. PRs that add
branding, trademarks, or imply an official relationship will be rejected.
