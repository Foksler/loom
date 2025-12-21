/// Query bridge components gallery
use crate::components::query::*;
use crate::prelude::*;
use serde_json::json;

#[component]
pub fn StyleguideQueryPage() -> impl IntoView {
    // Mock data for demonstrations
    let sample_steps = vec![
        QueryStep::new("Parse Query")
            .with_description("Parsing and validating user input")
            .with_log("Query received: 'What are the benefits?'")
            .with_log("Validation passed")
            .with_inputs(json!({ "query": "What are the benefits?" }))
            .with_outputs(json!({ "tokens": 5, "confidence": 0.95 })),
        QueryStep::new("Route to LLM")
            .with_description("Determining which LLM model to use")
            .with_log("Query routed to OpenAI GPT-4")
            .with_log("Model selected: gpt-4-turbo"),
        QueryStep::new("Execute Query")
            .with_description("Sending query to language model")
            .with_log("Request sent to LLM")
            .with_log("Awaiting response...")
            .with_inputs(json!({ "model": "gpt-4-turbo", "temperature": 0.7 })),
        QueryStep::new("Tool Invocation")
            .with_description("Invoking search tools as requested by LLM")
            .with_log("Tool call: search_documents")
            .with_log("Found 12 matching documents"),
        QueryStep::new("Format Response")
            .with_description("Formatting the final response")
            .with_outputs(json!({ "answer": "Benefits include...", "sources": 5 })),
    ];

    let sample_invocations = vec![
        ToolInvocation::new(
            "search_documents",
            json!({ "query": "benefits", "limit": 10 }),
            "Found 12 documents matching 'benefits'",
            245,
        ),
        ToolInvocation::new(
            "get_document",
            json!({ "id": "doc_123" }),
            "Retrieved document with 2500 tokens of content",
            89,
        ),
        ToolInvocation::new(
            "summarize",
            json!({ "text": "...", "length": "short" }),
            "Summary generated: Benefits include increased productivity...",
            567,
        ),
    ];

    let sample_transitions = vec![
        StateTransition::new("idle", "parsing", "2024-01-15T10:00:00Z", "query_received"),
        StateTransition::new(
            "parsing",
            "routing",
            "2024-01-15T10:00:01Z",
            "parse_complete",
        ),
        StateTransition::new(
            "routing",
            "executing",
            "2024-01-15T10:00:02Z",
            "route_selected",
        ),
        StateTransition::new(
            "executing",
            "tools",
            "2024-01-15T10:00:05Z",
            "tool_call_needed",
        ),
        StateTransition::new(
            "tools",
            "formatting",
            "2024-01-15T10:00:08Z",
            "tools_complete",
        ),
        StateTransition::new(
            "formatting",
            "complete",
            "2024-01-15T10:00:09Z",
            "formatting_done",
        ),
    ];

    view! {
        <div class="p-8 max-w-6xl">
            <h1 class="text-4xl font-bold mb-4 text-gray-900">Query Bridge</h1>
            <p class="text-lg text-gray-600 mb-8">
                Components for query visualization, state machine trace, and tool invocations.
            </p>

            {/* Query Timeline section */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-4 text-gray-900">Query Timeline</h2>
                <p class="text-gray-600 mb-4">
                    Vertical timeline showing query execution steps with status indicators.
                </p>
                <QueryTimeline
                    steps=sample_steps.clone().into()
                />
            </section>

            {/* Tool Invocations section */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-4 text-gray-900">Tool Invocations</h2>
                <p class="text-gray-600 mb-4">
                    List of tools called during query execution with execution times and results.
                </p>
                <ToolInvocationList invocations=sample_invocations.into() />
            </section>

            {/* State Machine Trace section */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-4 text-gray-900">State Machine Trace</h2>
                <p class="text-gray-600 mb-4">
                    Timeline of state transitions showing the flow through the query processing pipeline.
                </p>
                <StateMachineTrace trace=sample_transitions.into() />
            </section>

            {/* Collapsed Timeline Example */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-4 text-gray-900">Timeline - Early Stage</h2>
                <p class="text-gray-600 mb-4">
                    Example of timeline at an early execution stage.
                </p>
                <QueryTimeline
                    steps=sample_steps.clone().into()
                />
            </section>
        </div>
    }
}
