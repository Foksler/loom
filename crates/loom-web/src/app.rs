/// Root application component
///
/// This is the entry point for the SPA. It sets up:
/// - Global app state via Context providers
/// - Routing
/// - Layout shell
use crate::prelude::*;
use leptos_meta::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};

use crate::components::layout::AppShell;
use crate::routes::*;
use crate::services::state::provide_app_state;

/// Root `<App/>` component
///
/// Wraps the entire application with:
/// - AppState context for global state management
/// - Router for client-side routing
/// - Meta tags for SSR/hydration
#[component]
pub fn App() -> impl IntoView {
    // Setup meta tags
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/loom_web.css"/>
        <Title text="Loom - AI Coding Assistant"/>
        <Meta name="charset" content="utf-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>

        <AppStateProvider>
            <AppShell>
                <Router>
                    <Routes fallback=|| view! { <div>"Not found"</div> }>
                        <Route path=path!("/") view=HomePage/>
                        <Route path=path!("/threads") view=ThreadListPage/>
                        <Route path=path!("/threads/:id") view=ThreadDetailPage/>
                        <Route path=path!("/workspace") view=WorkspacePage/>
                        <ParentRoute path=path!("/styleguide") view=StyleguideLayout>
                            <Route path=path!("") view=StyleguideIndexPage/>
                            <Route path=path!("primitives") view=StyleguidePrimitivesPage/>
                            <Route path=path!("chat") view=StyleguideChatPage/>
                            <Route path=path!("query") view=StyleguideQueryPage/>
                            <Route path=path!("results") view=StyleguideResultsPage/>
                            <Route path=path!("layout") view=StyleguideLayoutPage/>
                        </ParentRoute>
                    </Routes>
                </Router>
            </AppShell>
        </AppStateProvider>
    }
}

/// Provider component for global AppState
///
/// Initializes app state and makes it available to all child components via Context.
#[component]
pub fn AppStateProvider(children: Children) -> impl IntoView {
    provide_app_state();

    view! { {children()} }
}
