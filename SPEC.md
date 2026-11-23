# SPEC: small Rust apps for the Waybar tray

The rules for every app built from this repo. `AGENTS.md` says how to work and
this file says what to build. Sections marked **(reference)** hold tables and
snippets. Skip them until a task needs them.

Every rule cites the reference app it comes from, as `repo/path:line`:

| Repo | Pattern | Commit read |
|---|---|---|
| `powerprofilesctl-tray` | SNI tray (`ksni`) + system D-Bus client (`zbus`) | `1f45ce0` |
| `yandex-tray-rs` | SNI tray (`ksni`) + process control | `8ee9895` |
| `countdown_class` | Waybar `custom` module (`--waybar` JSON) + session D-Bus service | `ae9a582` |

Rules marked *(measured)* come from the step-1 measurements in §4, not from an
app. `ksni/…` points into the `ksni` crate source in `~/.cargo/registry`.

## Contents

1. Pattern decision table
2. Stack
3. Release profile
4. Footprint budget
5. Event-driven rules
6. Project layout
7. Runtime behaviour
8. Waybar contract
9. Testing
10. Assets
11. README template
12. Definition of done
13. Out of scope

## 1. Pattern decision table

| The app needs | Use | Why | Source |
|---|---|---|---|
| An icon whose state changes, plus a click menu (toggle, pick one of N, actions) | **SNI tray item** | Waybar's `tray` module draws it, and Waybar needs no config beyond `"tray"`. One resident process owns the state and the menu. | `powerprofilesctl-tray/src/tray.rs:110-158`, `yandex-tray-rs/src/tray.rs:63-97` |
| Text or a number in the bar (a countdown, a value, a percentage), styled per state | **Waybar `custom` module** (`--waybar`) | Only a custom module shows text and gets a CSS `class`. SNI items show an icon only. | `countdown_class/src/waybar.rs:11-17`, `countdown_class/example/style.css:1-30` |
| Text in the bar **and** a menu or click actions | **Both**: one binary, a tray mode and a `--waybar` mode | Each mode reads the data source itself. If only the resident process has the data, `--waybar` asks it over the app's session D-Bus interface. | `countdown_class/src/dbus_service.rs:71-80` (`GetWaybarOutput`) |

Tie-breakers:

- Prefer the SNI tray item when an icon is enough. A resident SNI process can
  idle at 0 wakeups (§4). A `custom` module costs a process per refresh, or a
  second resident process (§8).
- Write the choice and the reason into `PROGRESS.md` (`AGENTS.md:20-22`).

## 2. Stack

Fixed by `AGENTS.md:47-59`. These are the exact versions:

| Crate | `Cargo.toml` line | When | Source |
|---|---|---|---|
| `ksni` | `{ version = "0.3.6", default-features = false, features = ["tokio"] }` | SNI tray item | `yandex-tray-rs/Cargo.toml:9` (0.3.6), `ksni-0.3.7/Cargo.toml` `[features]` (`default = ["tokio"]`) |
| `tokio` | `{ version = "1", default-features = false, features = ["macros", "rt", "signal", "sync"] }`. Add `time` for sleeps and timeouts, and `process` for child processes. | any long-running mode | `powerprofilesctl-tray/Cargo.toml:13`, `yandex-tray-rs/Cargo.toml:11` |
| `zbus` | `{ version = "5.19", default-features = false, features = ["tokio"] }` | talking to a D-Bus service, or serving one | `powerprofilesctl-tray/Cargo.toml:14` |
| `futures-util` | `{ version = "0.3", default-features = false }` | `StreamExt` on zbus signal streams | `powerprofilesctl-tray/Cargo.toml:9` |
| `libc` | `{ version = "0.2", default-features = false }` | `O_NOFOLLOW` for the lock file | `powerprofilesctl-tray/src/main.rs:91` |
| `serde`, `serde_json` | `serde = { version = "1", default-features = false, features = ["derive"] }`, `serde_json = { version = "1", default-features = false, features = ["std"] }` | `--waybar` JSON | `countdown_class/Cargo.toml:10-14` |
| dev only: `tokio` | `features = ["io-util", "net"]` | private-bus tests | `powerprofilesctl-tray/Cargo.toml:16-17` |

Any other crate is a new dependency: ask first (`AGENTS.md:33-34`). It needs
`default-features = false` and a reason in the commit message (`AGENTS.md:58-59`).

`Cargo.lock` is committed, and every check runs with `--locked`
(`powerprofilesctl-tray/CLAUDE.md:12-17`). The template's lock pins
`ksni 0.3.6`, `tokio 1.54.1`, `zbus 5.19.0` and `libc 0.2.191`.

Where the apps disagree, this spec decides:

| Topic | Apps | Chosen | Reason |
|---|---|---|---|
| Edition | 2024 (`powerprofilesctl-tray/Cargo.toml:4`, `yandex-tray-rs/Cargo.toml:4`), 2021 (`countdown_class/Cargo.toml:4`) | **2024** | Fixed by `AGENTS.md:54`; two of three apps already use it. |
| MSRV | 1.90 (`powerprofilesctl-tray/Cargo.toml:5`, `countdown_class/Cargo.toml:5`), 1.89 (`yandex-tray-rs/Cargo.toml:5`) | **1.90** | Fixed by `AGENTS.md:54`. `File::try_lock` needs ≥ 1.89 (`yandex-tray-rs/AGENTS.md:64`). |
| `zbus` | 5.19 (`powerprofilesctl-tray/Cargo.toml:14`), 4.0 (`countdown_class/Cargo.toml:10`) | **5** | `ksni` already depends on zbus 5, so zbus 4 would build a second D-Bus stack (`ksni-0.3.7/Cargo.toml` `[dependencies.zbus]`). |
| `ksni` | 0.3.7 (`powerprofilesctl-tray/Cargo.toml:10`), 0.3.6 (`yandex-tray-rs/Cargo.toml:9`) | **0.3.6** | *(measured)* 0.3.7 makes the template binary 57 KB bigger (1 727 512 vs 1 669 832 bytes) and over the §4 size budget. Its only addition, `current_xdg_activation_token` (`ksni-0.3.7/CHANGELOG.md:1-4`), isn't used by any app. |
| `ksni` features | default (`yandex-tray-rs/Cargo.toml:9`) | `default-features = false, features = ["tokio"]` | Same feature set, written out as `AGENTS.md:58` requires. |
| Release profile | full (`yandex-tray-rs/Cargo.toml:13-18`), partial (`powerprofilesctl-tray/Cargo.toml:19-21`), none (`countdown_class/Cargo.toml`) | full, §3 | §3 |
| Lints | full (`powerprofilesctl-tray/Cargo.toml:23-29`), pedantic only (`countdown_class/Cargo.toml:16-17`), none (`yandex-tray-rs/Cargo.toml`) | full, below | `AGENTS.md:56-57` |
| Tray icon pixmap | PNG decoded at startup with the `png` crate (`powerprofilesctl-tray/src/tray.rs:202-222`), raw ARGB bytes (`yandex-tray-rs/src/tray.rs:102-104`) | **raw ARGB** | Dropping the PNG decoder took the yandex binary from 3.5 to 1.7 MB (`yandex-tray-rs` commit `7440316`). |

Lints and clippy config, verbatim from `powerprofilesctl-tray/Cargo.toml:23-29`
and `powerprofilesctl-tray/clippy.toml:1-2`:

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
```

```toml
# clippy.toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

`-D warnings` in the checks turns these warnings into errors
(`powerprofilesctl-tray/CLAUDE.md:39-41`). Use `#[expect(lint, reason = "…")]`
only where the lint is wrong for that line
(`powerprofilesctl-tray/src/tray.rs:174-177`).

## 3. Release profile

Use this unchanged (`AGENTS.md:91`). It is `yandex-tray-rs/Cargo.toml:13-18`:

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = "symbols"
```

*(measured)* These builds used the profile above, changing only `opt-level`:

| App | no profile | `opt-level = 3` | `"s"` | `"z"` |
|---|---|---|---|---|
| `countdown_class` | 4 729 616 B | 2 121 952 B | 1 461 424 B | 1 344 440 B |
| `powerprofilesctl-tray` | — | 2 540 312 B | 1 887 784 B | 1 772 704 B |
| `countdown_class --waybar`, hyperfine mean | 741 µs | 727 µs | 710 µs | 718 µs |

`"z"` is 6–8 % smaller. Startup didn't change beyond noise (σ ≈ 170 µs), and
peak RSS was 0.1 MiB higher. That isn't a clear win, so `"s"` stays, as in
both apps that set a profile.

`panic = "abort"` has one consequence: `println!` panics when stdout is a
closed pipe, so the panic aborts the process. Long-running output must use
`write!` and handle the error (§8).

## 4. Footprint budget

A new app must match or beat the **best** column on every metric
(`AGENTS.md:72-100`). Record its numbers in the README's "Footprint" section.

### Baseline (step-1 measurements)

Measured on 2026-11-23 with the commands below. Setup: AMD Ryzen 7 8845HS,
Linux 7.2.15-zen1, rustc 1.100.0. Each app was a release build of the commit
in the table at the top, run under `dbus-run-session` with temporary XDG dirs
and no SNI watcher on the private bus.

| Metric | `powerprofilesctl-tray` | `yandex-tray-rs` | `countdown_class` (daemon) | **Best (budget)** |
|---|---|---|---|---|
| Idle RSS (`VmRSS`, 5 s after start) | 5 636 kB | 4 264 kB | 5 168 kB | **4 264 kB** |
| CPU over 60 s idle (`utime`+`stime`) | 0 ticks | 0 ticks | 0 ticks | **0 ticks** |
| Runtime over 60 s idle (`schedstat`) | 0.00 ms | 3.67 ms | 5.55 ms | **0.00 ms** |
| Wakeups over 60 s idle (context switches) | 0 | 129 | 129 | **0** |
| Release binary | 3 709 256 B | 1 702 512 B | 4 729 616 B | **1 702 512 B** |
| Short-lived mode (`--waybar`, hyperfine mean) | — | — | 741 µs ± 160 µs (peak 3.4 MiB) | **741 µs** |

Notes:

- `powerprofilesctl-tray` read the real system bus (read-only) during the
  measurement. `yandex-tray-rs` found the running browser and pinned its pidfd.
- 1 tick is 10 ms, so the tick count can't show small loads. `schedstat`
  (nanoseconds) and context switches can, and they expose the 1 s ticks (§5).
- The template in `templates/` measures 4 196 kB, 0 ticks, 0.00 ms, 0 wakeups
  and 1 669 832 B, so it fits the budget with room for the app's logic.
- A **time-driven display** (a countdown in seconds) can't be at 0 wakeups. Its
  budget is **at most one wakeup per displayed change**. Measure it during a
  60 s window and compare against the number of lines printed. The §8 shape-L
  snippet measured 60 wakeups in 60 s of per-second changes. The old 1 s tick
  measured 129.

### Commands (reference)

Save this script as `target/footprint.sh`. It's git-ignored, so it stays out of
the app. Run it from the repo root as
`sh target/footprint.sh NAME "$PWD/target/release/NAME" [args…]`. It starts the
binary on a private session bus with absolute XDG dirs under
`target/smoke/NAME`, waits 5 s, samples twice 60 s apart, and stops only the
process it started (`AGENTS.md:102-115`).

```sh
#!/bin/sh
# Usage: footprint.sh NAME /abs/path/to/release/binary [args...]
set -eu
NAME=$1; shift
X=$PWD/target/smoke/$NAME
mkdir -p "$X/run" "$X/config" "$X/cache" "$X/state" "$X/data"
chmod 700 "$X/run"
rm -f "$X/pid"
env XDG_RUNTIME_DIR="$X/run" XDG_CONFIG_HOME="$X/config" \
    XDG_CACHE_HOME="$X/cache" XDG_STATE_HOME="$X/state" XDG_DATA_HOME="$X/data" \
    dbus-run-session -- sh -c 'echo $$ >"$0"; exec "$@"' "$X/pid" "$@" \
    >"$X/out.log" 2>&1 &
sleep 5
P=/proc/$(cat "$X/pid")
# Per thread: "tid ctxt_switches runtime_ns"; threads that exit are counted
# up to their last sample, new threads from zero.
snap() {
    for t in "$P"/task/*; do
        echo "${t##*/} $(awk '/ctxt_switches/ {s += $2} END {print s}' "$t/status") $(cut -d' ' -f1 "$t/schedstat")"
    done
}
grep -E '^(VmRSS|Threads)' "$P/status"
c0=$(awk '{print $14 + $15}' "$P/stat"); snap >"$X/s0"
sleep 60
c1=$(awk '{print $14 + $15}' "$P/stat"); snap >"$X/s1"
awk 'NR == FNR {w[$1] = $2; r[$1] = $3; next}
     {dw += $2 - w[$1]; dr += $3 - r[$1]}
     END {printf "wakeups_60s:   %d\nruntime_60s:   %.2f ms\n", dw, dr / 1e6}' "$X/s0" "$X/s1"
echo "cpu_ticks_60s: $((c1 - c0))"
kill "$(cat "$X/pid")"; wait
echo "binary_bytes:  $(stat -c %s "$1")"
```

Short-lived modes, with the same XDG variables exported:

```sh
hyperfine -N --warmup 20 --runs 300 'target/release/NAME --waybar'
```

If the app needs config, copy a fixture into `target/smoke/NAME/config/NAME/`
before you run either command.

## 5. Event-driven rules

Wait on an event and never poll on a fixed tick (`AGENTS.md:79-85`). Pick the
source by the kind of data:

| Data | Event source | Source |
|---|---|---|
| Properties of a D-Bus service | `PropertiesChanged` stream filtered to the service, `CacheProperties::No`, then one `GetAll` on change | `powerprofilesctl-tray/src/profiles.rs:175-200`, `:227-262` |
| The service stops or restarts | `NameOwnerChanged` for its name: no owner means show "stopped", a new owner means read again | `powerprofilesctl-tray/src/profiles.rs:186`, `:227-262`, `:265-276` |
| Waybar or the SNI watcher restarts | Nothing to write. `ksni` watches `NameOwnerChanged` for `org.kde.StatusNotifierWatcher` and registers again | `ksni-0.3.7/src/service.rs:131`, `:148`; `powerprofilesctl-tray/README.md:61` |
| The tray service died | `events.recv()` returns `None` (the tray owns the only sender), then `handle.is_closed()` | `powerprofilesctl-tray/src/main.rs:175-177`, `:195-196`; `ksni-0.3.7/src/lib.rs:545-547` |
| A child the app spawned exits | `tokio::process::Child::wait()` in a spawned task | `yandex-tray-rs/src/browser.rs:94-113` |
| A process the app didn't spawn exits | Its pidfd, readable through `AsyncFd`. That needs `unsafe` (`pidfd_open`, `AsyncFd::register`), which `unsafe_code = "forbid"` blocks, so ask the user | `yandex-tray-rs/src/browser.rs:55-64`, `yandex-tray-rs/src/main.rs:180-197` |
| A process the app didn't spawn starts | No unprivileged event source exists. Scan `/proc` only while nothing is pinned, or on menu open (`Tray::menu_about_to_show`) | `yandex-tray-rs/src/main.rs:108-109`; `ksni-0.3.7/src/lib.rs:232` |
| Config files | inotify needs a crate outside the stack, so ask. Without it, re-read on an explicit trigger (a D-Bus `Update()` method or a menu item) | `countdown_class/src/dbus_service.rs:71-80` |
| Wall-clock text (countdowns, clocks) | Sleep until the shown text next changes: the next whole second or minute of the remaining time, or the next schedule boundary | `countdown_class/src/schedule.rs:107-124` |
| Timeouts, retries, "still running" notices | A `sleep_until(deadline)` branch in the same `select!`. Retry backoff goes 5 → 10 → 20 → 30 s | `powerprofilesctl-tray/src/main.rs:27-47`, `:167`; `powerprofilesctl-tray/src/profiles.rs:14` |

Also:

- Call `handle.update` only when the visible state changed, and log an error
  only when its message changed (`yandex-tray-rs/src/main.rs:146-151`,
  `powerprofilesctl-tray/src/tray.rs:53-59`,
  `countdown_class/src/dbus_service.rs:106-110`).
- A D-Bus service publishes only the properties that changed
  (`countdown_class/src/dbus_service.rs:118-130`).
- If you must poll, add a one-line comment saying why no event exists
  (`yandex-tray-rs/src/main.rs:108`, `AGENTS.md:80-82`).

### Are the existing 1 s ticks justified?

**`yandex-tray-rs/src/main.rs:100-101`: no.** The tick does three jobs:

- **The tray-death check** (`:107`). Replace it with `events.recv()` returning
  `None` (row "The tray service died").
- **The 5 s "still running" notice** (`:165-172`). Replace it with a
  `sleep_until` branch.
- **The `/proc` scan while no browser is pinned** (`:108-109`). It has no
  event source, but what it changes (the menu enablement and the tooltip) only
  shows when the user looks. Rescan in `menu_about_to_show` instead.

*(measured)* The tick costs 129 wakeups and 3.67 ms of CPU a minute at idle.

**`countdown_class/src/dbus_service.rs:152-157` and
`countdown_class/example/waybar-module.json:6`: partly.** The text shows
seconds during school hours, so 1 Hz updates are needed then. Three things are
wrong:

- `interval(1s)` drifts against the wall-clock second. Sleep until the next
  whole second of the remaining time.
- Outside school hours the text doesn't change, but the tick keeps running.
  Sleep until the next lesson boundary.
- Config is re-checked by `stat` on every tick (`countdown_class/src/config.rs:348-364`).
  Use the row "Config files" instead.

The worst cost is Waybar's `"interval": 1`, which starts a new process every
second. Replace it with refresh shape L (§8). *(measured)* The daemon costs
129 wakeups and 5.55 ms of CPU a minute, plus one 741 µs spawn a second.

## 6. Project layout

`templates/` holds the first five files below. Add the others only when the app
needs them.

| File | Holds | Source |
|---|---|---|
| `src/main.rs` | Argument parsing (before anything else), runtime construction, the instance lock, signals, the event loop. `APP = env!("CARGO_PKG_NAME")`. | `powerprofilesctl-tray/src/main.rs:49-198` |
| `src/tray.rs` | The `ksni::Tray` impl, the `Action` enum, the menu. Holds only the state it shows. Menu callbacks send an `Action` over an unbounded mpsc channel. | `powerprofilesctl-tray/src/tray.rs:19-160`, `yandex-tray-rs/src/tray.rs:28-30` |
| `src/error.rs` | `pub type Error = Box<dyn std::error::Error + Send + Sync>;` | `powerprofilesctl-tray/src/error.rs:1` |
| `src/log.rs` | `log()` always writes to stderr, prefixed with the app name. `debug()` writes only with `--verbose`. | `powerprofilesctl-tray/src/log.rs:3-5`, `countdown_class/src/app.rs:6-12` |
| `src/paths.rs` | Absolute-only XDG resolution | `powerprofilesctl-tray/src/paths.rs:8-25` |
| `src/<source>.rs` | One module per data source: a D-Bus client (`profiles.rs`), process control (`browser.rs`) or a D-Bus service (`dbus_service.rs`). Its domain error is a hand-written enum with `Display`. | `powerprofilesctl-tray/src/profiles.rs:18-72`, `yandex-tray-rs/src/browser.rs`, `countdown_class/src/dbus_service.rs` |
| `src/waybar.rs` | The `--waybar` line (§8) | `countdown_class/src/waybar.rs:11-55` |
| `src/<logic>.rs` | Pure logic, with the clock or the input passed in: `compute_state(now, &config)` | `countdown_class/src/schedule.rs:134`, `countdown_class/AGENTS.md:81-85` |
| `src/config.rs` | Config loading and validation, only if the user asked for config | `countdown_class/src/config.rs:175-305` |
| `tests/cli.rs` | Integration tests of the binary's CLI and D-Bus contract | `countdown_class/tests/cli.rs` |
| `assets/` | Icons and `assets/README.md` (§10) | `powerprofilesctl-tray/assets/README.md` |
| `waybar/` | The `config.jsonc` and `style.css` snippets from the template, cut down to the app's pattern | `countdown_class/example/` |

After copying `templates/` and renaming the crate in `Cargo.toml`, run
`cargo update --workspace`. It rewrites only the root package's entry in
`Cargo.lock`, so the `--locked` checks pass again *(measured)*.

## 7. Runtime behaviour

`templates/src/main.rs` implements all of this. Copy it; don't re-derive it.

- **Arguments first.** Parse arguments in a plain `fn main()`, then build the
  runtime with `tokio::runtime::Builder::new_current_thread().enable_all()`.
  Short-lived modes return before the runtime exists (`countdown_class/src/app.rs:32-46`,
  `AGENTS.md:86-88`). `#[tokio::main]` builds the runtime before parsing, so
  don't use it (`powerprofilesctl-tray/src/main.rs:49`).
- **Flags.** `-h`/`--help` prints usage and exits 0. `-v`/`--verbose` turns on
  `debug()`. Any other argument exits 2 (`powerprofilesctl-tray/src/main.rs:51-66`,
  `countdown_class/src/main.rs:13-20`, `:49-59`). Add more flags or
  subcommands (`--waybar`, `config check`) only if the app needs them
  (`countdown_class/src/main.rs:25-44`).
- **Single-instance lock.** The lock file is `$XDG_RUNTIME_DIR/<app>.lock`,
  falling back to `$XDG_STATE_HOME/<app>/<app>.lock`. It is opened with mode
  `0600` and `O_NOFOLLOW`, then `File::try_lock()`. If another instance holds
  it, log "already running" and exit 0. Take the lock before installing signal
  handlers and before touching D-Bus (`powerprofilesctl-tray/src/main.rs:73-109`,
  `yandex-tray-rs/src/main.rs:49-77`, `powerprofilesctl-tray` commit `edd1add`).
  A D-Bus service also relies on its bus name being unique
  (`countdown_class/src/dbus_service.rs:171-175`).
- **Signals and shutdown.** Install SIGTERM and SIGINT with
  `tokio::signal::unix::signal` and pin one `shutdown` future. Race it with
  `biased` `select!` against tray registration and the event loop. The Quit
  item breaks the loop. Then `handle.shutdown().await` removes the icon, and
  dropping the lock file releases it (`powerprofilesctl-tray/src/main.rs:110-155`).
- **Registering again.** Spawn with `.assume_sni_available(true)` and keep the
  default `watcher_offline` (it returns `true`). The app can then start before
  Waybar, survives Waybar restarts, and `ksni` registers again on its own
  (`powerprofilesctl-tray/src/main.rs:146`, `ksni-0.3.7/src/lib.rs:251-253`,
  `ksni-0.3.7/src/service.rs:88`, `:131-160`).
- **XDG paths.** An empty or relative value counts as unset. Fall back to
  `$HOME/<default>` only if `HOME` is absolute, and error otherwise. Never use
  a relative path (`powerprofilesctl-tray/src/paths.rs:8-25`,
  `countdown_class/src/config.rs:299-305`). Use runtime → lock, state → logs,
  cache → generated icon files, config → user config
  (`powerprofilesctl-tray/src/main.rs:74-80`, `powerprofilesctl-tray/src/tray.rs:163`).
- **Logging.** Only stderr, as `<app>: message`. No log files, no log crate.
  The autostart line redirects stderr to `$XDG_STATE_HOME/<app>/log.txt`
  (`powerprofilesctl-tray/src/log.rs:3-5`, `powerprofilesctl-tray/README.md:49-53`).
- **Errors.** Never `unwrap` or `expect` outside tests (§2). Every error says
  what failed and where, for example `could not open {path}: {error}`
  (`powerprofilesctl-tray/src/main.rs:82-93`). Show runtime failures in the
  tooltip and a disabled first menu line, not only on stderr
  (`powerprofilesctl-tray/src/tray.rs:65-73`, `:102-117`; `yandex-tray-rs/src/main.rs:129-131`).
  A transient failure (a scan error, a daemon that's away) must not stop the
  tray. Show it and retry with backoff (`yandex-tray-rs/src/main.rs:157-162`,
  `powerprofilesctl-tray/src/main.rs:27-47`). Give every D-Bus call a timeout
  (5 s) (`powerprofilesctl-tray/src/profiles.rs:14`, `:202-225`).
- **Exit codes.**

  | Code | Meaning | Source |
  |---|---|---|
  | 0 | Normal exit, `--help`, already running, and `--waybar` even when the data is in error | `powerprofilesctl-tray/src/main.rs:65`, `:94-98`; `countdown_class/src/waybar.rs:45-52` |
  | 1 | Runtime error (logged), or `config check` failed | `powerprofilesctl-tray/src/main.rs:67-70`, `countdown_class/src/main.rs:25-44` |
  | 2 | Unknown argument | `powerprofilesctl-tray/src/main.rs:58-61`, `yandex-tray-rs/src/main.rs:25-35`, `countdown_class/src/main.rs:49-59` |
- **Safety of actions.** Signal only the current user's processes, matched by
  exact executable, and hold a pidfd so a reused PID is never hit
  (`yandex-tray-rs/AGENTS.md:17-20`). An Exit item that leaves the controlled
  thing unchanged says so (`powerprofilesctl-tray/src/main.rs:63`).

## 8. Waybar contract

### The `--waybar` line

`--waybar` writes JSON objects to stdout, one per line, with exactly these
fields (`countdown_class/src/waybar.rs:11-17`):

| Field | Type | Meaning |
|---|---|---|
| `text` | string | What the bar shows |
| `tooltip` | string | Hover text, or the error message |
| `class` | string | One CSS class per state, plus `error` |
| `percentage` | number | 0–100, for `{percentage}` and `format-icons` |

On an error, still exit 0 and print `{"text":"Error","tooltip":"<message>","class":"error","percentage":0.0}`,
so Waybar shows the problem instead of hiding the module
(`countdown_class/src/waybar.rs:45-52`, `countdown_class/README.md:23-25`).
The module config keeps `"return-type": "json"` and `"escape": true`
(`countdown_class/example/waybar-module.json:3-8`, `countdown_class/README.md:53-56`).

### Refresh: pick one shape per app

Never use `"interval": 1` or any fixed interval that spawns a process for an
unchanged value (`AGENTS.md:89-90`, `countdown_class/example/waybar-module.json:6`).

| Shape | Config | The process | Use when | Measured with |
|---|---|---|---|---|
| **L: long-running `exec`** | `exec`, no `interval` | Prints a line at start and one each time the line changes. Sleeps until the next possible change. Exits 0 when a write fails because Waybar closed the pipe. | The text changes on a timer (countdowns), or on events the `--waybar` process can watch itself | `footprint.sh` (it is resident) |
| **S: signal** | `exec`, `"interval": "once"`, `"signal": N` | Prints one line and exits, as `countdown_class/src/waybar.rs:55` does. Whatever changes the state runs `pkill -RTMIN+N -x waybar`. | The value changes only on user actions or rare events, so a resident process would cost more RSS than an occasional spawn | `hyperfine` |

Both shapes start no Tokio runtime and open no D-Bus connection unless the
data source needs one (`AGENTS.md:86-88`, `countdown_class/src/app.rs:32-35`).
A time-driven shape L is a plain `std` loop with `std::thread::sleep`.

Known limit, from neither app: `std` and Tokio sleeps use the monotonic clock,
which stops during suspend. A long wait until a wall-clock time can end late
after resume. Mark such a wait with
`// shortcut: <the limit>, <when to upgrade>` (`countdown_class/AGENTS.md:55`)
and mention it in the README.

### Snippet: `src/waybar.rs` (reference)

This compiles with the §2 versions and passes `rustfmt` and the §2 lints on
1.90. *(measured)* Driving a per-second display, it used 2 356 kB RSS, 60
wakeups and 2.31 ms of CPU in 60 s, one wakeup per change, against 129 for the
old 1 s tick.

```rust
use std::{io::Write, process::ExitCode, time::SystemTime};

use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Line {
    pub text: String,
    pub tooltip: String,
    pub class: &'static str,
    pub percentage: f64,
}

impl Line {
    /// Errors still exit 0, so Waybar shows them instead of hiding the module.
    pub fn error(message: impl std::fmt::Display) -> Self {
        Self {
            text: "Error".into(),
            tooltip: message.to_string(),
            class: "error",
            percentage: 0.0,
        }
    }
}

/// Writes one line. False once Waybar has closed the pipe.
/// Not `println!`: it panics on EPIPE, and `panic = "abort"` turns that into a crash.
#[must_use]
pub fn print(line: &Line) -> bool {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, line).is_ok()
        && out.write_all(b"\n").is_ok()
        && out.flush().is_ok()
}

/// Shape L for time-driven text. `state_at` is the app's pure logic: what to
/// show at `now`, and when that text can next change.
pub fn stream(state_at: impl Fn(SystemTime) -> (Line, SystemTime)) -> ExitCode {
    let mut shown = None;
    loop {
        let (line, next_change) = state_at(SystemTime::now());
        if shown.as_ref() != Some(&line) {
            if !print(&line) {
                return ExitCode::SUCCESS;
            }
            shown = Some(line);
        }
        let wait = next_change
            .duration_since(SystemTime::now())
            .unwrap_or_default();
        std::thread::sleep(wait);
    }
}
```

### Snippets: Waybar config (reference)

`templates/waybar/config.jsonc` and `templates/waybar/style.css` hold both
patterns and both refresh shapes. Cut them down to the app's pattern.

- **SNI tray item.** Only `"tray"` in a module list. The app starts from the
  compositor's autostart, not from Waybar
  (`powerprofilesctl-tray/README.md:41-53`, `yandex-tray-rs/README.md:20-22`).
- **Custom module.** `"custom/<app>"` as above. In `style.css`, give every
  `class` the app prints its own rule, `error` included. The 16 px icon is a
  CSS `background-image` (`countdown_class/example/style.css:1-30`).

## 9. Testing

Never change real system state from a test: no real power profile, no real
target app, no session or system bus writes (`powerprofilesctl-tray/CLAUDE.md:45`,
`yandex-tray-rs/AGENTS.md:73-76`, `countdown_class/AGENTS.md:76-77`, `AGENTS.md:111-113`).

**Unit tests**, in `#[cfg(test)] mod tests` next to the code:

- Pure logic at fixed times, with the clock passed in
  (`countdown_class/src/schedule.rs:181-288`, `countdown_class/src/dbus_service.rs:242-247`).
- Menu and actions. Take items from `tray.menu()` and call their `activate` or
  `select`, then assert on the `Action` received (`powerprofilesctl-tray/src/tray.rs:236-333`,
  `templates/src/tray.rs`).
- XDG fallback (`powerprofilesctl-tray/src/paths.rs:31-54`), error texts
  (`powerprofilesctl-tray/src/profiles.rs:302-359`), retry backoff
  (`powerprofilesctl-tray/src/main.rs:277-291`) and the JSON shape
  (`countdown_class/src/waybar.rs:68`).
- Fixture files under `target/`, never `/tmp` or `~`
  (`countdown_class/src/config.rs:389-396`).

**Fake D-Bus service on a private bus:**

- Start `dbus-daemon --session --print-address` on a unique abstract socket
  (`unix:abstract=<app>-tests-<pid>-<n>`) and kill it in `Drop`
  (`powerprofilesctl-tray/src/profiles.rs:361-390`, `countdown_class/src/dbus_service.rs:201-240`).
- Serve a mock `#[zbus::interface]` with the real name and path on it. It can
  simulate failures, such as a Polkit denial
  (`powerprofilesctl-tray/src/profiles.rs:429-462`).
- The code under test takes a `zbus::connection::Builder`, so a test passes
  the private address where the app passes `Builder::system()`
  (`powerprofilesctl-tray/src/profiles.rs:175`, `:398-402`).

**Fake processes:** copy `/usr/bin/sleep` into a temp dir under the name the
code looks for, point the code at that copy, and retry the spawn on `ETXTBSY`
(`yandex-tray-rs/src/browser.rs:161-196`).

**Integration tests** (`tests/cli.rs`), for apps with a CLI or D-Bus contract:

- Run the built binary with `env_clear()`, a fake `HOME` and XDG dirs under
  `CARGO_TARGET_TMPDIR` (`countdown_class/tests/cli.rs:29-59`).
- For the service, set `DBUS_SESSION_BUS_ADDRESS` to a private bus
  (`countdown_class/tests/cli.rs:175-209`).
- Assert on exit codes (`countdown_class/tests/cli.rs:161-173`), and check the
  published interface by introspection (`countdown_class/tests/cli.rs:270-341`).

**Smoke test** (manual, before the last commit; `AGENTS.md:102-115`). Run it
on a private bus with absolute XDG dirs under `target/smoke/`:

- start the app
- a second start logs "already running" and exits 0
- SIGTERM to the PID you started exits 0
- `--help` exits 0 and an unknown flag exits 2

Afterwards, check that the state and cache dirs hold only what you expect
(`powerprofilesctl-tray/CLAUDE.md:55-68`).

## 10. Assets

| File | Convention | Source |
|---|---|---|
| `assets/icon.svg` | App icon, square viewBox. The README's first line shows it at 64 px. | `countdown_class/assets/README.md:8-17`, `powerprofilesctl-tray/README.md:1` |
| `assets/<app>-<state>-symbolic.svg` | One tray icon per state: transparent background, 256-unit square viewBox, 22 px nominal, single fill `#222222` (the desktop recolours symbolic icons). Shapes differ enough to read without colour. Must work from 16 px; aim for 20–24 px. | `powerprofilesctl-tray/assets/README.md:12-23` |
| Using the state icons | Embed them with `include_str!`. Write them once at startup to `$XDG_CACHE_HOME/<app>/icons/`, return that dir from `icon_theme_path()`, and return the state's name from `icon_name()`. Nothing is installed system-wide. | `powerprofilesctl-tray/src/tray.rs:87-96`, `:162-200`; `powerprofilesctl-tray/assets/README.md:58-59` |
| Pixmap fallback (optional) | A 32×32 raw ARGB file (`assets/<app>.argb`, 4096 bytes) embedded with `include_bytes!`, built into a `ksni::Icon` once at startup. No PNG decoding at runtime. | `yandex-tray-rs/src/tray.rs:102-104`, `yandex-tray-rs/src/main.rs:88` |
| Menu item icons | Freedesktop symbolic names, such as `application-exit-symbolic` and `process-stop-symbolic` | `yandex-tray-rs/src/tray.rs:71-97` |
| Custom-module icon | 16 px SVG used as a CSS `background-image`, copied next to `style.css` | `countdown_class/example/style.css:1-8`, `countdown_class/README.md:62-63` |
| `assets/README.md` | A table of every file (purpose, size), the palette, and licences for anything not drawn from scratch (fonts, derived artwork) | `powerprofilesctl-tray/assets/README.md:9-50`, `countdown_class/assets/README.md:8-36` |

Use the `logo-design` skill for new icons if it is installed (`AGENTS.md:160`).

## 11. README template

`templates/README.md` is the skeleton. The three READMEs share this order, and
"Footprint" is required by `AGENTS.md:98`:

1. The icon line: `<img src="assets/icon.svg" width="64" height="64" alt="">`
   (`powerprofilesctl-tray/README.md:1`, `countdown_class/README.md:1-4`).
2. `# <App name>` and one paragraph: what it shows and where the data comes
   from (`powerprofilesctl-tray/README.md:3-6`, `yandex-tray-rs/README.md:1-5`).
3. Bullets: what a click and each menu item does, and what the user sees on
   failure (`powerprofilesctl-tray/README.md:8-14`, `yandex-tray-rs/README.md:7-13`).
4. `## Install and run`: `cargo install --path . --locked`, the Waybar snippet,
   the autostart line with the stderr log, and where the lock lives
   (`powerprofilesctl-tray/README.md:32-61`).
5. `## Usage`: the synopsis, every flag, the exit codes (`countdown_class/README.md:16-30`).
6. `## Configuration`, only if the app has config (`countdown_class/README.md:32`).
7. `## D-Bus service`, only if the app serves one: name, path, a table of
   members (`countdown_class/README.md:66-82`).
8. `## Footprint`: the §4 metrics, with the budget beside each, plus the
   machine and the date.
9. `## Development`: the four checks (`powerprofilesctl-tray/README.md:63-70`).
10. `## Artwork`, only if `assets/` holds more than `icon.svg`
    (`countdown_class/README.md:84`).

## 12. Definition of done

Copy this into `PROGRESS.md` and add one line per requested feature above it
(`AGENTS.md:36-45`). Tick an item only after running the command that proves
it.

```markdown
# PROGRESS

- [ ] Pattern: <SNI tray | custom module (shape L/S) | both>, because <reason from SPEC.md §1>
- [ ] Restated request (3–5 bullets) and skipped extras recorded below
- [ ] Feature: <one line per requested feature>

## Setup
- [ ] templates/ copied to the root, crate renamed, `cargo update --workspace` run, templates/ deleted
- [ ] Every new dependency asked for, with `default-features = false` and a reason in its commit

## Checks (AGENTS.md "Checks")
- [ ] cargo fmt --check
- [ ] cargo test --locked --all-targets
- [ ] cargo clippy --locked --all-targets --all-features -- -D warnings
- [ ] cargo build --release --locked
- [ ] No polling without a one-line "no event source" comment (SPEC.md §5)

## Footprint (SPEC.md §4, all ≤ budget)
- [ ] Idle RSS ≤ 4264 kB: ___
- [ ] CPU over 60 s idle = 0 ticks: ___
- [ ] Runtime over 60 s idle = 0.00 ms (time-driven: record it): ___
- [ ] Wakeups over 60 s idle = 0 (time-driven: ≤ displayed changes): ___
- [ ] Release binary ≤ 1 702 512 bytes: ___
- [ ] Each short-lived mode ≤ 741 µs hyperfine mean (or n/a): ___

## Smoke test (SPEC.md §9, private bus, XDG under target/smoke/)
- [ ] Starts and registers; menu actions do what the README says (fake services for writes)
- [ ] Second instance: "already running", exit 0
- [ ] SIGTERM: exit 0; Quit item: exit 0
- [ ] --help exit 0; unknown flag exit 2
- [ ] --waybar prints valid JSON with an `error` class on failure, exit 0 (or n/a)
- [ ] No files left outside target/smoke/; no process left that I started

## Docs and commit
- [ ] README follows SPEC.md §11, Footprint section filled with the numbers above
- [ ] waybar/ snippets cut to the chosen pattern
- [ ] assets/ icons follow SPEC.md §10, assets/README.md lists them
- [ ] Everything committed; nothing pushed
```

## 13. Out of scope

- Packaging (AUR, `.deb`, `cargo publish`, install scripts).
- CI for generated apps. `countdown_class/.github/workflows/ci.yml` is that
  app's own.
- systemd user units. Autostart belongs to the compositor config.
- Bars other than Waybar (eww, polybar, yambar, ironbar).
- GUI toolkits and other tray backends (GTK, `libappindicator`, Qt).
- Other async runtimes or D-Bus crates (`AGENTS.md:49-52`).
- Notifications, settings windows, and i18n.
