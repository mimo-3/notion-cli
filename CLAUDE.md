# notion-cli

Rust CLI for the Notion API. Fills gaps left by the official `ntn` CLI: search, data source query with raw JSON filters, block/comment/view operations, multiple output formats, named profiles.

## Build / Test / Run

```bash
cargo build                    # debug build
cargo build --release          # release build
cargo run -- search "meeting"  # run a command
cargo test                     # all tests
cargo test filter              # filter-related tests only
cargo clippy -- -D warnings    # lint
cargo fmt --check              # format check
```

Binary name: `notion` (set in Cargo.toml `[[bin]]`).

MSRV: 1.75. Edition: 2021.

## Project Layout

```
src/
  main.rs          # Entry point, arg parsing, dispatch
  cli/             # clap derive structs (one file per resource)
  client/          # NotionClient, rate limiting, retry, pagination
  api/             # API method implementations (one file per resource)
  models/          # Serde types mirroring Notion API objects
  filter/          # Reserved for a future typed filter DSL
  output/          # Output formatters (json, yaml, csv, plain, id_only)
  config/          # Config file + credentials file + profile management
  error.rs         # CliError enum
tests/             # Integration tests using wiremock + assert_cmd
```

## Module Responsibilities

| Module | Owns | Does NOT own |
|---|---|---|
| `cli/` | Arg parsing, validation, help text | Business logic, HTTP calls |
| `client/` | HTTP transport, auth headers, rate limiting, retry, pagination | Notion domain knowledge |
| `api/` | Mapping commands to API endpoints, request/response shaping | Arg parsing, output formatting |
| `models/` | Serde structs for all Notion objects | Any logic beyond derive |
| `filter/` | Future typed DSL parsing and lowering | HTTP, output |
| `output/` | Serializing models to each output format | Fetching data |
| `config/` | Config file I/O, profile CRUD, credentials file access | HTTP, CLI args |
| `error.rs` | Error types, exit codes, Display impls | Recovery logic |

## Coding Standards

- All public types and functions have doc comments.
- Error handling: use `CliError` (thiserror) for typed errors. Functions return `Result<T, CliError>`. Use `anyhow` only in `main.rs` for top-level error reporting.
- No `unwrap()` or `expect()` outside of tests.
- All HTTP calls go through `NotionClient`; never use `reqwest` directly in `api/` or `cli/`.
- Models: derive `Serialize, Deserialize, Debug, Clone`. Use `#[serde(rename_all = "snake_case")]`. Optional fields use `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`.
- Output goes to stdout; status/progress/errors go to stderr.
- Respect `--no-color` and detect non-TTY (disable color automatically when piped).
- Tests: unit tests in-module (`#[cfg(test)]`), integration tests in `tests/`. Use `wiremock` for HTTP mocking. No real API calls in CI.
- Commits: one file per commit, short Japanese message, no AI attribution.

## Key Design Decisions

- **Token bucket rate limiter** (3 req/s) with server-side Retry-After support.
- **Cursor-based auto-pagination** via async Stream; `--all` fetches everything, `--limit N` caps total items.
- **Filter input** currently uses `--filter-json`; the DSL remains planned until property types can be resolved safely.
- **Token resolution**: `--token` flag > `NOTION_API_TOKEN` env > credentials file (`credentials.json`, 0600) > legacy config file.
- **Output format**: `--plain` default for TTY, `--json` for piping. All commands support all formats.

## API Version

All requests send `Notion-Version: 2026-03-11`. Override with `--api-version`.

**Known version-specific behavior**: The `after` parameter for block children append (`PATCH /v1/blocks/{id}/children`) requires `--api-version 2022-06-28`. The default version (`2026-03-11`) rejects it with `body.after should be not present` (400 validation_error). This is Notion's official version pinning — older versions remain supported, but the parameter was removed from newer API contracts.

`file upload --parent <id>` attaches the uploaded file to a page/block as a child block (block type derived from content type: image/video/audio/pdf/file). With `--after <block-id>` the attach request is automatically pinned to `2022-06-28` unless the user passes `--api-version` explicitly.

## Phase 1 Scope

Auth, search, page (get/content/create), db (get/query with raw JSON filters), user me, config, all output formats, rate limiting, pagination. See ARCHITECTURE.md for full phasing.
