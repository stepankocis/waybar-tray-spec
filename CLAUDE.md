@AGENTS.md

## Claude Code only

- `AGENTS.md` is the single source of rules. Change rules there, never here.
- The project enables two plugins in `.claude/settings.json`. Accept them when
  Claude Code offers to install them.
  - `context7`: look up current `ksni`, `zbus` and `tokio` APIs instead of
    guessing from memory. Check the result against the versions in
    `Cargo.lock`.
  - `rust-analyzer-lsp`: definitions, references and type info. Use it instead
    of grepping through `target/` or `~/.cargo/registry`.
