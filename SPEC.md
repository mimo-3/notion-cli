# notion-cli Specification

> Foundation document for building a comprehensive Notion CLI tool.
> Based on ntn CLI v0.18.1 (Beta) analysis and Notion API (version 2026-03-11) reference.

---

## 1. ntn CLI Capabilities & Limitations

### What ntn Does

| Area | Commands | Notes |
|------|----------|-------|
| **Auth** | `login`, `logout`, `whoami` | OAuth browser flow; keychain or file-based storage |
| **Pages** | `pages get/create/edit/trash` | Markdown-centric CRUD with frontmatter properties |
| **Data Sources** | `datasources query/resolve` | Query via data source ID (not database ID directly) |
| **Files** | `files create/get/list` | Upload from stdin or URL; list has no pagination |
| **Generic API** | `api ls`, `api <PATH>` | Call any public endpoint; inline body/query syntax |
| **Workers** | 20+ subcommands | Full lifecycle: init, deploy, delete, env, runs, sync, webhooks, OAuth, TUI |
| **Utility** | `completions`, `doctor`, `update` | Shell completions, health check, self-update |

### What ntn Does NOT Do

| Gap | Workaround |
|-----|------------|
| No `search` command | `ntn api v1/search -d '{"query":"..."}'` |
| No database CRUD | `ntn api v1/databases ...` |
| No block-level operations | `ntn api v1/blocks/...` |
| No comments | `ntn api v1/comments ...` |
| No user listing/lookup | `ntn api v1/users ...` |
| No page property editing via `pages` | `ntn api v1/pages/{id} -X PATCH ...` |
| No workspace management | Env var only (`NOTION_WORKSPACE_ID`) |
| No permissions/sharing management | Not in API either |
| No import/export beyond single pages | -- |
| `files list` has no pagination | First page only |
| `pages edit` replaces entire content | No partial/append |
| `pages` requires `NOTION_API_TOKEN` | Does not use keychain login |
| Truncated markdown on `pages get` | Use `--json` to see `unknown_block_ids` |

---

## 2. Complete API Endpoint Inventory

### Authentication & OAuth (4 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/oauth/token` | Exchange authorization code for tokens |
| POST | `/v1/oauth/token/refresh` | Refresh an OAuth token pair |
| POST | `/v1/oauth/token/revoke` | Revoke an OAuth token |
| GET | `/v1/oauth/token/introspect` | Introspect a token |

### Pages (7 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/pages` | Create a page |
| GET | `/v1/pages/{page_id}` | Retrieve a page (properties only) |
| PATCH | `/v1/pages/{page_id}` | Update page properties, icon, cover, archive/trash/lock |
| DELETE | `/v1/pages/{page_id}` | Trash a page |
| POST | `/v1/pages/{page_id}/move` | Move a page to new parent |
| GET | `/v1/pages/{page_id}/properties/{property_id}` | Retrieve a page property item (paginated) |
| GET | `/v1/pages/{page_id}/content` | Retrieve page content as markdown |
| PUT | `/v1/pages/{page_id}/content` | Update page content as markdown |

### Databases (3 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/databases` | Create a database |
| GET | `/v1/databases/{database_id}` | Retrieve a database schema |
| PATCH | `/v1/databases/{database_id}` | Update database title, description, schema, icon, cover |

### Blocks (5 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/blocks/{block_id}` | Retrieve a single block |
| PATCH | `/v1/blocks/{block_id}` | Update a block's content |
| DELETE | `/v1/blocks/{block_id}` | Soft-delete a block |
| GET | `/v1/blocks/{block_id}/children` | List child blocks (depth=1, paginated) |
| PATCH | `/v1/blocks/{block_id}/children` | Append child blocks (max 100, max 2 nesting levels) |

### Users (3 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/users/me` | Retrieve authenticated bot/user |
| GET | `/v1/users/{user_id}` | Retrieve a user by ID |
| GET | `/v1/users` | List workspace users (excludes guests) |

### Comments (5 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/comments` | Create a comment (page-level, block-level, or reply) |
| GET | `/v1/comments` | List comments on a block |
| GET | `/v1/comments/{comment_id}` | Retrieve a comment |
| PATCH | `/v1/comments/{comment_id}` | Update a comment |
| DELETE | `/v1/comments/{comment_id}` | Delete a comment |

### Search (1 endpoint)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/search` | Search by page title; filter by object type; sort by relevance or last_edited_time |

### Data Sources (5 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/data_sources` | Create a data source |
| GET | `/v1/data_sources/{data_source_id}` | Retrieve a data source |
| PATCH | `/v1/data_sources/{data_source_id}` | Update a data source |
| POST | `/v1/data_sources/{data_source_id}/query` | Query a data source |
| GET | `/v1/data_source_templates` | List data source templates |

### Views (7 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/databases/{database_id}/views` | Create a view |
| GET | `/v1/databases/{database_id}/views` | List views |
| GET | `/v1/views/{view_id}` | Retrieve a view |
| PATCH | `/v1/views/{view_id}` | Update a view |
| DELETE | `/v1/views/{view_id}` | Delete a view |
| POST | `/v1/views/{view_id}/query` | Create a view query |
| GET | `/v1/views/{view_id}/query_results` | Get view query results |
| DELETE | `/v1/views/{view_id}/query` | Delete a view query |

### File Uploads (5 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/file_uploads` | Create a file upload |
| POST | `/v1/file_uploads/{file_upload_id}/send` | Send file upload data |
| POST | `/v1/file_uploads/{file_upload_id}/complete` | Complete a multi-part upload |
| GET | `/v1/file_uploads/{file_upload_id}` | Retrieve a file upload |
| GET | `/v1/file_uploads` | List file uploads |

### Meeting Notes (1 endpoint)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/meeting_notes` | Query meeting notes |

### Async Tasks (1 endpoint)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/async_tasks/{task_id}` | Retrieve async task status |

### Admin / Enterprise (13 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/legal_holds` | Create a legal hold |
| GET | `/v1/legal_holds` | List legal holds |
| GET | `/v1/legal_holds/{legal_hold_id}` | Retrieve a legal hold |
| PATCH | `/v1/legal_holds/{legal_hold_id}` | Update a legal hold |
| DELETE | `/v1/legal_holds/{legal_hold_id}` | Release a legal hold |
| POST | `/v1/legal_holds/{legal_hold_id}/users` | Add users to a legal hold |
| GET | `/v1/legal_holds/{legal_hold_id}/users` | List users on a legal hold |
| DELETE | `/v1/legal_holds/{legal_hold_id}/users/{user_id}` | Remove user from legal hold |
| GET | `/v1/legal_holds/{legal_hold_id}/pages` | List pages on a legal hold |
| GET | `/v1/legal_holds/{legal_hold_id}/workspaces` | List workspaces on a legal hold |
| POST | `/v1/legal_holds/{legal_hold_id}/export` | Export a legal hold |
| POST | `/v1/workspaces/{workspace_id}/export` | Enqueue workspace export |
| POST | `/v1/users/{user_id}/revoke_sessions` | Revoke managed user sessions |

**Total: 58+ endpoints across 12 resource categories.**

---

## 3. Authentication Model

### Token Types

| Type | How Obtained | Scope | Refresh | Use Case |
|------|-------------|-------|---------|----------|
| **Internal Connection Token** | Created in integration settings | Single workspace, pages explicitly shared | No expiry/refresh | Team automations |
| **Personal Access Token (PAT)** | Developer portal | User's workspace permissions | No expiry/refresh | CLI tools, scripts |
| **OAuth access_token** | `POST /v1/oauth/token` with `authorization_code` | Pages selected during consent | Via refresh_token | Public apps |
| **OAuth refresh_token** | Returned alongside access_token | -- | Rotates on each use | Token renewal |

### Auth Headers (all requests)

```
Authorization: Bearer <token>
Notion-Version: 2026-03-11
```

### OAuth Flow

1. Redirect user to Notion auth URL with `client_id`, `redirect_uri`, `response_type=code`
2. User selects pages to share
3. Exchange `code` via `POST /v1/oauth/token` (Basic auth with `client_id:client_secret`)
4. Receive `access_token`, `refresh_token`, `bot_id`, `workspace_id`
5. Refresh via `POST /v1/oauth/token` with `grant_type=refresh_token`

### ntn CLI Auth

- `ntn login` opens browser OAuth flow
- Stores credentials in OS keychain (or `~/.config/notion/auth.json` if `NOTION_KEYRING=0`)
- `NOTION_API_TOKEN` env var overrides keychain for all commands
- **`pages` commands require `NOTION_API_TOKEN`** -- they don't use keychain login

### For notion-cli

Our CLI should support:
1. `NOTION_API_TOKEN` env var (simplest, covers internal + PAT tokens)
2. Config file (`~/.config/notion-cli/config.json`) for persistent token storage
3. Named profiles for multiple workspaces

---

## 4. Data Model

### Pages

- **Properties**: key-value pairs defined by parent database schema (or just `title` for page-parent pages)
- **Content**: tree of blocks (separate from properties)
- **Metadata**: `id`, `created_time`, `last_edited_time`, `created_by`, `last_edited_by`, `parent`, `url`, `icon`, `cover`, `in_trash`, `is_archived`, `is_locked`
- **Property types (writable)**: title, rich_text, number, select, multi_select, status, date, checkbox, url, email, phone_number, people, relation, files, verification, place
- **Property types (read-only)**: formula, rollup, created_time, created_by, last_edited_time, last_edited_by, unique_id, button

### Databases

- **Schema**: `properties` map defining column names and types
- **Parent**: page_id or workspace
- **Fields**: `id`, `title`, `description`, `icon`, `cover`, `is_inline`, `is_locked`, `in_trash`, `url`, `parent`, `properties`
- **Supported property types**: title, rich_text, number, select, multi_select, status, date, people, files, checkbox, url, email, phone_number, formula, relation, rollup, unique_id, created_by, created_time, last_edited_by, last_edited_time, button, location, verification

### Blocks

- **Structure**: tree; each block has `has_children` flag; children fetched separately (depth=1 per request)
- **Common fields**: `id`, `type`, `created_time`, `last_edited_time`, `created_by`, `last_edited_by`, `has_children`, `in_trash`, `parent`
- **Block types**: paragraph, heading_1-4, bulleted_list_item, numbered_list_item, to_do, toggle, template, quote, callout, code, equation, image, video, pdf, file, audio, embed, bookmark, link_preview, column_list, column, table, table_row, tab, child_page, child_database, synced_block, divider, breadcrumb, table_of_contents, link_to_page, meeting_notes, unsupported

### Rich Text

- Array of objects, each with: `type` (text, mention, equation), `annotations` (bold, italic, strikethrough, underline, code, color), `plain_text`, `href`
- Max 100 items per array, 2000 chars per text element

### Users

- **Fields**: `id`, `object` ("user"), `name`, `avatar_url`, `type` ("person" | "bot")
- **Person-specific**: `person.email`, `person.email_verified`
- **Bot-specific**: `bot.owner`, `bot.workspace_id`, `bot.workspace_name`

### Comments

- **Fields**: `id`, `object` ("comment"), `parent` (page_id or block_id), `discussion_id`, `created_time`, `last_edited_time`, `created_by`, `rich_text`, `display_name`, `attachments`
- **Targets**: page-level comment, block-level comment, or reply to existing discussion

---

## 5. API Constraints

### Rate Limits
- **Per-integration**: average 3 requests/second (some bursts allowed)
- **Per-workspace**: shared across all integrations, scaled by plan
- **429 response**: respect `Retry-After` header (integer seconds)
- **529 response**: treat identically to 429
- **Strategy**: exponential backoff with jitter

### Pagination
- **Type**: cursor-based
- **Parameters**: `page_size` (default 100, max 100), `start_cursor`
- **Response**: `has_more` (bool), `next_cursor` (string|null)
- **Location**: query string for GET; request body for POST

### Payload Limits
- Max 1000 block elements per payload
- Max 500 KB total payload
- Max 100 blocks per append-children request
- Max 2 nesting levels per append-children request
- Rich text / URLs: 2000 chars per element
- Rich text arrays: max 100 items
- Email/Phone: 200 chars each
- Multi-select options, relations, people: 100 per request
- Comment attachments: max 3 per create, 100 in response
- `filter_properties`: max 100 property IDs

### Versioning
- **Header**: `Notion-Version` (required on every request)
- **Current**: `2026-03-11`
- **Omitting returns**: `400 missing_version`

### ID Format
- UUIDv4 (dashes optional in requests)

### Other
- Empty strings not supported; use `null` instead
- `snake_case` property naming throughout
- Dates: `YYYY-MM-DD` or ISO 8601 datetime
- Soft-delete model: `in_trash`/`is_archived` flags; resources remain retrievable

---

## 6. Gap Analysis

### API features ntn CLI does NOT expose (opportunities for notion-cli)

| API Capability | ntn Status | Opportunity |
|----------------|-----------|-------------|
| **Search** (`POST /v1/search`) | No command | High-value `search` command |
| **Database CRUD** | No command | `db create/get/update/delete` |
| **Block operations** | No command | `block get/update/delete/children/append` |
| **Comments** | No command | `comment create/list/get/update/delete` |
| **User listing** | Only `whoami` | `user list/get/me` |
| **Page property editing** | Not via `pages` | `page update` with property flags |
| **Page move** | No command | `page move` |
| **Views** | No command | `view create/list/get/update/delete/query` |
| **Data source query** | Available via datasources | `db query` with raw JSON filters |
| **Partial page editing** | Replaces entire content | Block-level append/update |
| **Bulk operations** | One-at-a-time | Batch commands with rate limiting |
| **Output formats** | Markdown + JSON | Add YAML, CSV, TSV for database queries |
| **Filtering syntax** | Raw JSON only | Raw JSON with pagination and output controls |
| **Interactive mode** | Only workers TUI | General-purpose interactive browse/edit |
| **Pipe-friendly** | Partial | Consistent stdin/stdout/stderr conventions |

### ntn strengths we should NOT replicate

| Feature | Reason |
|---------|--------|
| Workers lifecycle | Notion-specific serverless platform; out of scope |
| Workers TUI | Workers-specific |
| Self-update | Package manager handles this |
| Doctor | Specific to ntn internals |

---

## 7. Recommended CLI Command Structure

### Design Principles

1. **Resource-action pattern**: `notion <resource> <action> [args] [flags]`
2. **Consistent output**: `--json`, `--plain`, `--yaml`, `--csv` on all commands
3. **Pipe-friendly**: stdin for content, stdout for data, stderr for status
4. **Pagination built-in**: `--limit`, `--all` (auto-paginate), `--cursor`
5. **Explicit filters**: `--filter-json` accepts the exact Notion filter contract
6. **Rate-limit aware**: built-in retry with exponential backoff

### Command Tree

```
notion
├── auth
│   ├── login              # Store token (prompt or --token flag)
│   ├── logout             # Remove stored token
│   ├── whoami             # Show current user/workspace info
│   └── switch <profile>   # Switch named profile
│
├── search <query>         # POST /v1/search
│   ├── --filter page|data_source
│   ├── --sort relevance|last_edited
│   └── --limit N / --all
│
├── page
│   ├── get <page_id>      # Retrieve page properties
│   ├── content <page_id>  # Get page content as markdown
│   ├── create             # Create page (--parent, --parent-type page|data-source)
│   ├── update <page_id>   # Update properties, icon, cover, lock, archive
│   ├── edit <page_id>     # Replace content (markdown via stdin or --content)
│   ├── move <page_id>     # Move to page/data-source parent (--parent, --parent-type)
│   ├── trash <page_id>    # Soft-delete
│   ├── restore <page_id>  # Restore from trash
│   └── prop <page_id> <property_id>  # Get single property (paginated)
│
├── db
│   ├── get <db_id>        # Retrieve database schema
│   ├── create             # Create database (--parent, --title, --props-json)
│   ├── update <db_id>     # Update schema, title, description
│   ├── query <ds_id>      # Query a data source with filters and sorts
│   │   ├── --filter-json <JSON>
│   │   ├── --sort <prop> [asc|desc]
│   │   ├── --props <prop1,prop2>  # filter_properties
│   │   └── --limit N / --all
│   ├── list               # List data sources via search
│   └── trash <db_id>      # Archive database
│
├── block
│   ├── get <block_id>     # Retrieve single block
│   ├── children <block_id>  # List children (--recursive for full tree)
│   ├── append <block_id>  # Append children (--content markdown, or --blocks-json)
│   ├── update <block_id>  # Update block content
│   ├── delete <block_id>  # Soft-delete
│   └── move <block_id>    # Move block (--after, --parent)
│
├── comment
│   ├── list               # List comments (--block-id or --page-id)
│   ├── get <comment_id>   # Retrieve comment
│   ├── create             # Create comment (--page, --block, or --discussion for reply)
│   ├── update <comment_id>  # Update comment text
│   └── delete <comment_id>  # Delete comment
│
├── user
│   ├── me                 # Current bot/user
│   ├── get <user_id>      # Retrieve user
│   └── list               # List workspace users
│
├── file
│   ├── upload <path>      # Upload file (or stdin)
│   ├── get <file_id>      # Retrieve file metadata
│   └── list               # List uploads
│
├── view
│   ├── list <db_id>       # List views for database
│   ├── get <view_id>      # Retrieve view
│   ├── create <db_id>     # Create view
│   ├── update <view_id>   # Update view
│   ├── delete <view_id>   # Delete view
│   └── query <view_id>    # Query view results
│
├── datasource
│   ├── get <ds_id>        # Retrieve data source
│   ├── query <ds_id>      # Query data source
│   ├── resolve <db_id>    # Map database ID to data source IDs
│   └── templates          # List data source templates
│
├── api <method> <path>    # Raw API escape hatch (like ntn api)
│   ├── --data <JSON>
│   ├── --header <K:V>
│   └── --query <K=V>
│
└── config
    ├── get <key>          # Read config value
    ├── set <key> <value>  # Set config value
    └── list               # Show all config
```

### Filter DSL (planned)

The current CLI intentionally accepts only `--filter-json`. A future DSL may compile
human-friendly expressions to Notion filter JSON once property-type resolution is defined.

```
# Current syntax
--filter-json '{"property": "Status", "status": {"equals": "Done"}}'
```

### Output Formats

All commands support:
- `--json` -- full API response JSON
- `--plain` -- human-readable text (default for TTY)
- `--yaml` -- YAML output
- `--csv` -- CSV (for `db query`, `user list`, etc.)
- `--tsv` -- TSV variant
- `--id-only` -- just print IDs (for piping)

### Global Flags

```
--token <TOKEN>      # Override auth token
--workspace <ID>     # Override workspace
--profile <NAME>     # Use named profile
--api-version <VER>  # Override Notion-Version header
--verbose            # Debug output to stderr
--dry-run            # Show what would be sent without executing
--no-color           # Disable color output
```

### Config File

`~/.config/notion-cli/config.json`:
```json
{
  "default_profile": "work",
  "profiles": {
    "work": {
      "token": "ntn_...",
      "workspace_id": "..."
    },
    "personal": {
      "token": "ntn_...",
      "workspace_id": "..."
    }
  },
  "defaults": {
    "output_format": "plain",
    "page_size": 50
  }
}
```

---

## Appendix: Error Codes Reference

| HTTP | Code | Meaning |
|------|------|---------|
| 400 | `invalid_json` | Body isn't valid JSON |
| 400 | `invalid_request_url` | Malformed URL |
| 400 | `invalid_request` | Unsupported operation |
| 400 | `validation_error` | Parameters don't match schema |
| 400 | `missing_version` | Missing Notion-Version header |
| 401 | `unauthorized` | Invalid bearer token |
| 403 | `restricted_resource` | Insufficient permissions |
| 404 | `object_not_found` | Resource not found or not shared |
| 409 | `conflict_error` | Data collision |
| 429 | `rate_limited` | Over rate limit (check Retry-After) |
| 500 | `internal_server_error` | Server error |
| 503 | `service_unavailable` | Notion down |
| 503 | `database_connection_unavailable` | DB unreachable |
| 504 | `gateway_timeout` | Request timed out |
| 529 | `service_overload` | Temporary overload |
