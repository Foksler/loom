/// Routes and pages for the Loom SPA
///
/// Organized by feature area:
/// - Home / workspace
/// - Threads (list & detail)
/// - Styleguide (component gallery)
pub mod home;
pub mod styleguide;
pub mod threads;
pub mod workspace;

pub use home::HomePage;
pub use styleguide::{
    StyleguideChatPage, StyleguideIndexPage, StyleguideLayout, StyleguideLayoutPage,
    StyleguidePrimitivesPage, StyleguideQueryPage, StyleguideResultsPage,
};
pub use threads::*;
pub use workspace::WorkspacePage;
