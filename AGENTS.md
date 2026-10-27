# fractic-aws-secrets

Fetching and parsing AWS Secrets Manager secrets (plain and structured JSON
values, sub-keys) for `fractic-context` and the CLIs. `src/lib.rs` →
`errors.rs`, `util.rs`; re-exports `serde_json` for consumers that parse
structured secrets. No context view: this crate is what `define_ctx!` uses
to load secrets.

- Commit / PR titles: `[util] …`, `[ext] …`.
