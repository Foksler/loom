# LLM Query Handler Implementation Summary

## Overview
Fully implemented `crates/loom-server/src/llm_query_handler.rs` with complete functionality for detecting and processing LLM queries embedded in text output.

## Components Implemented

### 1. SimpleRegexDetector
A regex-based detector that identifies query patterns in LLM output.

#### Patterns Supported:
- **ReadFile**: Triggers like "I need to read", "show me", "let me read", "read", "get file"
  - Requires file path with extension (`.txt`, `.json`) or absolute path (`/etc/passwd`)
  - Pattern: `r"(?i)(?:i need to read|let me read|show me|read|get file)\s+(?:the\s+)?(?:file\s+)?([^\s,;?!]*[/\.][^\s,;?!]*)"`
  - Extracted via `extract_path()` which handles quotes and path normalization

- **ExecuteCommand**: Triggers "run", "execute", "run command"
  - Pattern: `r"(?i)(?:run|execute|run command)\s+(.+?)(?:\s+or\s+|\s+and\s+|$)"`
  - Extracted via `extract_command()` which splits into command and args

- **GetEnvironment**: Triggers "get environment", "what's in", "show me env", "check env"
  - Pattern: `r"(?i)(?:get environment|what's in|show me env|check env)\b(?:\s+(.+))?$"`
  - Extracted via `extract_env_vars()` which parses and uppercases var names

- **RequestUserInput**: Triggers "ask", "request", "prompt" + "user"
  - Pattern: `r"(?i)(?:ask|request|prompt)\s+(?:the\s+)?user(?:\s+(.*))?$"`
  - Extracts prompt text, defaults to "Please provide input"

#### Helper Methods:
- `extract_path(text)`: Removes quotes, ensures absolute paths start with `/`
- `extract_command(text)`: Splits on whitespace, returns (command, args_vec)
- `extract_env_vars(text)`: Filters valid env var names, uppercases them
- `generate_query_id()`: Creates UUID-based IDs with `Q-` prefix

### 2. LlmQueryHandler
Main handler coordinating detection and processing of queries.

#### Key Methods:
```rust
pub fn new(detector: SimpleRegexDetector, query_manager: Arc<ServerQueryManager>) -> Self
pub fn with_default_detector(query_manager: Arc<ServerQueryManager>) -> Self
pub async fn handle_llm_output(&self, session_id: &str, llm_output: &str) 
    -> Result<Option<ServerQueryResponse>, ServerQueryError>
pub async fn handle_llm_output_batch(&self, session_id: &str, llm_output: &str)
    -> Result<Vec<ServerQueryResponse>, ServerQueryError>
```

#### Behavior:
- `handle_llm_output`: Returns first detected query result (Phase 2)
- `handle_llm_output_batch`: Processes all detected queries sequentially (Phase 3)
- All operations instrumented with `#[instrument]` for structured logging

### 3. Query Metadata
All generated queries include:
- Unique ID starting with `Q-`
- Kind-specific fields (path, command, env vars, prompt)
- `sent_at`: RFC3339 timestamp
- `timeout_secs`: Per-type timeout (ReadFile: 10s, ExecuteCommand: 30s, RequestUserInput: 60s, GetEnvironment: 10s)
- `metadata`: JSON with detector source ("simple_regex")

## Test Coverage

### Unit Tests (14 total, all with documentation)
1. **test_detect_read_file_patterns**: Path extraction from natural language
   - Tests: simple files, absolute paths, relative paths, quoted paths
   - Validates: normalization, quote removal, path format

2. **test_detect_execute_command_patterns**: Command detection and parsing
   - Tests: command with args, argument extraction
   - Validates: proper command/args separation

3. **test_detect_get_environment_patterns**: Env var detection
   - Tests: multiple vars, case handling
   - Validates: uppercasing, parsing

4. **test_detect_user_input_patterns**: User input detection
   - Tests: prompt extraction, defaults
   - Validates: prompt content, input type consistency

5. **test_multiple_queries_detected**: Multiple query detection
   - Tests: compound sentences with multiple query types
   - Validates: detector handles complex outputs

6. **test_malformed_input_handling**: Edge case robustness
   - Tests: empty strings, random text, incomplete patterns
   - Validates: no panics, returns Ok(vec![])

7. **test_path_extraction**: Path normalization
   - Tests: various path formats and quote styles
   - Validates: quote removal, path prefix addition

8. **test_command_extraction**: Command parsing
   - Tests: commands with args, single commands
   - Validates: proper argument splitting

9. **test_env_var_extraction**: Env var parsing
   - Tests: multiple vars, separators, empty input
   - Validates: uppercasing, filtering

10. **test_generated_query_ids_are_valid**: ID format validation
    - Tests: ID prefix, length
    - Validates: correlation capability

11. **test_handler_no_queries**: Handler graceful handling
    - Tests: regular text without queries
    - Validates: returns None (no processing)

12. **test_handler_with_default_detector**: Constructor convenience
    - Tests: default detector creation
    - Validates: proper initialization

13. **test_query_metadata**: Metadata correctness
    - Tests: metadata presence and content
    - Validates: detector attribution

14. **test_query_timeouts**: Timeout configuration per type
    - Tests: timeouts for each query type
    - Validates: ReadFile=10s, Command=30s, Input=60s

### Test Quality
- All tests include `/// **Why Important**:` documentation explaining test purpose
- Tests use property-based approach (testing multiple cases per test)
- Unit tests isolate detector behavior
- Async tests verify handler integration
- Error cases explicitly tested

## Logging

All public methods instrumented with `#[instrument]`:
- Session tracking via `session_id` field
- Query tracking via `query_id` field
- Output size tracking via `output_len`
- Debug logs for detection results
- Info logs for successful processing
- Warn logs for timeouts and errors

## Design Decisions

1. **Regex-based Detection**: Simple, fast, easy to extend vs ML models
2. **Required Path Separators**: Prevents false positives like "show me the file"
3. **Quote Handling**: Strips quotes in extraction for CLI compatibility
4. **Path Normalization**: Ensures consistency across relative/absolute paths
5. **Per-Type Timeouts**: Different query types have different expected response times
6. **Metadata**: Enables debugging and monitoring of query sources
7. **First-Query Processing (Phase 2)**: Returns immediately to unblock LLM continuation
8. **Batch Processing (Phase 3)**: Allows future expansion for multiple queries

## Migration Path

- **Phase 2**: Current implementation - single query processing
- **Phase 3**: WebSocket migration - use same detector/handler, transport changes only
- **Future**: Could add ML-based detector as alternative to regex without changing handler interface

## File Location
`crates/loom-server/src/llm_query_handler.rs` - 724 lines
- Exports: `SimpleRegexDetector`, `LlmQueryHandler`
- Public API: Fully documented with examples in tests
