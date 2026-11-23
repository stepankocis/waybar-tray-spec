# waybar-tray-spec

A spec and starter templates for small Rust apps that live in the Waybar tray.
People clone this repo and ask their agent for an app ("a tray icon that
toggles X"). The agent builds that app here, following `SPEC.md`.

There are two kinds of task. Work out which one you have before you do anything:

- **A. Build an app.** This is the normal case. The user describes an app.
  Follow "Building an app" below.
- **B. Change the spec.** The user explicitly asks to change `SPEC.md`,
  `templates/` or these agent files. Follow "Changing the spec". If `SPEC.md`
  doesn't exist yet, the spec is still being written, so you're in B.

## Building an app

1. Restate the request in 3–5 bullets: what the app shows, what it does on
   click or in its menu, and where its data comes from. Then build exactly
   that, and list the extras you skipped in one line.
2. Choose the pattern (SNI tray item, Waybar `custom` module, or both) with the
   decision table in `SPEC.md`. Write the choice and the reason into the first
   bullet of `PROGRESS.md`.
3. Copy `templates/` to the repo root, rename the crate (`app-name` in
   `Cargo.toml`, `README.md` and `waybar/`), run `cargo update --workspace` so
   `Cargo.lock` matches the new name, and delete `templates/`.
   Keep `SPEC.md`, `AGENTS.md` and `CLAUDE.md`. Replace `README.md` with the
   app's README, following the README template in `SPEC.md`.
4. Implement the app. Read only the `SPEC.md` sections the task needs, not the
   whole file each time.
5. Run the checks and the footprint measurements (below). Tick items in
   `PROGRESS.md` only after you've verified them.
6. Commit. Don't push or change `origin` unless the user asks. The clone still
   points at the spec repo.

If the request needs something the spec doesn't cover (a new crate, a pattern
it doesn't describe), ask once, with a concrete proposal. Don't experiment.

### PROGRESS.md

Create it at the start of an app task. It holds the definition-of-done
checklist from `SPEC.md`, plus one line per requested feature. This file is
how you stay accountable:

- Tick an item only after you ran the command that proves it.
- If something was skipped or failed, write it there and in your final report.
- The final report repeats `PROGRESS.md` in its current state. Don't summarise
  it more favourably.

## Fixed stack

The stack is Rust, `ksni` (SNI tray), `zbus` (D-Bus) and `tokio`
(`current_thread`). Exact versions and features come from `SPEC.md`. Don't
evaluate alternatives (GTK, `libappindicator`, other async runtimes, other
D-Bus crates). Changing the stack is a task-B decision for the user.

- Edition 2024, MSRV 1.90. The local `rustc` may be newer, so don't use
  features newer than 1.90.
- `unsafe_code` is forbidden. Clippy `pedantic` is on, and `unwrap`/`expect`
  are allowed only in tests.
- Each new dependency needs `default-features = false`, only the features in
  use, and a one-line reason in the commit message.

## Checks

All of these must pass before every commit:

```sh
cargo fmt --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked
```

## Resource footprint

The apps run in the background all the time. Idle CPU, memory, wakeups and
startup time must be as low as possible. When this conflicts with convenience
or shorter code, footprint wins. Design for it from the first line. Don't build
a polling version and plan to optimise it later.

- Use events, not polling: D-Bus signals (`PropertiesChanged`,
  `NameOwnerChanged`), inotify, child or pidfd exit. Poll only when no event
  source exists. Then explain why in a one-line comment, and sleep until the
  displayed value can next change, not on a fixed 1 s tick.
- When idle, the app makes no wakeups, allocations or D-Bus traffic. Update the
  tray only when the visible state changed. Use icon-theme names, or build
  pixmaps once and cache them.
- Short-lived modes (`--waybar`, `config check`) start no Tokio runtime and
  open no D-Bus connection unless they need one, and do no work before parsing
  arguments.
- Don't have Waybar spawn a process every second. Use a long-running `exec`
  that prints one line per change, or refresh by signal.
- Use the release profile from `SPEC.md` unchanged.
- Measure with tools that are already installed, and never install new ones.
  The exact commands are in `SPEC.md`, "Footprint targets":
  - `VmRSS` in `/proc/PID/status` after startup settles
  - CPU over 60 s idle (difference of `utime`+`stime` in `/proc/PID/stat`)
  - wakeups and runtime over the same 60 s (context switches and `schedstat`
    per thread)
  - release binary size
  - `hyperfine` for any short-lived mode

  Record the numbers in the README's "Footprint" section. Idle wakeups and
  CPU are hard rules. RSS, binary size and startup time are recommendations
  only: going over, even by about 100 %, is fine. Write the number and a
  one-line reason in the README and the final report, and move on. Ask the
  user only if a number is more than double its target.

## Running and smoke-testing safely

The user runs Waybar and other tray apps in their live session.

- Run the app on a private bus (`dbus-run-session -- ./target/debug/<app>`),
  with **absolute** temporary XDG dirs under the git-ignored `target/`. Never
  touch the user's real config, cache or state directories for the app.
- Never kill processes you didn't start, including Waybar, other tray apps and
  the compositor's `dbus-run-session`.
- Never change real system state as a test (power profiles, network, services,
  other running apps). Read-only queries are fine. For writes, test against a
  fake D-Bus service on the private bus.
- `/tmp` may be a small tmpfs. Keep scratch files and extra target dirs under
  `target/`.

## Token and time discipline

- Don't re-read files already in context. Read only the `SPEC.md` sections you
  need.
- `templates/` and `SPEC.md` already answer the boilerplate (startup,
  single-instance lock, signals, logging, XDG paths, errors). Copy them. Don't
  re-derive them.
- The reference apps that `SPEC.md` cites are for targeted lookups of one file
  or function, never for a full read. Locally they may be in
  `~/.t3/projects/` or `~/dev/rust/`; otherwise they're on
  `github.com/stepankocis`.
- No speculative work: no packaging, CI, config options, CLI flags or
  abstractions the user didn't ask for and the spec doesn't require.
- When a check fails, fix the cause. Don't silence the lint, skip the test or
  loosen the profile.

## Subagents

You may use subagents without asking. Use them when they save time or context
in the main thread:

- a broad read-only search across the reference apps, returning a short answer
- independent parallel work, such as the icon or README while you write code,
  or a fake D-Bus service for tests
- an independent review of the finished diff against `SPEC.md` and
  `PROGRESS.md` before the last commit

Don't use them for small edits, single-file tasks, or anything already in your
context. Give each one a self-contained brief that includes the safety rules
above, and ask for a conclusion, not file dumps.

## Skills

Vendored in `.claude/skills/` (Codex sees the same files through
`.agents/skills`):

- `rust-best-practices` (Apollo, MIT): when writing or reviewing Rust. This
  file and `SPEC.md` win over its generic examples; don't add crates or
  abstractions because it shows them.

Use these if they are installed on the machine:

- `code-review` or `simplify`: final pass before the last commit.
- `logo-design`: the app icon in `assets/`.
- `graphify`: query a reference app's `graphify-out/` instead of reading its
  files.

## Changing the spec (task B)

- Every rule in `SPEC.md` points to the reference-app file it came from. Don't
  add rules without such a source, unless the user asks for them.
- Keep `templates/` minimal and in sync with `SPEC.md`. They must pass the
  checks above once copied to the root.
- After a change, update this file if the workflow changed. `CLAUDE.md` only
  imports this file.

## Committing

- Commit each self-contained step once the checks pass. Don't end a task with
  uncommitted changes of your own.
- Use a short imperative subject and add a body when the reason isn't obvious.
