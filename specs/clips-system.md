<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Clips System Specification

**Status:** Implemented\
**Version:** 1.0\
**Last Updated:** 2026-01-25

---

## 1. Overview

### Purpose

Clips are short-form, shareable code snippets stored as git repositories with automatic secret redaction. Think of them as a self-hosted alternative to GitHub Gists, integrated into the Loom platform with support for organizations, visibility controls, and forking.

### Goals

- **Git-based storage** using bare repositories for clone/push operations
- **Secret redaction** via `loom-redact` when viewing files through the API
- **Visibility levels** (private, internal, public) for access control
- **Organization support** for team-owned clips
- **Forking** to duplicate clips for modification
- **Language detection** for syntax highlighting and filtering
- **Web UI** for browsing, creating, and viewing clips

### Non-Goals (v1)

- Embedded snippets (iframe/oEmbed) - deferred
- Revision history UI - use git directly
- Comments on clips - deferred
- Star/favorite functionality - deferred
- Search across all clips - deferred

---

## 2. Architecture

### Crate Structure

```
crates/
├── loom-server-clips/                # Clips management
│   ├── src/
│   │   ├── lib.rs                    # Re-exports
│   │   ├── error.rs                  # ClipsError enum
│   │   ├── types.rs                  # ClipId, Clip, ClipFile types
│   │   ├── store.rs                  # SqliteClipsRepository
│   │   └── git.rs                    # ClipsGitStore (bare repos)
│   └── Cargo.toml
├── loom-server-db/
│   └── src/
│       └── clips.rs                  # ClipsRepository trait, ClipRecord
├── loom-server-api/
│   └── src/
│       └── clips.rs                  # API types (CreateClipRequest, ClipResponse)
├── loom-server/
│   └── src/
│       └── routes/
│           └── clips/
│               ├── mod.rs            # Module exports
│               ├── handlers.rs       # HTTP handlers
│               └── types.rs          # Error response types

web/
├── loom-web/
│   └── src/
│       ├── lib/
│       │   ├── api/
│       │   │   └── clips.ts          # ClipsApiClient
│       │   └── components/
│       │       └── clips/
│       │           ├── ClipCard.svelte
│       │           ├── ClipList.svelte
│       │           ├── ClipForm.svelte
│       │           ├── ClipFileView.svelte
│       │           └── ClipVisibilityBadge.svelte
│       └── routes/
│           └── (app)/
│               └── clips/
│                   ├── +page.svelte          # List clips
│                   ├── new/+page.svelte      # Create clip
│                   └── [owner]/[name]/+page.svelte  # View clip
```

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Web UI                                          │
├─────────────────────────────────────────────────────────────────────────────┤
│    /clips                    /clips/new              /clips/:owner/:name     │
│    (ClipList)                (ClipForm)              (ClipDetail)            │
└────────┬─────────────────────────┬──────────────────────────┬───────────────┘
         │                         │                          │
         └─────────────────────────┼──────────────────────────┘
                                   │
                                   ▼ REST API
┌─────────────────────────────────────────────────────────────────────────────┐
│                           loom-server                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐              │
│  │ Clips Routes    │  │ User Auth       │  │ Visibility      │              │
│  │ /api/clips/*    │  │ Middleware      │  │ Checks          │              │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘              │
│           │                    │                    │                        │
│           └────────────────────┼────────────────────┘                        │
│                                ▼                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     ClipsGitStore                                     │   │
│  │    Bare git repos → File read → loom_redact::redact() → Response     │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                │                                             │
│                                ▼                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     SqliteClipsRepository                             │   │
│  │    Clip metadata, ownership, visibility                               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                │
                 ┌──────────────┴──────────────┐
                 ▼                             ▼
┌────────────────────────────┐  ┌────────────────────────────────────────────┐
│      SQLite Database        │  │           Git Bare Repositories             │
├────────────────────────────┤  ├────────────────────────────────────────────┤
│  clips                      │  │  /var/lib/loom/clips/{owner}/{name}.git    │
└────────────────────────────┘  └────────────────────────────────────────────┘
```

---

## 3. Core Entities

### 3.1 Clip

Metadata for a code snippet repository.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub owner: String,              // Username or org name
    pub owner_type: OwnerType,      // User or Org
    pub user_id: Option<UserId>,    // Owner if user
    pub org_id: Option<OrgId>,      // Owner if org
    pub name: String,               // URL-safe identifier
    pub description: Option<String>,
    pub visibility: ClipVisibility,
    pub language: Option<String>,   // Primary language detected
    pub clone_url: String,          // Git clone URL
    pub forked_from: Option<ClipId>, // If this is a fork
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClipVisibility {
    Private,   // Only owner can view
    Internal,  // Org members can view (if org-owned)
    Public,    // Anyone can view
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OwnerType {
    User,
    Org,
}
```

### 3.2 ClipFile

A file within a clip, with content optionally redacted.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipFile {
    pub path: String,           // Relative path in repo
    pub content: String,        // File content (may be redacted)
    pub size_bytes: u64,
    pub language: Option<String>,
    pub is_redacted: bool,      // True if secrets were redacted
}
```

### 3.3 ClipId

Type-safe identifier using UUID v7.

```rust
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipId(pub String);

impl ClipId {
    pub fn new() -> Self {
        Self(format!("clip_{}", uuid7()))
    }
}
```

---

## 4. Git Storage

### 4.1 Repository Layout

Clips are stored as bare git repositories:

```
/var/lib/loom/clips/
├── ghuntley/
│   ├── my-snippet.git/       # Bare repo
│   └── another-clip.git/
└── acme-corp/
    └── shared-util.git/
```

### 4.2 ClipsGitStore

```rust
pub struct ClipsGitStore {
    base_path: PathBuf,
}

impl ClipsGitStore {
    /// Initialize a new bare repository for a clip
    pub async fn create_repo(&self, owner: &str, name: &str) -> Result<PathBuf> {
        let path = self.repo_path(owner, name);
        tokio::fs::create_dir_all(&path).await?;

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(&path)
            .output()
            .await?;

        Ok(path)
    }

    /// Read a file from the repository, applying secret redaction
    pub async fn read_file_redacted(
        &self,
        owner: &str,
        name: &str,
        file_path: &str,
        branch: &str,
    ) -> Result<ClipFile> {
        let repo_path = self.repo_path(owner, name);

        // Get file content from git
        let output = Command::new("git")
            .args(["show", &format!("{}:{}", branch, file_path)])
            .current_dir(&repo_path)
            .output()
            .await?;

        let original_content = String::from_utf8_lossy(&output.stdout).to_string();

        // Apply secret redaction
        let (redacted_content, findings) = loom_redact::redact(&original_content);

        Ok(ClipFile {
            path: file_path.to_string(),
            content: redacted_content,
            size_bytes: original_content.len() as u64,
            language: detect_language(file_path),
            is_redacted: !findings.is_empty(),
        })
    }

    /// List all files in the repository
    pub async fn list_files(
        &self,
        owner: &str,
        name: &str,
        branch: &str,
    ) -> Result<Vec<String>> {
        let repo_path = self.repo_path(owner, name);

        let output = Command::new("git")
            .args(["ls-tree", "-r", "--name-only", branch])
            .current_dir(&repo_path)
            .output()
            .await?;

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();

        Ok(files)
    }

    /// Get the clone URL for a clip
    pub fn clone_url(&self, owner: &str, name: &str, base_url: &str) -> String {
        format!("{}/clips/{}/{}.git", base_url, owner, name)
    }

    fn repo_path(&self, owner: &str, name: &str) -> PathBuf {
        self.base_path.join(owner).join(format!("{}.git", name))
    }
}
```

### 4.3 Secret Redaction

When files are read through the API, `loom_redact::redact()` is applied to detect and mask secrets:

```rust
// Before redaction
let content = "API_KEY=sk-1234567890abcdef";

// After redaction
let (redacted, findings) = loom_redact::redact(content);
// redacted = "API_KEY=[REDACTED]"
// findings = [Finding { rule: "generic-api-key", ... }]
```

This protects against accidental exposure of secrets in shared clips.

---

## 5. Visibility & Access Control

### 5.1 Visibility Levels

| Level | Owner | Org Members | Authenticated | Anonymous |
|-------|-------|-------------|---------------|-----------|
| Private | Read/Write | - | - | - |
| Internal | Read/Write | Read | - | - |
| Public | Read/Write | Read | Read | Read |

### 5.2 Access Check

```rust
pub fn can_view_clip(clip: &Clip, user: Option<&CurrentUser>) -> bool {
    match clip.visibility {
        ClipVisibility::Public => true,
        ClipVisibility::Internal => {
            if let Some(user) = user {
                // Owner can always view
                if clip.user_id.as_ref() == Some(&user.id) {
                    return true;
                }
                // Org members can view internal clips
                if let Some(org_id) = &clip.org_id {
                    return user.org_memberships.contains(org_id);
                }
            }
            false
        }
        ClipVisibility::Private => {
            if let Some(user) = user {
                clip.user_id.as_ref() == Some(&user.id)
            } else {
                false
            }
        }
    }
}

pub fn can_edit_clip(clip: &Clip, user: &CurrentUser) -> bool {
    // Only owner can edit
    if clip.user_id.as_ref() == Some(&user.id) {
        return true;
    }
    // Org admins can edit org clips
    if let Some(org_id) = &clip.org_id {
        return user.is_org_admin(org_id);
    }
    false
}
```

---

## 6. API Endpoints

### 6.1 Base Path

All clip endpoints are under `/api/clips/*`.

### 6.2 Clip CRUD

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/clips` | Create a new clip |
| GET | `/api/clips/{owner}/{name}` | Get clip metadata |
| PATCH | `/api/clips/{owner}/{name}` | Update clip metadata |
| DELETE | `/api/clips/{owner}/{name}` | Delete clip |

### 6.3 Clip Listing

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/clips/user` | List current user's clips |
| GET | `/api/clips/org/{org_id}` | List organization's clips |
| GET | `/api/clips/explore` | List public clips |

### 6.4 File Operations

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/clips/{owner}/{name}/files` | List files in clip |
| GET | `/api/clips/{owner}/{name}/files/{path}` | Get file content (redacted) |

### 6.5 Forking

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/clips/{owner}/{name}/fork` | Fork clip to user's namespace |

---

## 7. API Types

### 7.1 CreateClipRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClipRequest {
    pub name: String,                    // URL-safe, max 100 chars
    pub description: Option<String>,     // Max 500 chars
    pub visibility: ClipVisibility,
    pub org_id: Option<OrgId>,          // If creating for org
}
```

### 7.2 UpdateClipRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateClipRequest {
    pub description: Option<String>,
    pub visibility: Option<ClipVisibility>,
}
```

### 7.3 ClipResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipResponse {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub language: Option<String>,
    pub clone_url: String,
    pub forked_from: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

### 7.4 ClipFileResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipFileResponse {
    pub path: String,
    pub content: String,
    pub size_bytes: u64,
    pub language: Option<String>,
    pub is_redacted: bool,
}
```

---

## 8. Database Schema

### 8.1 Migration: `037_clips.sql`

```sql
-- Clips table
CREATE TABLE clips (
    id TEXT PRIMARY KEY,
    owner TEXT NOT NULL,              -- Username or org name
    owner_type TEXT NOT NULL,         -- 'user' or 'org'
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    org_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    visibility TEXT NOT NULL DEFAULT 'private',
    language TEXT,
    forked_from TEXT REFERENCES clips(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(owner, name),
    CHECK (
        (owner_type = 'user' AND user_id IS NOT NULL AND org_id IS NULL) OR
        (owner_type = 'org' AND org_id IS NOT NULL AND user_id IS NULL)
    )
);

CREATE INDEX idx_clips_user_id ON clips(user_id);
CREATE INDEX idx_clips_org_id ON clips(org_id);
CREATE INDEX idx_clips_visibility ON clips(visibility);
CREATE INDEX idx_clips_owner_name ON clips(owner, name);
```

---

## 9. Web UI Components

### 9.1 Component Structure

```
web/loom-web/src/lib/components/clips/
├── ClipCard.svelte            # Card display for clip in list
├── ClipList.svelte            # Grid of clip cards
├── ClipForm.svelte            # Create/edit form
├── ClipFileView.svelte        # File content viewer
├── ClipVisibilityBadge.svelte # Visibility indicator
└── index.ts                   # Re-exports
```

### 9.2 ClipCard Component

```svelte
<script lang="ts">
  import type { Clip } from '$lib/api/clips';
  import ClipVisibilityBadge from './ClipVisibilityBadge.svelte';

  interface Props {
    clip: Clip;
    onclipclick?: (clip: Clip) => void;
  }

  let { clip, onclipclick }: Props = $props();

  function formatDate(dateString: string): string {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  }
</script>

<article class="clip-card" onclick={() => onclipclick?.(clip)}>
  <header>
    <h3>{clip.owner}/{clip.name}</h3>
    <ClipVisibilityBadge visibility={clip.visibility} />
  </header>
  {#if clip.description}
    <p class="description">{clip.description}</p>
  {/if}
  <footer>
    {#if clip.language}
      <span class="language">{clip.language}</span>
    {/if}
    <span class="date">Created {formatDate(clip.created_at)}</span>
  </footer>
</article>
```

### 9.3 ClipFileView Component

```svelte
<script lang="ts">
  import type { ClipFile } from '$lib/api/clips';

  interface Props {
    file: ClipFile;
  }

  let { file }: Props = $props();
</script>

<div class="file-view">
  <header>
    <span class="path">{file.path}</span>
    {#if file.language}
      <span class="language">{file.language}</span>
    {/if}
    {#if file.is_redacted}
      <span class="redacted-badge">Contains redacted content</span>
    {/if}
  </header>
  <pre><code>{file.content}</code></pre>
</div>
```

---

## 10. Git Operations

### 10.1 Clone URL Format

```
https://loom.example.com/clips/{owner}/{name}.git
```

### 10.2 Workflow

1. **Create clip** via API - initializes bare repo
2. **Clone locally** - `git clone <clone_url>`
3. **Add files** - `git add .`
4. **Commit** - `git commit -m "Initial commit"`
5. **Push** - `git push origin main`
6. **View via API** - files are redacted on read

### 10.3 Git HTTP Backend

The server exposes git-http-backend for clone/push operations:

```
GET  /clips/{owner}/{name}.git/info/refs?service=git-upload-pack
POST /clips/{owner}/{name}.git/git-upload-pack
POST /clips/{owner}/{name}.git/git-receive-pack
```

Authentication is required for push operations and private/internal clips.

---

## 11. Language Detection

### 11.1 Detection Logic

Language is detected from file extensions:

```rust
pub fn detect_language(file_path: &str) -> Option<String> {
    let ext = Path::new(file_path).extension()?.to_str()?;

    match ext.to_lowercase().as_str() {
        "rs" => Some("Rust"),
        "ts" | "tsx" => Some("TypeScript"),
        "js" | "jsx" => Some("JavaScript"),
        "py" => Some("Python"),
        "go" => Some("Go"),
        "java" => Some("Java"),
        "rb" => Some("Ruby"),
        "sh" | "bash" => Some("Shell"),
        "sql" => Some("SQL"),
        "json" => Some("JSON"),
        "yaml" | "yml" => Some("YAML"),
        "toml" => Some("TOML"),
        "md" => Some("Markdown"),
        "html" | "htm" => Some("HTML"),
        "css" => Some("CSS"),
        "svelte" => Some("Svelte"),
        _ => None,
    }.map(String::from)
}
```

### 11.2 Primary Language

The clip's primary language is set to the most common language among its files.

---

## 12. Forking

### 12.1 Fork Process

```rust
pub async fn fork_clip(
    original: &Clip,
    user: &CurrentUser,
    git_store: &ClipsGitStore,
    repo: &dyn ClipsRepository,
) -> Result<Clip> {
    // Generate unique fork name
    let fork_name = generate_fork_name(&original.name, &user.username, repo).await?;

    // Create new clip record
    let fork = Clip {
        id: ClipId::new(),
        owner: user.username.clone(),
        owner_type: OwnerType::User,
        user_id: Some(user.id.clone()),
        org_id: None,
        name: fork_name,
        description: original.description.clone(),
        visibility: ClipVisibility::Private, // Forks start private
        language: original.language.clone(),
        forked_from: Some(original.id.clone()),
        ..Default::default()
    };

    // Clone git repo
    git_store.clone_repo(
        &original.owner,
        &original.name,
        &fork.owner,
        &fork.name,
    ).await?;

    repo.create_clip(&fork).await?;

    Ok(fork)
}
```

---

## 13. Configuration

### 13.1 Environment Variables

| Variable | Type | Description | Default |
|----------|------|-------------|---------|
| `LOOM_CLIPS_ENABLED` | boolean | Enable clips feature | `true` |
| `LOOM_CLIPS_BASE_PATH` | path | Git repos storage path | `/var/lib/loom/clips` |
| `LOOM_CLIPS_MAX_FILE_SIZE` | bytes | Max file size to display | `1048576` (1MB) |
| `LOOM_CLIPS_MAX_FILES` | integer | Max files per clip | `100` |

---

## 14. Error Handling

### 14.1 ClipsError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ClipsError {
    #[error("clip not found")]
    NotFound,

    #[error("clip already exists")]
    AlreadyExists,

    #[error("access denied")]
    AccessDenied,

    #[error("invalid clip name: {0}")]
    InvalidName(String),

    #[error("git operation failed: {0}")]
    Git(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

### 14.2 HTTP Status Mapping

| Error | HTTP Status |
|-------|-------------|
| NotFound | 404 |
| AlreadyExists | 409 |
| AccessDenied | 403 |
| InvalidName | 400 |
| Git | 500 |
| Database | 500 |

---

## 15. Implementation Checklist

### Phase 1: Core Types & Database (Completed)

- [x] Create `loom-server-clips` crate
- [x] Define Clip, ClipFile, ClipId types
- [x] Define ClipVisibility enum
- [x] Add database migration (037_clips.sql)
- [x] Create ClipsRepository trait and implementation

### Phase 2: Git Storage (Completed)

- [x] Implement ClipsGitStore
- [x] Bare repository creation
- [x] File listing
- [x] File reading with redaction

### Phase 3: API Endpoints (Completed)

- [x] Create clip handler
- [x] Get clip handler
- [x] Update clip handler
- [x] Delete clip handler
- [x] List user clips handler
- [x] List org clips handler
- [x] List public clips handler
- [x] List files handler
- [x] Get file handler
- [x] Fork clip handler

### Phase 4: Web UI (Completed)

- [x] ClipCard component
- [x] ClipList component
- [x] ClipForm component
- [x] ClipFileView component
- [x] ClipVisibilityBadge component
- [x] Clips list page
- [x] New clip page
- [x] Clip detail page
- [x] Navigation link

### Phase 5: Git HTTP Backend (Planned)

- [ ] git-upload-pack endpoint
- [ ] git-receive-pack endpoint
- [ ] Authentication for push
- [ ] Pack file handling

---

## 16. Security Considerations

### 16.1 Secret Redaction

All file content returned through the API is processed by `loom_redact::redact()`, which uses gitleaks patterns to detect and mask:

- API keys
- Access tokens
- Passwords
- Private keys
- Connection strings

### 16.2 Name Validation

Clip names are validated to prevent path traversal:

```rust
fn validate_clip_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 100 {
        return Err(ClipsError::InvalidName("name must be 1-100 characters".into()));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(ClipsError::InvalidName("invalid characters in name".into()));
    }
    if name.starts_with('.') || name.contains("..") {
        return Err(ClipsError::InvalidName("name cannot start with dot or contain ..".into()));
    }
    Ok(())
}
```

### 16.3 Git Command Injection

All git operations use fixed command structures with parameters passed as arguments, not interpolated into command strings.

---

## 17. Future Considerations

| Feature | Description |
|---------|-------------|
| Embedded snippets | oEmbed/iframe support for embedding in docs |
| Revision history UI | Browse commit history in web UI |
| Comments | Discussion threads on clips |
| Stars/favorites | Bookmark clips for quick access |
| Search | Full-text search across public clips |
| Syntax highlighting | Server-side or client-side highlighting |
| Line linking | Link to specific lines in files |
| Raw file access | Direct download of files |
