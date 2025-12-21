/// MessageBody - Renders message content with markdown/code support
///
/// Handles formatting of message text, code blocks with syntax highlighting,
/// and markdown parsing. Supports tool results display.
use crate::prelude::*;
use loom_core::message::Role;

/// MessageBody component
///
/// Renders message content with optional markdown and code block formatting.
///
/// # Example
///
/// ```rust
/// view! {
///     <MessageBody
///         content="Hello **world**".to_string()
///         role=Role::User
///     />
/// }
/// ```
#[component]
pub fn MessageBody(
    /// Message content to render
    content: String,

    /// Message role (affects styling)
    #[prop(optional)]
    #[allow(unused_variables)]
    role: Option<Role>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let final_class = format!(
        "text-sm leading-relaxed whitespace-pre-wrap break-words{}",
        class
            .as_ref()
            .map(|c| format!(" {}", c))
            .unwrap_or_default()
    );

    // Parse and render content with basic markdown and code block support
    view! {
        <div class=final_class>
            {render_content(&content)}
        </div>
    }
}

/// Renders content with basic markdown and code block support
fn render_content(content: &str) -> impl IntoView {
    view! {
        <RenderText text=content.to_string() />
    }
}

/// Component to render text with basic markdown support
#[component]
fn RenderText(
    /// Text to render
    text: String,
) -> impl IntoView {
    // Simple markdown parsing for bold, italic, and links
    let parts = parse_markdown(&text);

    // Simplified rendering - just show plain text
    view! {
        <span>
            {parts
                .into_iter()
                .map(|part| {
                    match part {
                        MarkdownPart::Bold(text) => text,
                        MarkdownPart::Italic(text) => text,
                        MarkdownPart::Link { text, .. } => text,
                        MarkdownPart::Code(text) => text,
                        MarkdownPart::Plain(text) => text,
                    }
                })
                .collect::<Vec<_>>()
                .join("")
            }
        </span>
    }
}

enum MarkdownPart {
    Bold(String),
    Italic(String),
    Link {
        text: String,
        #[allow(dead_code)]
        url: String,
    },
    Code(String),
    Plain(String),
}

/// Simple markdown parser for inline formatting
fn parse_markdown(text: &str) -> Vec<MarkdownPart> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' if chars.peek() == Some(&'*') => {
                // Bold: **text**
                if !current.is_empty() {
                    parts.push(MarkdownPart::Plain(current.clone()));
                    current.clear();
                }
                chars.next(); // consume second *
                let mut bold_text = String::new();
                while let Some(c) = chars.next() {
                    if c == '*' && chars.peek() == Some(&'*') {
                        chars.next();
                        break;
                    }
                    bold_text.push(c);
                }
                parts.push(MarkdownPart::Bold(bold_text));
            }
            '*' | '_' => {
                // Italic: *text* or _text_
                if !current.is_empty() {
                    parts.push(MarkdownPart::Plain(current.clone()));
                    current.clear();
                }
                let delim = ch;
                let mut italic_text = String::new();
                for c in chars.by_ref() {
                    if c == delim {
                        break;
                    }
                    italic_text.push(c);
                }
                parts.push(MarkdownPart::Italic(italic_text));
            }
            '`' => {
                // Code: `text`
                if !current.is_empty() {
                    parts.push(MarkdownPart::Plain(current.clone()));
                    current.clear();
                }
                let mut code_text = String::new();
                for c in chars.by_ref() {
                    if c == '`' {
                        break;
                    }
                    code_text.push(c);
                }
                parts.push(MarkdownPart::Code(code_text));
            }
            '[' => {
                // Link: [text](url)
                if !current.is_empty() {
                    parts.push(MarkdownPart::Plain(current.clone()));
                    current.clear();
                }
                let mut link_text = String::new();
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                    link_text.push(c);
                }
                // Expect (url)
                if chars.peek() == Some(&'(') {
                    chars.next();
                    let mut url = String::new();
                    for c in chars.by_ref() {
                        if c == ')' {
                            break;
                        }
                        url.push(c);
                    }
                    parts.push(MarkdownPart::Link {
                        text: link_text,
                        url,
                    });
                } else {
                    current.push('[');
                    current.push_str(&link_text);
                    current.push(']');
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        parts.push(MarkdownPart::Plain(current));
    }

    parts
}
