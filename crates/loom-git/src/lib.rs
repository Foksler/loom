mod detect;
mod error;
mod normalize;

pub use detect::{
    current_branch, default_remote_url, detect_repo_metadata, detect_repo_status, head_commit_sha,
    is_dirty, RepoMetadata,
};
pub use error::GitError;
pub use normalize::normalize_remote_url;

/// Commit information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// Full 40-character SHA
    pub sha: String,
    /// Optional short subject line
    pub summary: Option<String>,
    /// Commit timestamp (unix epoch seconds)
    pub timestamp: Option<i64>,
}

/// Full repository status with commit and dirty info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    /// Current branch name; None if detached HEAD
    pub branch: Option<String>,
    /// Normalized remote URL slug
    pub remote_slug: Option<String>,
    /// HEAD commit information
    pub head: Option<CommitInfo>,
    /// Whether working tree has uncommitted changes
    pub is_dirty: Option<bool>,
}
