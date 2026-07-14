# notion-cli Architecture

## 1. Tech Stack

| Dependency | Version | Purpose |
|---|---|---|
| Rust | edition 2021, MSRV 1.75 | Language |
| clap | 4.x (derive) | CLI parsing, help generation, shell completions |
| reqwest | 0.12+ (rustls) | Async HTTP client |
| tokio | 1.x (rt-multi-thread, macros) | Async runtime |
| serde / serde_json | 1.x | JSON serialization |
| serde_yaml | 0.9+ | YAML output |
| csv | 1.x | CSV/TSV output |
| keyring | 3.x | OS keychain token storage |
| dirs | 5.x | XDG-compliant config paths |
| colored | 2.x | Terminal color output |
| thiserror | 2.x | Error type derivation |
| anyhow | 1.x | Application-level error handling |
| chrono | 0.4 | Date parsing for filter DSL |
| pest | 2.x | PEG parser for filter DSL |
| wiremock | 0.6+ | HTTP mock server for integration tests |
| assert_cmd / predicates | latest | CLI integration testing |
| tracing / tracing-subscriber | 0.1 / 0.3 | Structured logging (--verbose) |

## 2. Project Structure

```
notion-cli/
├── Cargo.toml
├── Cargo.lock
├── SPEC.md
├── ARCHITECTURE.md
├── CLAUDE.md
├── LICENSE (MIT)
├── README.md
├── .github/
│   └── workflows/
│       ├── ci.yml            # lint + test on PR
│       └── release.yml       # cross-compile + GitHub Release
├── src/
│   ├── main.rs               # Entry point: parse args, init runtime, dispatch
│   ├── cli/
│   │   ├── mod.rs            # Top-level Cli struct, GlobalOpts
│   │   ├── auth.rs           # AuthCommand enum
│   │   ├── search.rs         # SearchCommand
│   │   ├── page.rs           # PageCommand enum
│   │   ├── db.rs             # DbCommand enum
│   │   ├── block.rs          # BlockCommand enum
│   │   ├── comment.rs        # CommentCommand enum
│   │   ├── user.rs           # UserCommand enum
│   │   ├── file.rs           # FileCommand enum
│   │   ├── view.rs           # ViewCommand enum
│   │   ├── datasource.rs     # DatasourceCommand enum
│   │   ├── api.rs            # Raw API command
│   │   └── config_cmd.rs     # Config command
│   ├── client/
│   │   ├── mod.rs            # NotionClient struct, builder
│   │   ├── auth.rs           # Auth header injection
│   │   ├── rate_limit.rs     # Rate limiter (token bucket)
│   │   ├── retry.rs          # Retry logic with backoff
│   │   └── pagination.rs     # Auto-pagination iterator
│   ├── api/
│   │   ├── mod.rs            # Re-exports
│   │   ├── pages.rs          # Page API methods
│   │   ├── databases.rs      # Database API methods
│   │   ├── blocks.rs         # Block API methods
│   │   ├── search.rs         # Search API method
│   │   ├── comments.rs       # Comments API methods
│   │   ├── users.rs          # Users API methods
│   │   ├── files.rs          # Files API methods
│   │   ├── views.rs          # Views API methods
│   │   ├── datasources.rs    # Data sources API methods
│   │   └── raw.rs            # Raw API passthrough
│   ├── models/
│   │   ├── mod.rs            # Re-exports
│   │   ├── page.rs           # Page, PageProperties
│   │   ├── database.rs       # Database, DatabaseSchema
│   │   ├── block.rs          # Block enum with all block types
│   │   ├── user.rs           # User, Bot, Person
│   │   ├── comment.rs        # Comment
│   │   ├── rich_text.rs      # RichText, Annotations, Mention
│   │   ├── property.rs       # PropertyValue, PropertyConfig (all types)
│   │   ├── filter.rs         # Filter, Sort (API JSON structures)
│   │   ├── file.rs           # FileObject
│   │   ├── view.rs           # View
│   │   ├── common.rs         # Parent, Icon, Cover, Color, Emoji, IdOrUrl
│   │   └── response.rs       # PaginatedResponse<T>, ErrorResponse
│   ├── filter/
│   │   ├── mod.rs            # Public parse_filter() function
│   │   ├── parser.rs         # Pest grammar consumer
│   │   ├── ast.rs            # FilterExpr, CompoundFilter, PropertyFilter
│   │   └── filter.pest       # PEG grammar file
│   ├── output/
│   │   ├── mod.rs            # OutputFormat enum, format_output() dispatch
│   │   ├── json.rs           # JSON (pretty / compact)
│   │   ├── yaml.rs           # YAML
│   │   ├── csv.rs            # CSV / TSV
│   │   ├── plain.rs          # Human-readable tables and summaries
│   │   └── id_only.rs        # ID-only output for piping
│   ├── config/
│   │   ├── mod.rs            # Config struct, load/save
│   │   ├── profile.rs        # Profile management
│   │   └── auth_store.rs     # Keyring + file fallback
│   └── error.rs              # CliError enum, conversion impls
└── tests/
    ├── common/
    │   └── mod.rs             # Shared test helpers, mock server setup
    ├── test_auth.rs
    ├── test_search.rs
    ├── test_page.rs
    ├── test_db_query.rs
    └── test_filter_parser.rs
```

## 3. Core Abstractions

### 3.1 NotionClient

```rust
pub struct NotionClient {
    http: reqwest::Client,
    base_url: Url,              // https://api.notion.com (overridable for tests)
    token: String,
    api_version: String,        // "2026-03-11"
    rate_limiter: RateLimiter,
}
```

Responsibilities:
- Inject `Authorization` and `Notion-Version` headers on every request
- Enforce client-side rate limiting (token bucket, ~3 req/s default)
- Retry on 429/529 with `Retry-After` header, exponential backoff with jitter (max 3 retries)
- Provide `get`, `post`, `patch`, `delete` methods that return `Result<serde_json::Value, CliError>`
- `paginate<T>()` method returning `Stream<Item = Result<T>>` for auto-pagination

### 3.2 Config & Profile

```rust
pub struct Config {
    pub default_profile: String,
    pub profiles: HashMap<String, Profile>,
    pub defaults: Defaults,
}

pub struct Profile {
    pub token: Option<String>,    // Stored in config file (fallback)
    pub workspace_id: Option<String>,
}

pub struct Defaults {
    pub output_format: OutputFormat,
    pub page_size: u8,            // 1-100, default 50
}
```

Token resolution order:
1. `--token` flag
2. `NOTION_API_TOKEN` env var
3. OS keyring (service: `notion-cli`, account: profile name)
4. Config file `profiles.<name>.token`

Config path: `$XDG_CONFIG_HOME/notion-cli/config.json` (typically `~/.config/notion-cli/config.json`)

### 3.3 Output Formatting

```rust
pub enum OutputFormat { Json, Yaml, Csv, Tsv, Plain, IdOnly }

pub fn format_output<T: Serialize + Displayable>(
    data: &T,
    format: OutputFormat,
    writer: &mut dyn Write,
) -> Result<()>;
```

- `Plain` output uses the `Displayable` trait (implemented per model) for human-friendly rendering
- `Csv`/`Tsv` flatten nested properties into columns; only meaningful for list/query results
- `IdOnly` extracts just the `id` field, one per line

### 3.4 Filter DSL

PEG grammar (simplified):

```
filter     = { compound }
compound   = { clause ~ (("AND" | "OR") ~ clause)* }
clause     = { "(" ~ compound ~ ")" | predicate }
predicate  = { property ~ operator ~ value? }
property   = { quoted_string | identifier }
operator   = { "=" | "!=" | ">" | ">=" | "<" | "<=" | "contains" | "does_not_contain" | "starts_with" | "ends_with" | "is_empty" | "is_not_empty" }
value      = { quoted_string | number | date_literal | "true" | "false" | "today" }
```

The parser produces an AST (`FilterExpr`) which is lowered to the Notion API filter JSON structure. `today` is expanded to the current date at parse time.

Limitation: Mixed AND/OR at the same level requires explicit parentheses (mirrors Notion's restriction that compound filters use either `and` or `or`, not both).

### 3.5 Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("Not authenticated. Run `notion auth login` first.")]
    NotAuthenticated,

    #[error("API error ({status}): {code} - {message}")]
    Api { status: u16, code: String, message: String },

    #[error("Rate limited. Retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Invalid filter: {0}")]
    FilterParse(String),

    #[error("Invalid ID format: {0}")]
    InvalidId(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
```

Exit codes: 0 = success, 1 = general error, 2 = usage error (clap handles this), 3 = auth error, 4 = rate limited (all retries exhausted).

## 4. Command Organization

```rust
// src/cli/mod.rs
#[derive(Parser)]
#[command(name = "notion", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub global: GlobalOpts,
}

#[derive(Args)]
pub struct GlobalOpts {
    #[arg(long, env = "NOTION_API_TOKEN")]
    pub token: Option<String>,
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub workspace: Option<String>,
    #[arg(long)]
    pub api_version: Option<String>,
    #[arg(long, default_value = "plain")]
    pub format: Option<OutputFormat>,
    #[arg(long)]
    pub json: bool,           // Shorthand for --format json
    #[arg(long)]
    pub verbose: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub no_color: bool,
}

#[derive(Subcommand)]
pub enum Command {
    Auth(AuthCommand),
    Search(SearchArgs),
    Page(PageCommand),
    Db(DbCommand),
    Block(BlockCommand),
    Comment(CommentCommand),
    User(UserCommand),
    File(FileCommand),
    View(ViewCommand),
    Datasource(DatasourceCommand),
    Api(ApiArgs),
    Config(ConfigCommand),
}
```

Each subcommand module defines its own clap structs. Dispatch happens in `main.rs`:

```rust
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;
    let client = NotionClient::from_opts(&cli.global, &config)?;

    match cli.command {
        Command::Auth(cmd) => cmd.run(&config).await,
        Command::Search(args) => args.run(&client, &cli.global).await,
        // ...
    }
}
```

## 5. Authentication Flow

### `notion auth login`

1. Prompt user for token (or accept `--token` flag)
2. Validate token by calling `GET /v1/users/me`
3. Store token in OS keyring via `keyring` crate (service: `notion-cli`, account: `<profile>`)
4. If keyring unavailable (headless server, WSL), fall back to config file with `0600` permissions
5. Save workspace metadata to config

### `notion auth logout`

1. Delete token from keyring
2. Remove profile from config file

### `notion auth switch <profile>`

1. Set `default_profile` in config
2. Verify token still valid

### Environment variable override

`NOTION_API_TOKEN` always takes precedence. When set, no keyring/config lookup occurs.

## 6. Rate Limiting Strategy

**Client-side: Token bucket**

- Capacity: 3 tokens, refill rate: 3 tokens/second
- Each request consumes 1 token
- If bucket empty, sleep until a token is available before sending

**Server-side: Retry-After**

- On 429 or 529 response, read `Retry-After` header (seconds)
- Wait for the specified duration, then retry
- Max 3 retries with exponential backoff: `retry_after * 2^attempt` (capped at 60s)
- Add jitter: `+/- 20%` randomization
- After max retries, return `CliError::RateLimited`

**Implementation**: `RateLimiter` wraps a `tokio::sync::Semaphore`-based token bucket. The retry logic lives in `client/retry.rs` as a middleware around each request.

## 7. Pagination Strategy

```rust
pub struct PaginatedRequest {
    pub page_size: u8,       // Default 50, max 100
    pub start_cursor: Option<String>,
    pub fetch_all: bool,     // --all flag
    pub limit: Option<u32>,  // --limit N (total items, not pages)
}
```

- **Default**: Return first page of results (up to `page_size` items)
- **`--all`**: Auto-paginate, streaming results to output as they arrive
- **`--limit N`**: Fetch up to N total items across pages, stop early
- **`--cursor <CURSOR>`**: Start from a specific cursor (for manual pagination)

For `--all`, results stream to stdout incrementally (one JSON object per line in JSON mode, rows appended in CSV mode, items printed in plain mode). This avoids buffering large result sets in memory.

The `paginate()` method on `NotionClient` returns an `async Stream` that handles cursor chaining internally.

## 8. Testing Strategy

### Unit Tests (`#[cfg(test)]` in each module)

- **filter/parser.rs**: Parse various DSL expressions, verify AST
- **filter/mod.rs**: DSL-to-JSON lowering
- **output/**: Format model objects in each format, snapshot test output
- **config/**: Load/save config files, profile resolution
- **models/**: Serde round-trip tests (deserialize API JSON, re-serialize)
- **client/rate_limit.rs**: Token bucket timing behavior

### Integration Tests (`tests/`)

- Use `wiremock` to spin up a mock Notion API server
- Test full command execution via `assert_cmd`:
  - Auth flow (login stores token, whoami returns user)
  - Search returns results in each output format
  - Db query with filters, pagination, output formats
  - Error handling (401, 429 with retry, 404)
- Filter DSL end-to-end: DSL string -> API request body

### Test Conventions

- No real Notion API calls in CI
- Fixtures in `tests/fixtures/` for sample API responses
- `#[tokio::test]` for all async tests
- Snapshot testing via `insta` crate for output format stability

## 9. CI/CD

### GitHub Actions: `ci.yml`

Triggers: push to `main`, all PRs

```yaml
jobs:
  check:
    - cargo fmt --check
    - cargo clippy -- -D warnings
    - cargo test
  msrv:
    - Install MSRV toolchain
    - cargo check
```

### GitHub Actions: `release.yml`

Triggers: tag `v*`

Build matrix:
| Target | OS | Binary |
|---|---|---|
| x86_64-unknown-linux-gnu | ubuntu-latest | notion-linux-amd64 |
| aarch64-unknown-linux-gnu | ubuntu-latest (cross) | notion-linux-arm64 |
| x86_64-apple-darwin | macos-latest | notion-darwin-amd64 |
| aarch64-apple-darwin | macos-latest | notion-darwin-arm64 |
| x86_64-pc-windows-msvc | windows-latest | notion-windows-amd64.exe |

Steps:
1. Cross-compile with `cross` or native toolchain
2. Strip binaries
3. Create GitHub Release with all binaries
4. Generate SHA256 checksums
5. Update Homebrew tap formula (separate repo)
6. Publish to crates.io

## 10. Phase 1 Scope

Phase 1 delivers the highest-value commands that cover the most common CLI workflows:

### Phase 1 deliverables

| Command | Priority | Rationale |
|---|---|---|
| `auth login/logout/whoami` | P0 | Required for everything else |
| `search` | P0 | Highest-value gap vs ntn; enables discovery |
| `page get/content/create` | P0 | Core CRUD, most common operation |
| `db get/query` | P0 | Database queries with filter DSL are the killer feature |
| `user me` | P0 | Needed for auth validation |
| `config get/set/list` | P0 | Profile management |
| Global flags & output formats | P0 | `--json`, `--plain`, `--csv` on all commands |
| Rate limiting & retry | P0 | Production readiness |
| Auto-pagination | P0 | Essential for db query and search |

### Phase 1 non-goals

- Block operations (Phase 2)
- Comments (Phase 2)
- Views (Phase 2)
- File upload (Phase 2)
- Data sources (Phase 3)
- `api` raw passthrough (Phase 2)
- OAuth browser flow (Phase 3; Phase 1 uses token-based auth only)
- Interactive/TUI mode (Phase 3+)

### Phase 1 milestones

1. **Skeleton**: Cargo project, clap command tree, config loading, NotionClient with auth
2. **Core client**: Rate limiting, retry, pagination, error handling
3. **Auth + User**: `auth login/logout/whoami`, `user me`, keyring storage
4. **Search**: `search` command with all output formats
5. **Pages**: `page get/content/create` with markdown I/O
6. **Database query**: `db get/query` with filter DSL, sort, output formats
7. **Polish**: Shell completions, `--dry-run`, README, CI/CD pipeline
