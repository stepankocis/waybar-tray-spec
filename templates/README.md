<img src="assets/icon.svg" width="64" height="64" alt="">

# app-name

TODO: one paragraph. What the app shows in Waybar and where its data comes from.

- TODO: what a left click does.
- TODO: what each menu item does.
- TODO: what the user sees when the data source is unavailable.

## Install and run

```sh
cargo install --path . --locked
app-name
```

TODO: keep the snippet for the pattern the app uses, from `waybar/config.jsonc`
and `waybar/style.css`.

```jsonc
"modules-right": ["tray"],
"tray": { "icon-size": 20, "spacing": 8 }
```

Start it from the compositor's autostart and keep its log:

```sh
state="${XDG_STATE_HOME:-$HOME/.local/state}/app-name"
mkdir -p "$state"
app-name >>"$state/log.txt" 2>&1 &
```

The single-instance lock lives in `XDG_RUNTIME_DIR`, or under
`XDG_STATE_HOME/app-name` when that isn't set. Empty and relative XDG paths
count as unset. The app can start before Waybar and registers again when
Waybar restarts.

## Usage

```text
app-name [--verbose | -v]
```

- `--verbose` adds diagnostics on stderr.
- Exit codes: 0 on success, including when another instance already runs. 1 on
  a runtime error. 2 on an unknown argument.

## Footprint

Measured with the commands in `SPEC.md`, "Footprint targets", on TODO (CPU, kernel).

| Metric | Value | Target |
|---|---|---|
| Idle RSS (`VmRSS`) | TODO kB | ≤ 8528 kB |
| CPU over 60 s idle (`utime`+`stime`) | TODO ticks | 0 ticks |
| Runtime over 60 s idle (`schedstat`) | TODO ms | 0.00 ms |
| Wakeups over 60 s idle | TODO | 0 |
| Release binary | TODO bytes | ≤ 3 405 024 bytes |

RSS and binary targets are recommendations (SPEC.md §4). TODO: for any value
over its target, one line saying why, or delete this paragraph.

## Development

```sh
cargo fmt --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked
```
