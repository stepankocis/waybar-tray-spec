<img src="assets/icon.svg" width="64" height="64" alt="">

# waybar-tray-spec

A spec and starter templates for small Rust apps that live in the Waybar tray:
SNI tray items, Waybar `custom` modules, or both. They use `ksni`, `zbus` and
`tokio`. Idle CPU and wakeups must be zero. Memory, binary size and startup
time have recommended targets. The rules come from three working apps, and each
rule cites the line it came from.

## Usage

1. Clone this repo and `cd` into it.
2. Open your coding agent there (Claude Code, Codex, …).
3. Describe the app, for example "a tray icon that toggles Bluetooth".

The agent follows [`AGENTS.md`](AGENTS.md), builds the app from
[`templates/`](templates/) and checks it against [`SPEC.md`](SPEC.md).

## Contents

- [`SPEC.md`](SPEC.md): which pattern to pick, the stack, the footprint targets,
  runtime behaviour, the Waybar contract, testing, assets, and the
  definition of done.
- [`templates/`](templates/): `Cargo.toml`, `Cargo.lock`, `clippy.toml`, a
  `src/` skeleton with a Quit-only tray item, Waybar snippets and a README
  skeleton. They pass all the checks as they are.
- [`AGENTS.md`](AGENTS.md): how the agent works here (`CLAUDE.md` imports it).
