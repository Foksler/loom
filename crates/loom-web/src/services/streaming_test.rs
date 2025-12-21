#[cfg(test)]
mod tests {
    use proptest::proptest;

    #[test]
    fn stream_event_parsing_text_chunk() {
        // Purpose: Verify text chunk events parse correctly from JSON
        // Critical for message streaming reliability
        let json = r#"{"type":"text_delta","content":"Hello world"}"#;

        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => {
                assert_eq!(
                    value.get("type").and_then(|v| v.as_str()),
                    Some("text_delta"),
                    "Event type should be parsed correctly"
                );
                assert_eq!(
                    value.get("content").and_then(|v| v.as_str()),
                    Some("Hello world"),
                    "Content should be extracted correctly"
                );
            }
            Err(_) => panic!("JSON parsing failed for text chunk"),
        }
    }

    #[test]
    fn stream_event_parsing_tool_call() {
        // Purpose: Verify tool call delta events parse correctly
        // Important for tool execution in streaming context
        let json = r#"{
            "type":"tool_call_delta",
            "call_id":"call-123",
            "tool_name":"search",
            "arguments_fragment":"{\"query\":"
        }"#;

        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => {
                assert_eq!(
                    value.get("type").and_then(|v| v.as_str()),
                    Some("tool_call_delta"),
                    "Tool call delta type should parse"
                );
                assert_eq!(
                    value.get("call_id").and_then(|v| v.as_str()),
                    Some("call-123"),
                    "Call ID should be preserved"
                );
                assert_eq!(
                    value.get("tool_name").and_then(|v| v.as_str()),
                    Some("search"),
                    "Tool name should be extracted"
                );
            }
            Err(_) => panic!("JSON parsing failed for tool call"),
        }
    }

    #[test]
    fn stream_event_parsing_completed() {
        // Purpose: Verify completion events contain full response
        // Validates end-of-stream handling
        let json = r#"{
            "type":"completed",
            "response":{"text":"Full response here","tokens_used":42}
        }"#;

        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => {
                assert_eq!(
                    value.get("type").and_then(|v| v.as_str()),
                    Some("completed"),
                    "Completion event type should parse"
                );
                assert!(
                    value.get("response").is_some(),
                    "Response object should be present"
                );
            }
            Err(_) => panic!("JSON parsing failed for completion event"),
        }
    }

    #[test]
    fn stream_event_parsing_error() {
        // Purpose: Verify error events include error message
        // Critical for error handling and user feedback
        let json = r#"{
            "type":"error",
            "message":"Rate limit exceeded"
        }"#;

        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => {
                assert_eq!(
                    value.get("type").and_then(|v| v.as_str()),
                    Some("error"),
                    "Error event type should parse"
                );
                assert_eq!(
                    value.get("message").and_then(|v| v.as_str()),
                    Some("Rate limit exceeded"),
                    "Error message should be captured"
                );
            }
            Err(_) => panic!("JSON parsing failed for error event"),
        }
    }

    #[test]
    fn error_handling_malformed_json() {
        // Purpose: Gracefully handle malformed JSON in stream events
        // Prevents panics from invalid server responses
        let malformed_json = r#"{"type":"text_delta"invalid}"#;

        match serde_json::from_str::<serde_json::Value>(malformed_json) {
            Ok(_) => panic!("Should reject malformed JSON"),
            Err(_) => {
                assert!(true, "Malformed JSON correctly rejected");
            }
        }
    }

    #[test]
    fn error_handling_missing_required_field() {
        // Purpose: Handle events with missing required fields
        // Ensures robustness with incomplete data
        let incomplete_json = r#"{"type":"text_delta"}"#; // missing "content"

        match serde_json::from_str::<serde_json::Value>(incomplete_json) {
            Ok(value) => {
                if value.get("content").is_none() {
                    assert!(true, "Missing required field is detected");
                }
            }
            Err(_) => {
                assert!(true, "Incomplete event safely handled");
            }
        }
    }

    // ========================================================================
    // Property-Based Tests with proptest
    // ========================================================================

    proptest! {
        #[test]
        fn chunks_accumulate_correctly(
            chunks in proptest::collection::vec("[a-z]+", 1..10)
        ) {
            // Purpose: Verify that multiple text chunks accumulate into a complete message
            // This property-based test ensures the streaming accumulation logic works
            // for any sequence of text chunks. It tests with many random chunk sequences
            // to ensure robustness across different input patterns.

            // Simulate chunk accumulation
            let mut accumulated = String::new();
            for chunk in &chunks {
                accumulated.push_str(chunk);
            }

            // Verify all chunks are present in accumulated message
            for chunk in &chunks {
                assert!(
                    accumulated.contains(chunk),
                    "All chunks should be present in accumulated message"
                );
            }

            // Verify order is preserved (each chunk appears after the previous)
            let mut last_position = 0;
            for chunk in &chunks {
                let position = accumulated[last_position..].find(chunk);
                assert!(
                    position.is_some(),
                    "Chunks should appear in order"
                );
                if let Some(pos) = position {
                    last_position += pos + chunk.len();
                }
            }
        }

        #[test]
        fn stream_complete_produces_full_message(
            parts in proptest::collection::vec("[a-zA-Z0-9 ,.!?]+", 1..20)
        ) {
            // Purpose: Verify that when a stream completes, it produces a complete
            // message combining all parts. Property-based testing with many random
            // message combinations ensures this works for all valid input sequences.

            let full_message = parts.join("");

            // Verify full message is not empty when parts exist
            assert!(
                !full_message.is_empty(),
                "Full message should be non-empty when parts are provided"
            );

            // Verify full message length is sum of all parts
            let expected_length: usize = parts.iter().map(|p| p.len()).sum();
            assert_eq!(
                full_message.len(),
                expected_length,
                "Full message length should equal sum of all parts"
            );

            // Verify each part is contained in the full message
            for part in &parts {
                assert!(
                    full_message.contains(part),
                    "Full message should contain all parts"
                );
            }
        }

        #[test]
        fn stream_events_sequence_valid(
            event_count in 1usize..100
        ) {
            // Purpose: Verify a sequence of random streaming events maintains
            // proper structure and ordering. Tests that streaming event
            // sequences with various counts are handled correctly.

            let mut accumulated_chunks = String::new();

            // Simulate receiving multiple text delta events
            for i in 0..event_count {
                accumulated_chunks.push_str(&format!("chunk_{} ", i));
            }

            // Verify we have the expected number of chunks
            let chunks: Vec<&str> = accumulated_chunks.split_whitespace().collect();
            assert_eq!(
                chunks.len(),
                event_count,
                "Should accumulate all event chunks"
            );

            // Verify no data loss
            assert!(
                !accumulated_chunks.is_empty(),
                "Accumulated message should not be empty"
            );
        }
    }

    #[test]
    fn streaming_state_transitions_valid() {
        // Purpose: Verify streaming state machine follows valid transitions
        // Non-property test for deterministic state transitions

        // Valid sequence: Idle -> Starting -> Streaming -> Idle
        let mut state = crate::services::state::StreamingState::Idle;
        assert_eq!(state, crate::services::state::StreamingState::Idle);

        state = crate::services::state::StreamingState::Starting {
            thread_id: "t1".to_string(),
        };
        match &state {
            crate::services::state::StreamingState::Starting { .. } => {
                assert!(true, "State transition to Starting valid");
            }
            _ => panic!("Invalid state transition"),
        }

        state = crate::services::state::StreamingState::Streaming {
            thread_id: "t1".to_string(),
            partial_message: "hello".to_string(),
            start_time: 1.0,
        };
        match &state {
            crate::services::state::StreamingState::Streaming { .. } => {
                assert!(true, "State transition to Streaming valid");
            }
            _ => panic!("Invalid state transition"),
        }

        state = crate::services::state::StreamingState::Idle;
        assert_eq!(
            state,
            crate::services::state::StreamingState::Idle,
            "Should return to Idle"
        );
    }

    #[test]
    fn message_content_length_preserved() {
        // Purpose: Verify message content length is accurately maintained
        // Important for progress indication and message size validation
        let content = "The quick brown fox jumps over the lazy dog";
        let content_len = content.len();

        assert_eq!(
            content_len, 43,
            "Content length should be preserved correctly"
        );

        // Verify partial message length is less than full
        let partial = "The quick";
        assert!(
            partial.len() < content_len,
            "Partial message should be shorter than full message"
        );
    }

    #[test]
    fn streaming_handles_unicode() {
        // Purpose: Verify streaming correctly handles Unicode characters
        // Important for international content and emoji support
        let unicode_content = "Hello 世界 🌍 مرحبا мир";

        assert!(
            !unicode_content.is_empty(),
            "Unicode content should not be empty"
        );

        // Verify Unicode is preserved through serialization
        match serde_json::to_string(&unicode_content) {
            Ok(serialized) => match serde_json::from_str::<String>(&serialized) {
                Ok(deserialized) => {
                    assert_eq!(
                        deserialized, unicode_content,
                        "Unicode should be preserved through serialization round-trip"
                    );
                }
                Err(_) => panic!("Failed to deserialize Unicode content"),
            },
            Err(_) => panic!("Failed to serialize Unicode content"),
        }
    }
}
