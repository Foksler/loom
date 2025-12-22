# Loom Web-Server Integration: Data Flow Reference

**Complete request/response flows for all integrated API endpoints**

---

## 1. GET THREADS (List all threads)

### Client Flow
```
Browser: Click "Threads" sidebar
  ↓
Leptos Component: thread_list.rs
  ↓ calls get_threads()
```

### Server Function Call
```
loom-web/src/server_fns.rs::get_threads() [line 67]
  ✓ Function: pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError>
  ✓ Macro: #[server(GetThreads, "/api")]
  ✓ Logging: info!("Fetching all threads from loom-server")
```

### HTTP Request
```
GET /api/web/threads?limit=50&offset=0 HTTP/1.1
Host: localhost:3000
Content-Type: application/json
User-Agent: reqwest/0.11.x
```

### Backend Handler
```
loom-server/src/web_integration.rs::get_threads_handler() [line 45]
  ✓ Extracts query params: limit=50, offset=0
  ✓ Calls: state.repo.list(workspace=None, limit=50, offset=0)
  ✓ Queries: SELECT * FROM threads LIMIT 50 OFFSET 0
  ✓ Returns: Vec<ThreadSummary>
```

### Database Query
```sql
SELECT 
  id, title, created_at, updated_at, last_activity_at, 
  provider, model
FROM threads
WHERE deleted_at IS NULL
LIMIT 50 OFFSET 0
```

### HTTP Response
```
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 1234

[
  {
    "id": "T-ABC123DEF456",
    "title": "Debug webpack config",
    "created_at": "2025-01-15T10:30:00Z",
    "updated_at": "2025-01-15T10:30:00Z",
    "last_activity_at": "2025-01-15T10:30:00Z",
    "provider": "OpenAI",
    "model": "gpt-4"
  },
  {
    "id": "T-XYZ789GHI012",
    "title": "Rust async patterns",
    "created_at": "2025-01-14T15:45:00Z",
    "updated_at": "2025-01-14T15:45:00Z",
    "last_activity_at": "2025-01-14T15:45:00Z",
    "provider": "Claude",
    "model": "claude-3-opus"
  }
]
```

### Client Processing
```
Leptos Server Function
  ↓ JSON deserialize: Vec<ThreadSummary>
  ↓
Leptos Client Component
  ↓ Update state with threads
  ↓
Browser Render
  ↓
Display thread list
```

### Error Cases

#### Case 1: Server Unreachable
```
Connection Error → ServerFnError::new("Failed to fetch threads: connection refused")
Browser Alert → "Failed to fetch threads"
```

#### Case 2: Malformed Response
```
Invalid JSON → ServerFnError::new("Failed to parse response: invalid type")
Browser Alert → "Failed to parse response"
```

---

## 2. CREATE THREAD (New thread with title)

### Client Flow
```
Browser: Click "Create Thread" button
  ↓
Modal Form: Enter title "Debug webpack config"
  ↓
Click "Create" button
  ↓
```

### Server Function Call
```
loom-web/src/server_fns.rs::create_thread() [line 169]
  ✓ Input: title = "Debug webpack config"
  ✓ Validation:
    - title.is_empty()? → Error
    - title.len() > 500? → Error
  ✓ Generate ID: thread_id = "T-ABC123DEF456" (format: T-{UUID-12-chars})
  ✓ Logging: info!(title = %title, "Creating new thread")
```

### HTTP Request
```
PUT /api/web/threads/T-ABC123DEF456 HTTP/1.1
Host: localhost:3000
Content-Type: application/json
Content-Length: 45

{"title": "Debug webpack config"}
```

### Backend Handler
```
loom-server/src/web_integration.rs::create_thread_handler() [line 95]
  ✓ Extract path param: id = "T-ABC123DEF456"
  ✓ Extract body: CreateThreadRequest { title: "Debug webpack config" }
  ✓ Validate:
    - id.is_empty()? → Error
    - title.is_empty()? → Error
    - title.len() > 500? → Error
  ✓ Parse thread ID: ThreadId::parse("T-ABC123DEF456")
  ✓ Create: Thread::new() { id, metadata.title, ... }
  ✓ Call: state.repo.upsert(thread, expected_version=None)
  ✓ Returns: (StatusCode::CREATED, Json(thread))
```

### Database Operations
```sql
-- Check if exists
SELECT id FROM threads WHERE id = 'T-ABC123DEF456'

-- Insert (if new)
INSERT INTO threads (
  id, title, created_at, updated_at, last_activity_at, 
  version, deleted_at
) VALUES (
  'T-ABC123DEF456',
  'Debug webpack config',
  '2025-01-15T10:35:00Z',
  '2025-01-15T10:35:00Z',
  '2025-01-15T10:35:00Z',
  1,
  NULL
)

-- Return created thread
SELECT * FROM threads WHERE id = 'T-ABC123DEF456'
```

### HTTP Response
```
HTTP/1.1 201 Created
Content-Type: application/json
Location: /api/web/threads/T-ABC123DEF456

{
  "id": "T-ABC123DEF456",
  "title": "Debug webpack config",
  "created_at": "2025-01-15T10:35:00Z",
  "updated_at": "2025-01-15T10:35:00Z",
  "last_activity_at": "2025-01-15T10:35:00Z",
  "provider": null,
  "model": null,
  "conversation": {
    "messages": []
  }
}
```

### Client Processing
```
Leptos Server Function
  ↓ Check status code == 201? (or just 2xx)
  ↓ JSON deserialize: Thread
  ✓ Status 201 CREATED
  ↓
Leptos Client Component
  ↓ Add thread to list
  ↓ Clear form modal
  ↓
Browser Render
  ↓
Display new thread in list
```

### Error Cases

#### Case 1: Empty Title
```
Input: title = ""
Validation: if title.is_empty() { return Error }
Response: 400 Bad Request
Body: {"error": "Thread title cannot be empty"}
```

#### Case 2: Title Too Long
```
Input: title = "A" * 501 (501 characters)
Validation: if title.len() > 500 { return Error }
Response: 400 Bad Request
Body: {"error": "Thread title must be less than 500 characters"}
```

#### Case 3: Invalid Thread ID Format
```
Path: /api/web/threads/invalid-id
Parse: ThreadId::parse("invalid-id") → Error
Response: 400 Bad Request
Body: {"error": "Invalid thread ID"}
```

---

## 3. GET THREAD BY ID (Fetch single thread)

### Client Flow
```
Browser: Click thread in list
  ↓
Leptos Router: Navigate to /thread/{id}
  ↓
Thread Detail Component: Load thread
  ↓ calls get_thread(id)
```

### Server Function Call
```
loom-web/src/server_fns.rs::get_thread() [line 111]
  ✓ Input: id = "T-ABC123DEF456"
  ✓ Validation:
    - id.is_empty()? → Error
  ✓ Logging: info!(thread_id = %id, "Fetching thread with messages")
```

### HTTP Request
```
GET /api/web/threads/T-ABC123DEF456 HTTP/1.1
Host: localhost:3000
Content-Type: application/json
User-Agent: reqwest/0.11.x
```

### Backend Handler
```
loom-server/src/web_integration.rs::get_thread_handler() [line 70]
  ✓ Extract path param: id = "T-ABC123DEF456"
  ✓ Parse thread ID: ThreadId::parse(id)
  ✓ Call: state.repo.get(&thread_id)
  ✓ Check: Some(thread) or NotFound error
  ✓ Returns: Json(thread)
```

### Database Query
```sql
SELECT 
  id, title, created_at, updated_at, last_activity_at,
  provider, model, metadata, conversation, version, deleted_at
FROM threads
WHERE id = 'T-ABC123DEF456' AND deleted_at IS NULL
```

### HTTP Response
```
HTTP/1.1 200 OK
Content-Type: application/json

{
  "id": "T-ABC123DEF456",
  "title": "Debug webpack config",
  "created_at": "2025-01-15T10:35:00Z",
  "updated_at": "2025-01-15T10:35:00Z",
  "last_activity_at": "2025-01-15T10:35:00Z",
  "provider": "OpenAI",
  "model": "gpt-4",
  "conversation": {
    "messages": [
      {
        "role": "user",
        "content": "How do I debug webpack config issues?"
      },
      {
        "role": "assistant",
        "content": "Here are some debugging techniques..."
      }
    ]
  }
}
```

### Client Processing
```
Leptos Server Function
  ↓ JSON deserialize: Thread
  ↓
Leptos Client Component
  ↓ Render thread detail page
  ↓ Display title, messages, metadata
  ↓
Browser Render
  ↓
Show complete thread view
```

### Error Cases

#### Case 1: Empty Thread ID
```
Input: id = ""
Validation: if id.is_empty() { return Error }
Response: 400 Bad Request
```

#### Case 2: Thread Not Found
```
Path: /api/web/threads/T-NONEXISTENT
Query: SELECT * WHERE id = 'T-NONEXISTENT'
Result: No rows returned
Response: 404 Not Found
Body: {"error": "Thread not found"}
```

#### Case 3: Thread Deleted
```
Query result: deleted_at IS NOT NULL
Response: 404 Not Found (treated same as nonexistent)
```

---

## 4. UPDATE THREAD TITLE

### Client Flow
```
Browser: Click edit icon on thread title
  ↓
Modal: Edit title field
  ↓
Click "Save"
  ↓
```

### Server Function Call
```
loom-web/src/server_fns.rs::update_thread() [line 245]
  ✓ Input: id = "T-ABC123DEF456", title = "New webpack guide"
  ✓ Validation:
    - id.is_empty()? → Error
    - title.is_empty()? → Error
    - title.len() > 500? → Error
  ✓ Logging: info!(thread_id = %id, "Updating thread title")
```

### HTTP Request
```
POST /api/web/threads/T-ABC123DEF456 HTTP/1.1
Host: localhost:3000
Content-Type: application/json
Content-Length: 42

{"title": "New webpack guide"}
```

### Backend Handler
```
loom-server/src/web_integration.rs::update_thread_handler() [line 144]
  ✓ Extract path param: id = "T-ABC123DEF456"
  ✓ Extract body: UpdateThreadRequest { title: "New webpack guide" }
  ✓ Validate inputs (all fields)
  ✓ Parse thread ID: ThreadId::parse(id)
  ✓ Fetch current: state.repo.get(&thread_id)
  ✓ Check exists: Some(thread) or NotFound
  ✓ Update: thread.metadata.title = Some(new_title)
  ✓ Call: state.repo.upsert(&thread, None)
  ✓ Returns: Json(updated_thread)
```

### Database Operations
```sql
-- Fetch current thread
SELECT * FROM threads 
WHERE id = 'T-ABC123DEF456' AND deleted_at IS NULL

-- Update thread
UPDATE threads SET
  title = 'New webpack guide',
  updated_at = '2025-01-15T10:40:00Z',
  version = version + 1
WHERE id = 'T-ABC123DEF456'

-- Return updated thread
SELECT * FROM threads WHERE id = 'T-ABC123DEF456'
```

### HTTP Response
```
HTTP/1.1 200 OK
Content-Type: application/json

{
  "id": "T-ABC123DEF456",
  "title": "New webpack guide",
  "created_at": "2025-01-15T10:35:00Z",
  "updated_at": "2025-01-15T10:40:00Z",
  "last_activity_at": "2025-01-15T10:40:00Z",
  "provider": "OpenAI",
  "model": "gpt-4",
  "conversation": { ... }
}
```

### Client Processing
```
Leptos Server Function
  ↓ JSON deserialize: Thread
  ↓
Leptos Client Component
  ↓ Close edit modal
  ↓ Update thread in state
  ↓ Refresh display
  ↓
Browser Render
  ↓
Show updated thread with new title
```

### Error Cases

#### Case 1: Empty New Title
```
Input: title = ""
Validation: if title.is_empty() { return Error }
Response: 400 Bad Request
```

#### Case 2: Thread Not Found
```
Query: SELECT * WHERE id = 'T-ABC123DEF456'
Result: No rows
Response: 404 Not Found
```

---

## 5. DELETE THREAD

### Client Flow
```
Browser: Click delete icon
  ↓
Confirmation: "Are you sure?"
  ↓
Click "Delete"
  ↓
```

### Server Function Call
```
loom-web/src/server_fns.rs::delete_thread() [line 329]
  ✓ Input: id = "T-ABC123DEF456"
  ✓ Validation:
    - id.is_empty()? → Error
  ✓ Logging: info!(thread_id = %id, "Deleting thread")
```

### HTTP Request
```
DELETE /api/web/threads/T-ABC123DEF456 HTTP/1.1
Host: localhost:3000
User-Agent: reqwest/0.11.x
```

### Backend Handler
```
loom-server/src/web_integration.rs::delete_thread_handler() [line 202]
  ✓ Extract path param: id = "T-ABC123DEF456"
  ✓ Validate: !id.is_empty()
  ✓ Parse thread ID: ThreadId::parse(id)
  ✓ Check exists: state.repo.get(&thread_id)
  ✓ Delete: state.repo.delete(&thread_id)
  ✓ Returns: StatusCode::NO_CONTENT (204)
```

### Database Operations
```sql
-- Check exists
SELECT id FROM threads WHERE id = 'T-ABC123DEF456'

-- Soft delete (set deleted_at)
UPDATE threads SET
  deleted_at = '2025-01-15T10:45:00Z'
WHERE id = 'T-ABC123DEF456'
```

### HTTP Response
```
HTTP/1.1 204 No Content
(No body)
```

### Client Processing
```
Leptos Server Function
  ↓ Check status code == 204
  ✓ Success
  ↓
Leptos Client Component
  ↓ Remove thread from list
  ↓ Navigate away from thread (if on detail page)
  ↓
Browser Render
  ↓
Thread removed from display
```

### Error Cases

#### Case 1: Empty Thread ID
```
Input: id = ""
Validation: if id.is_empty() { return Error }
Response: 400 Bad Request
```

#### Case 2: Thread Not Found
```
Query: SELECT * WHERE id = 'T-ABC123DEF456'
Result: No rows
Response: 404 Not Found
```

---

## 6. SEARCH THREADS

### Client Flow
```
Browser: Type in search box
  ↓ (debounced, after 500ms)
Search Bar Component
  ↓ calls search_threads(query)
```

### Server Function Call
```
loom-web/src/server_fns.rs::search_threads() [line 386]
  ✓ Input: query = "webpack"
  ✓ Validation:
    - query.is_empty()? → Error
    - query.len() > 200? → Error
  ✓ Logging: info!(query = %query, "Searching threads")
```

### HTTP Request
```
GET /api/web/threads/search?q=webpack&limit=50&offset=0 HTTP/1.1
Host: localhost:3000
Content-Type: application/json
User-Agent: reqwest/0.11.x
```

### Backend Handler
```
loom-server/src/web_integration.rs::search_threads_handler() [line 237]
  ✓ Extract query param: q = "webpack"
  ✓ Extract pagination: limit=50, offset=0
  ✓ Validate:
    - q.is_empty()? → Error
    - q.len() > 200? → Error
  ✓ Call: state.repo.search(q="webpack", limit=50, offset=0)
  ✓ Returns: Vec<ThreadSummary> (sorted by relevance)
```

### Database Query
```sql
-- Using SQLite FTS (Full Text Search)
SELECT 
  t.id, t.title, t.created_at, t.updated_at, t.last_activity_at,
  t.provider, t.model,
  fts.rank as relevance
FROM threads t
JOIN threads_fts fts ON t.rowid = fts.rowid
WHERE fts MATCH 'webpack' AND t.deleted_at IS NULL
ORDER BY fts.rank DESC
LIMIT 50 OFFSET 0
```

### HTTP Response
```
HTTP/1.1 200 OK
Content-Type: application/json

[
  {
    "id": "T-ABC123DEF456",
    "title": "Debug webpack configuration",
    "created_at": "2025-01-15T10:35:00Z",
    "updated_at": "2025-01-15T10:35:00Z",
    "last_activity_at": "2025-01-15T10:35:00Z",
    "provider": "OpenAI",
    "model": "gpt-4"
  },
  {
    "id": "T-XYZ789GHI012",
    "title": "Webpack advanced techniques",
    "created_at": "2025-01-14T15:45:00Z",
    "updated_at": "2025-01-14T15:45:00Z",
    "last_activity_at": "2025-01-14T15:45:00Z",
    "provider": "Claude",
    "model": "claude-3-opus"
  }
]
```

### Client Processing
```
Leptos Server Function
  ↓ JSON deserialize: Vec<ThreadSummary>
  ↓
Search Bar Component
  ↓ Update results list
  ↓ Render matching threads (ranked by relevance)
  ↓
Browser Render
  ↓
Display search results
```

### Error Cases

#### Case 1: Empty Search Query
```
Input: query = ""
Validation: if query.is_empty() { return Error }
Response: 400 Bad Request
Body: {"error": "Search query cannot be empty"}
```

#### Case 2: Query Too Long
```
Input: query = "A" * 201 (201 characters)
Validation: if query.len() > 200 { return Error }
Response: 400 Bad Request
Body: {"error": "Search query must be less than 200 characters"}
```

#### Case 3: No Results
```
Query: FTS search for "xyzabc123nonexistent"
Result: No rows returned
Response: 200 OK
Body: [] (empty array)
```

---

## Summary of All Flows

| Operation | HTTP Method | Status | Request Size | Response Size |
|-----------|------------|--------|--------------|---------------|
| Get Threads | GET | 200 | 0 bytes | 1-5 KB |
| Create Thread | PUT | 201 | ~100 bytes | 500-800 bytes |
| Get Thread | GET | 200 | 0 bytes | 500-2 KB |
| Update Thread | POST | 200 | ~100 bytes | 500-800 bytes |
| Delete Thread | DELETE | 204 | 0 bytes | 0 bytes |
| Search Threads | GET | 200 | 0 bytes | 1-5 KB |

All flows fully documented and verified ✅
