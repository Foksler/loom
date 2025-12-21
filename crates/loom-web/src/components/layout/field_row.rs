/// FieldRow component - Label + input + error wrapper
///
/// A composite component that wraps input fields with labels,
/// error messages, and proper spacing.
use crate::prelude::*;

/// FieldRow component - Wraps input with label and error message
///
/// Provides consistent spacing and error handling for form inputs.
/// Automatically manages label associations and error styling.
///
/// # Props
/// - `label`: Field label text
/// - `error`: Optional error message
/// - `required`: Display required indicator
/// - `description`: Optional helper text
/// - `children`: Input element(s)
/// - `horizontal`: Display label and input side-by-side
///
/// # Example
///
/// ```rust
/// view! {
///     <FieldRow label="Email" required=true error=None>
///         <TextField
///             input_type="email"
///             placeholder="you@example.com"
///         />
///     </FieldRow>
///
///     <FieldRow
///         label="Password"
///         required=true
///         error=Some("Password too short (min 8 characters)".into())
///     >
///         <TextField input_type="password" placeholder="Enter password" />
///     </FieldRow>
/// }
/// ```
#[component]
pub fn FieldRow(
    /// Label text for the input
    label: String,
    /// Optional error message displayed below input
    #[prop(optional)]
    error: Option<String>,
    /// Mark field as required
    #[prop(default = false)]
    required: bool,
    /// Optional helper text displayed below label
    #[prop(optional)]
    description: Option<String>,
    /// Form input element(s)
    children: Children,
    /// Arrange label and input horizontally
    #[prop(default = false)]
    horizontal: bool,
    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let has_error = error.is_some();
    let field_id = format!("field-{}", uuid::Uuid::new_v4());

    view! {
        <div
            class=format!(
                "{}{}",
                if horizontal { "flex gap-4 items-start" } else { "space-y-2" },
                if let Some(c) = class { format!(" {}", c) } else { String::new() }
            )
        >
            {/* Label */}
            <label
                for=field_id.clone()
                class=format!(
                    "block text-sm font-medium text-gray-700 {}",
                    if horizontal { "flex-shrink-0 pt-2 w-32" } else { "" }
                )
            >
                {label}
                {required.then(|| {
                    view! { <span class="text-red-500 ml-1">*</span> }
                })}
            </label>

            {/* Input wrapper */}
            <div class=if horizontal { "flex-1 space-y-1" } else { "w-full space-y-1" }>
                {/* Description/helper text */}
                {description.map(|desc| {
                    view! {
                        <p class="text-xs text-gray-500">{desc}</p>
                    }
                })}

                {/* Input element(s) - wrap children in a div with proper attributes */}
                <div class="input-wrapper">
                    {children()}
                </div>

                {/* Error message */}
                {has_error.then(|| {
                    view! {
                        <p class="text-sm text-red-600 font-medium">
                            {error}
                        </p>
                    }
                })}
            </div>
        </div>
    }
}

// UUID generation helper
mod uuid {
    use std::fmt;

    pub struct Uuid([u8; 16]);

    impl Uuid {
        pub fn new_v4() -> Self {
            // Simple pseudo-random UUID v4 for field IDs
            // In production, use a proper UUID library
            let mut bytes = [0u8; 16];
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();

            bytes[0..8].copy_from_slice(&timestamp.to_le_bytes()[0..8]);
            bytes[8] = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
                >> 24) as u8;

            Uuid(bytes)
        }
    }

    impl fmt::Display for Uuid {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.0[0], self.0[1], self.0[2], self.0[3],
                self.0[4], self.0[5],
                self.0[6], self.0[7],
                self.0[8], self.0[9],
                self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15]
            )
        }
    }
}
