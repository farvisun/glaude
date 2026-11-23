# Contributing

Thanks for helping. Bug reports, ideas and pull requests are all welcome.

## Set up

You need Rust (stable, 1.85+), [Bun](https://bun.sh) and the system
libraries Tauri 2 needs on your platform
([prerequisites](https://tauri.app/start/prerequisites/)).

```sh
cd ui && bun install && cd ..
bunx @tauri-apps/cli@2 dev
```

The UI also runs in a plain browser with a mock backend
(`cd ui && bun run dev`, then open http://localhost:1420), which is the
quickest way to work on screens.

## Before you open a pull request

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd ui && bun run check && bun run test
```

CI runs the same checks and builds the app on macOS, Windows and Linux.

- One topic per pull request, with a short description of what changed and
  how you checked it. Screenshots help for UI changes (light and dark, and
  right-to-left text where it matters).
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org)
  (`feat:`, `fix:`, `docs:`, ...).
- Keep right-to-left text working: anything that shows model output must
  keep per-block direction and must not apply negative letter spacing to
  Persian or Arabic text.

## Releases

Bump `version` in `Cargo.toml`, `src-tauri/tauri.conf.json` and
`ui/package.json`, add notes in `docs/releases/vX.Y.Z.md`, then push a tag
`vX.Y.Z`. The release workflow builds the installers, uploads them with
SHA-256 checksums and publishes the release.

By contributing you agree that your contributions are licensed under the
Apache License 2.0.
