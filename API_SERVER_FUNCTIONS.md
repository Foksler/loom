# API Server Functions Documentation

## Overview

The `services/api.rs` module implements 7 Leptos server functions for managing threads and messages in the Loom application. Each function uses the `#[server]` macro to create type-safe RPC endpoints that are automatically serialized/deserialized.

## Architecture

- **Framework**: Leptos 0.7 with `#[server]` macro
- **Route**: All endpoints at `/api` prefix
- **Serialization**: serde + JSON (automatic)
- **Error Handling**: `ServerFnError` with descriptive messages
- **Logging**: Structured tracing with `#[instrument]` macro

## Server Functions

### 1. `get_threads()`

**Signature**:
```rust
#[server(GetThreads, "/api")]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError>
```

**Purpose**: Fetch all threads from the database for the thread list sidebar.

**Returns**: 
- `Ok(Vec<ThreadSummary>)` - List of thread summaries containing id, title, updated_at, provider, and status
- `Err(ServerFnError)` - Database errors

**Logging**:
- `INFO`: "Fetching all threads from database"
- `DEBUG`: "Successfully fetched threads" with count

**Implementation Notes**:
- No input validation required (no parameters)
- Should use indexed queries for performance
- Consider caching for frequently accessed list
- TODO: Query SQLite via loom-server

**Example**:
```rust
let threads = get_threads().await?;
for thread in threads {
    println!("{}: {}", thread.id, thread.title);
}
```

---

### 2. `get_thread(id: String)`

**Signature**:
```rust
#[server(GetThread, "/api")]
pub async fn get_thread(id: String) -> Result<Thread, ServerFnError>
```

**Purpose**: Fetch a single thread with all its messages.

**Parameters**:
- `id` (String) - Thread identifier

**Returns**:
- `Ok(Thread)` - Complete thread with messages, metadata, and tools
- `Err(ServerFnError)` - Thread not found or database error

**Validation**:
- Thread ID cannot be empty
- Thread must exist in database

**Logging**:
- `INFO`: "Fetching thread with messages" (with thread_id)
- `ERROR`: "Invalid thread ID: empty string"
- `DEBUG`: "Successfully fetched thread" (with message_count)

**Implementation Notes**:
- Validates ID is non-empty
- Should fetch thread + all associated messages in one query
- Consider JOIN optimization for performance
- TODO: Query SQLite for thread and messages

**Example**:
```rust
match get_thread("thread_001".to_string()).await {
    Ok(thread) => println!("Thread: {}", thread.title),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

### 3. `create_thread(title: String)`

**Signature**:
```rust
#[server(CreateThread, "/api")]
pub async fn create_thread(title: String) -> Result<Thread, ServerFnError>
```

**Purpose**: Create a new thread in the database.

**Parameters**:
- `title` (String) - Thread title/name

**Returns**:
- `Ok(Thread)` - Newly created thread with generated ID and timestamps
- `Err(ServerFnError)` - Validation or database error

**Validation**:
- Title cannot be empty
- Title must be ≤ 500 characters
- Generates UUID-based thread ID

**Logging**:
- `INFO`: "Creating new thread" (with title)
- `ERROR`: Error messages with validation failures
- `INFO`: "Successfully created thread" (with thread_id)

**Implementation Notes**:
- Generates thread ID using UUID v4 (truncated)
- Initializes empty message list
- Sets created_at and updated_at to current time
- Status defaults to Active
- TODO: Insert into SQLite database

**Example**:
```rust
match create_thread("Debug webpack issues".to_string()).await {
    Ok(thread) => println!("Created: {}", thread.id),
    Err(e) => eprintln!("Failed: {}", e),
}
```

---

### 4. `add_message(thread_id: String, content: String, role: String)`

**Signature**:
```rust
#[server(AddMessage, "/api")]
pub async fn add_message(
    thread_id: String,
    content: String,
    role: String,
) -> Result<Message, ServerFnError>
```

**Purpose**: Add a message to an existing thread.

**Parameters**:
- `thread_id` (String) - ID of the thread
- `content` (String) - Message content
- `role` (String) - Message role: "user", "assistant", or "system"

**Returns**:
- `Ok(Message)` - Created message with ID and timestamp
- `Err(ServerFnError)` - Validation or database error

**Validation**:
- Thread ID cannot be empty
- Content cannot be empty
- Content must be ≤ 10,000 characters
- Role must be "user", "assistant", or "system"

**Logging**:
- `INFO`: "Adding message to thread" (with thread_id, role, content_len)
- `ERROR`: Validation errors with details
- `INFO`: "Successfully added message" (with message_id)

**Implementation Notes**:
- Generates message ID using UUID v4 (truncated)
- Sets created_at to current time
- Updates thread's updated_at timestamp
- TODO: Insert into SQLite database

**Example**:
```rust
match add_message(
    "thread_001".to_string(),
    "Help me debug this".to_string(),
    "user".to_string()
).await {
    Ok(msg) => println!("Message added: {}", msg.id),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

### 5. `update_thread(id: String, title: String)`

**Signature**:
```rust
#[server(UpdateThread, "/api")]
pub async fn update_thread(id: String, title: String) -> Result<Thread, ServerFnError>
```

**Purpose**: Update a thread's title.

**Parameters**:
- `id` (String) - Thread identifier
- `title` (String) - New thread title

**Returns**:
- `Ok(Thread)` - Updated thread with new title and updated_at timestamp
- `Err(ServerFnError)` - Thread not found or validation error

**Validation**:
- ID cannot be empty
- Title cannot be empty
- Title must be ≤ 500 characters

**Logging**:
- `INFO`: "Updating thread title" (with thread_id)
- `ERROR`: Validation errors
- `INFO`: "Successfully updated thread title"

**Implementation Notes**:
- Updates title field
- Updates updated_at to current time
- TODO: Update SQLite database

**Example**:
```rust
match update_thread("thread_001".to_string(), "New title".to_string()).await {
    Ok(thread) => println!("Updated: {}", thread.title),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

### 6. `delete_thread(id: String)`

**Signature**:
```rust
#[server(DeleteThread, "/api")]
pub async fn delete_thread(id: String) -> Result<(), ServerFnError>
```

**Purpose**: Delete a thread and all associated messages.

**Parameters**:
- `id` (String) - Thread identifier

**Returns**:
- `Ok(())` - Success
- `Err(ServerFnError)` - Thread not found or database error

**Validation**:
- ID cannot be empty

**Logging**:
- `INFO`: "Deleting thread" (with thread_id)
- `ERROR`: Validation errors
- `INFO`: "Successfully deleted thread"

**Implementation Notes**:
- This operation is permanent
- Should cascade delete all associated messages
- TODO: Delete from SQLite database with cascading

**Example**:
```rust
match delete_thread("thread_001".to_string()).await {
    Ok(_) => println!("Thread deleted"),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

### 7. `search_threads(query: String)`

**Signature**:
```rust
#[server(SearchThreads, "/api")]
pub async fn search_threads(query: String) -> Result<Vec<ThreadSummary>, ServerFnError>
```

**Purpose**: Full-text search threads by title and message content.

**Parameters**:
- `query` (String) - Search query

**Returns**:
- `Ok(Vec<ThreadSummary>)` - Matching threads in relevance order
- `Err(ServerFnError)` - Search error

**Validation**:
- Query cannot be empty
- Query must be ≤ 200 characters

**Logging**:
- `INFO`: "Searching threads" (with query)
- `ERROR`: Validation errors
- `DEBUG`: "Executing full-text search"
- `INFO`: "Search completed" (with result_count)

**Implementation Notes**:
- Should use SQLite FTS (Full Text Search) for performance
- Results should be ranked by relevance
- TODO: Implement full-text search in SQLite

**Example**:
```rust
match search_threads("webpack debug".to_string()).await {
    Ok(results) => println!("Found {} threads", results.len()),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Type Definitions

All types used in server functions are Serialize/Deserialize:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub id: String,
    pub title: String,
    pub updated_at: DateTime<Utc>,
    pub provider: String,
    pub status: ThreadStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadStatus {
    Active,
    Archived,
    Processing,
}
```

---

## Error Handling

All functions return `Result<T, ServerFnError>`:

```rust
// Success case
Ok(value)

// Error cases
Err(ServerFnError::new_default("Human-readable error message"))
```

Common error scenarios:
- **Validation errors**: Empty IDs, titles, oversized content
- **Not found errors**: Thread or message not in database
- **Database errors**: Connection, query, or constraint violations

---

## Structured Logging

All functions use `#[instrument]` macro with field tracking:

```rust
#[instrument(skip_all, fields(thread_id = %id))]
pub async fn get_thread(id: String) -> Result<Thread, ServerFnError> {
    info!("Fetching thread with messages");
    // ...
}
```

Log levels:
- `INFO`: Function start/completion
- `ERROR`: Validation/database failures
- `DEBUG`: Detailed operation steps

---

## Integration Examples

### Using in Leptos Component

```rust
use leptos::*;
use crate::services::api::*;

#[component]
fn ThreadList() -> impl IntoView {
    let threads = create_resource(|| (), |_| async { get_threads().await });
    
    view! {
        <Transition fallback=|| view! { <p>"Loading..."</p> }>
            {move || threads.read().map(|threads| {
                threads.into_iter().map(|thread| {
                    view! {
                        <div>{thread.title}</div>
                    }
                }).collect_view()
            })}
        </Transition>
    }
}
```

### Server-side Backend Integration

```rust
// Implement actual database queries
#[server(GetThreads, "/api")]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    // Query SQLite via loom-server
    let db = get_database()?;
    let threads = db.query_all_threads()?;
    Ok(threads)
}
```

---

## Performance Considerations

1. **Indexing**: Add database indexes on:
   - `threads.id` (PRIMARY KEY)
   - `threads.created_at` (for sorting)
   - `messages.thread_id` (for JOINs)

2. **Caching**: Consider caching:
   - Frequently accessed thread lists
   - Search results

3. **Query Optimization**:
   - Fetch thread + messages in single query
   - Use FTS for text search
   - Limit result sets

4. **Pagination**: Consider adding offset/limit for large datasets

---

## Testing

Property-based tests verify:
- UUID generation works
- Input validation catches invalid data
- Error messages are descriptive
- Timestamps are set correctly

```rust
#[test]
fn test_thread_id_generation() {
    let id = format!("thread_{}", Uuid::new_v4().to_string().replace("-", "")[..12].to_string());
    assert!(id.starts_with("thread_"));
    assert!(id.len() > 10);
}
```

---

## Implementation Status

| Function | Status | Notes |
|----------|--------|-------|
| `get_threads()` | ✅ Skeleton | Mock data, needs DB query |
| `get_thread(id)` | ✅ Skeleton | Mock data, needs DB query |
| `create_thread(title)` | ✅ Skeleton | Needs DB insert |
| `add_message(...)` | ✅ Skeleton | Needs DB insert |
| `update_thread(id, title)` | ✅ Skeleton | Needs DB update |
| `delete_thread(id)` | ✅ Skeleton | Needs DB delete with cascade |
| `search_threads(query)` | ✅ Skeleton | Needs FTS implementation |

---

## Next Steps

1. **Database Layer**: Implement actual SQLite queries
2. **Connection Pool**: Add database connection pooling
3. **Caching**: Implement L1/L2 caching strategy
4. **Pagination**: Add offset/limit to list endpoints
5. **Advanced Search**: Implement full-text search with ranking
6. **Streaming**: Add streaming support for long-running operations
7. **Testing**: Add integration tests with test database
