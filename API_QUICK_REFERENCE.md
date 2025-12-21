# API Server Functions - Quick Reference

## All 7 Functions at a Glance

```rust
// 1. GET ALL THREADS
#[server(GetThreads, "/api")]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError>

// 2. GET SINGLE THREAD
#[server(GetThread, "/api")]
pub async fn get_thread(id: String) -> Result<Thread, ServerFnError>

// 3. CREATE THREAD
#[server(CreateThread, "/api")]
pub async fn create_thread(title: String) -> Result<Thread, ServerFnError>

// 4. ADD MESSAGE
#[server(AddMessage, "/api")]
pub async fn add_message(
    thread_id: String,
    content: String,
    role: String,  // "user" | "assistant" | "system"
) -> Result<Message, ServerFnError>

// 5. UPDATE THREAD
#[server(UpdateThread, "/api")]
pub async fn update_thread(id: String, title: String) -> Result<Thread, ServerFnError>

// 6. DELETE THREAD
#[server(DeleteThread, "/api")]
pub async fn delete_thread(id: String) -> Result<(), ServerFnError>

// 7. SEARCH THREADS
#[server(SearchThreads, "/api")]
pub async fn search_threads(query: String) -> Result<Vec<ThreadSummary>, ServerFnError>
```

---

## Input Validation Rules

| Function | Parameter | Rules |
|----------|-----------|-------|
| `create_thread` | `title` | Non-empty, ≤500 chars |
| `add_message` | `thread_id` | Non-empty |
| | `content` | Non-empty, ≤10,000 chars |
| | `role` | Must be "user", "assistant", or "system" |
| `update_thread` | `id` | Non-empty |
| | `title` | Non-empty, ≤500 chars |
| `delete_thread` | `id` | Non-empty |
| `search_threads` | `query` | Non-empty, ≤200 chars |

---

## Return Types

```rust
// Thread Summary (for lists)
pub struct ThreadSummary {
    pub id: String,
    pub title: String,
    pub updated_at: DateTime<Utc>,
    pub provider: String,
    pub status: ThreadStatus,  // Active | Archived | Processing
}

// Complete Thread (with messages)
pub struct Thread {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub model: String,
    pub status: ThreadStatus,
    pub messages: Vec<Message>,
    pub repository: Option<String>,
    pub tools: Vec<String>,
}

// Message
pub struct Message {
    pub id: String,
    pub content: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}
```

---

## Logging Levels

| Function | Level | Message |
|----------|-------|---------|
| `get_threads()` | INFO | "Fetching all threads from database" |
| | DEBUG | "Successfully fetched threads" (count: N) |
| `get_thread(id)` | INFO | "Fetching thread with messages" |
| | ERROR | "Invalid thread ID: empty string" |
| | DEBUG | "Successfully fetched thread" (message_count) |
| `create_thread(title)` | INFO | "Creating new thread" (title) |
| | ERROR | "Thread title exceeds maximum length" |
| | INFO | "Successfully created thread" (thread_id) |
| `add_message(...)` | INFO | "Adding message to thread" |
| | ERROR | "Invalid message role" |
| | INFO | "Successfully added message" |
| `update_thread(id, title)` | INFO | "Updating thread title" |
| | INFO | "Successfully updated thread title" |
| `delete_thread(id)` | INFO | "Deleting thread" |
| | INFO | "Successfully deleted thread" |
| `search_threads(query)` | INFO | "Searching threads" |
| | DEBUG | "Executing full-text search" |
| | INFO | "Search completed" (result_count) |

---

## Usage in Components

```rust
use leptos::*;
use crate::services::api::*;

// Load threads on component mount
let threads = create_resource(
    || (),
    |_| async { get_threads().await }
);

// Get single thread
let thread = create_resource(
    move || thread_id.get(),
    |id| async move { get_thread(id).await }
);

// Create thread
let create_action = create_action(|title: &String| {
    let title = title.clone();
    async move { create_thread(title).await }
});

// Add message
let add_msg = create_action(|(thread_id, content, role): &(String, String, String)| {
    let (tid, cnt, r) = (thread_id.clone(), content.clone(), role.clone());
    async move { add_message(tid, cnt, r).await }
});

// Update thread
let update_action = create_action(|(id, title): &(String, String)| {
    let (i, t) = (id.clone(), title.clone());
    async move { update_thread(i, t).await }
});

// Delete thread
let delete_action = create_action(|id: &String| {
    let id = id.clone();
    async move { delete_thread(id).await }
});

// Search threads
let search = create_action(|query: &String| {
    let q = query.clone();
    async move { search_threads(q).await }
});
```

---

## File Locations

- **Implementation**: `crates/loom-web/src/services/api.rs` (411 lines)
- **Types**: `crates/loom-web/src/components/threads/mod.rs`
- **Exports**: `crates/loom-web/src/services/mod.rs`
- **Full Docs**: `API_SERVER_FUNCTIONS.md`
- **Dev Guide**: `IMPLEMENTATION_API_SERVER_FUNCTIONS.md`
- **Summary**: `IMPLEMENTATION_SUMMARY_API_SERVER_FUNCTIONS.md`

---

## Key Facts

✅ All 7 functions implemented with full docstrings  
✅ Type-safe RPC via Leptos `#[server]` macro  
✅ Comprehensive input validation  
✅ Structured logging with `#[instrument]`  
✅ Mock data ready for testing  
✅ Zero compilation errors (API-specific)  
✅ Ready for database layer integration  

---

## Status

| Function | Status | DB Ready |
|----------|--------|----------|
| `get_threads()` | ✅ | ❌ (mock data) |
| `get_thread(id)` | ✅ | ❌ (mock data) |
| `create_thread(title)` | ✅ | ❌ (mock data) |
| `add_message(...)` | ✅ | ❌ (mock data) |
| `update_thread(id, title)` | ✅ | ❌ (mock data) |
| `delete_thread(id)` | ✅ | ❌ (mock data) |
| `search_threads(query)` | ✅ | ❌ (needs FTS) |

All functions have skeleton implementations with mock data, ready for actual database queries.

