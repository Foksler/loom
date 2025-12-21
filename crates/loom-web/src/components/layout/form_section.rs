/// FormSection component - Group form fields with visual grouping
///
/// A composite component that provides visual structure and organization
/// for form elements, with support for titles, descriptions, and validation errors.
use crate::prelude::*;

/// FormSection component - Group related form fields
///
/// Provides a semantic container for organizing related form fields
/// with consistent styling, descriptions, and error messaging.
///
/// # Props
/// - `title`: Section heading
/// - `description`: Optional descriptive text
/// - `children`: Form field components
/// - `error`: Optional error message for the entire section
/// - `disabled`: Disable all child interactions
///
/// # Example
///
/// ```rust
/// view! {
///     <FormSection title="Account Settings" description="Update your account information">
///         <FieldRow label="Username" required=true>
///             <TextField placeholder="Enter username" />
///         </FieldRow>
///         <FieldRow label="Email" required=true>
///             <TextField input_type="email" placeholder="your@email.com" />
///         </FieldRow>
///     </FormSection>
/// }
/// ```
#[component]
pub fn FormSection(
    /// Section title/heading
    title: String,
    /// Optional descriptive text
    #[prop(optional)]
    description: Option<String>,
    /// Form field children
    children: Children,
    /// Optional section-level error message
    #[prop(optional)]
    error: Option<String>,
    /// Disable all child interactions
    #[prop(default = false)]
    disabled: bool,
    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    view! {
        <div class=format!(
            "space-y-4 rounded-lg border border-gray-200 bg-white p-6 {}",
            class.unwrap_or_default()
        )>
            {/* Header */}
            <div class="border-b border-gray-200 pb-4">
                <h3 class="text-lg font-semibold text-gray-900">{title}</h3>
                {description.map(|desc| {
                    view! {
                        <p class="mt-1 text-sm text-gray-600">{desc}</p>
                    }
                })}
            </div>

            {/* Error message (section-level) */}
            {error.map(|err| {
                view! {
                    <div class="rounded-md bg-red-50 p-3 text-sm text-red-800 border border-red-200">
                        <div class="flex gap-2">
                            <span class="text-red-600 font-semibold">"✕"</span>
                            <span>{err}</span>
                        </div>
                    </div>
                }
            })}

            {/* Form fields - wrap in disabled container if needed */}
            <div
                class=if disabled {
                    "opacity-60 pointer-events-none space-y-4"
                } else {
                    "space-y-4"
                }
            >
                {children()}
            </div>
        </div>
    }
}
