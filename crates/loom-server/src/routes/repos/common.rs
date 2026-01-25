// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for repository routes.

use axum::{http::StatusCode, Json};
use loom_server_auth::middleware::CurrentUser;
use loom_server_auth::types::{OrgId, OrgRole};
use loom_server_scm::{RepoRole, RepoTeamAccessStore, Repository};
use std::path::PathBuf;
use uuid::Uuid;

use crate::api::AppState;
use crate::i18n::t;

// Re-export API types
pub use loom_server_api::repos::*;

pub fn get_repos_base_dir() -> PathBuf {
	std::env::var("LOOM_SERVER_DATA_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|_| PathBuf::from("/var/lib/loom"))
		.join("repos")
}

pub fn get_repo_disk_path(repo_id: Uuid) -> PathBuf {
	let id_str = repo_id.to_string();
	let shard = &id_str[..2];
	get_repos_base_dir().join(shard).join(&id_str).join("git")
}

pub fn build_clone_url(base_url: &str, owner_name: &str, repo_name: &str) -> String {
	format!(
		"{}/git/{}/{}.git",
		base_url.trim_end_matches('/'),
		owner_name,
		repo_name
	)
}

pub fn repo_to_response(repo: Repository, clone_url: String) -> RepoResponse {
	RepoResponse {
		id: repo.id,
		owner_type: repo.owner_type.into(),
		owner_id: repo.owner_id,
		name: repo.name,
		visibility: repo.visibility.into(),
		default_branch: repo.default_branch,
		clone_url,
		created_at: repo.created_at,
		updated_at: repo.updated_at,
	}
}

pub fn generate_initial_readme(repo_name: &str) -> String {
	format!(
		r#"# {repo_name}

Hello World! Welcome to your new repository.

## Getting Started

Clone this repository:

```bash
git clone <your-clone-url>
cd {repo_name}
```

## GitHub Flavored Markdown Examples

This README demonstrates various GitHub Flavored Markdown features for testing markdown rendering.

### Text Formatting

- **Bold text** using `**bold**`
- *Italic text* using `*italic*`
- ~~Strikethrough~~ using `~~strikethrough~~`
- `Inline code` using backticks
- ***Bold and italic*** using `***bold and italic***`

### Links and Images

- [External link](https://example.com)
- [Link with title](https://example.com "Example Title")
- Autolinked URL: https://example.com
- Email autolink: user@example.com

### Lists

#### Unordered List
- Item 1
- Item 2
  - Nested item 2.1
  - Nested item 2.2
- Item 3

#### Ordered List
1. First item
2. Second item
   1. Nested item 2.1
   2. Nested item 2.2
3. Third item

### Task List

- [x] Completed task
- [ ] Incomplete task
- [ ] Another task to do

### Code Blocks

```rust
fn main() {{
    println!("Hello, World!");
}}
```

```python
def hello():
    print("Hello, World!")
```

```javascript
const greeting = () => console.log("Hello, World!");
```

### Tables

| Feature | Supported | Notes |
|---------|-----------|-------|
| Headers | Yes | Using `#` syntax |
| Tables | Yes | GFM extension |
| Task lists | Yes | `- [x]` syntax |
| Footnotes | Partial | Depends on renderer |

### Blockquotes

> This is a blockquote.
> It can span multiple lines.
>
> > Nested blockquotes are also possible.

### Horizontal Rule

---

### Headings

The document uses headings from `#` (H1) through `######` (H6).

### Emoji (if supported)

:rocket: :tada: :sparkles:

### Footnotes

Here's a sentence with a footnote[^1].

[^1]: This is the footnote content.

### Alerts (GitHub specific)

> [!NOTE]
> Useful information that users should know.

> [!TIP]
> Helpful advice for doing things better.

> [!WARNING]
> Urgent info that needs immediate attention.

## Contributing

Feel free to open issues and pull requests!

## License

Add your license information here.
"#,
		repo_name = repo_name
	)
}

pub async fn check_repo_admin_access(
	current_user: &CurrentUser,
	repo: &Repository,
	state: &AppState,
	locale: &str,
) -> Result<(), (StatusCode, Json<RepoErrorResponse>)> {
	use loom_server_scm::OwnerType;

	let is_admin = match repo.owner_type {
		OwnerType::User => repo.owner_id == current_user.user.id.into_inner(),
		OwnerType::Org => {
			let org_id = OrgId::new(repo.owner_id);
			match state
				.org_repo
				.get_membership(&org_id, &current_user.user.id)
				.await
			{
				Ok(Some(m)) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
				_ => false,
			}
		}
	};

	if !is_admin {
		if let Some(store) = &state.scm_team_access_store {
			if let Ok(Some(role)) = store
				.get_user_role_via_teams(current_user.user.id.into_inner(), repo.id)
				.await
			{
				if role == RepoRole::Admin {
					return Ok(());
				}
			}
		}
		return Err((
			StatusCode::FORBIDDEN,
			Json(RepoErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.scm.admin_required").to_string(),
			}),
		));
	}

	Ok(())
}
