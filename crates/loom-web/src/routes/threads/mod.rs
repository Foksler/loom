mod detail;
/// Thread management pages
///
/// - ThreadListPage: List all threads with search/filter
/// - ThreadDetailPage: View single thread conversation
mod list;

pub use detail::ThreadDetailPage;
pub use list::ThreadListPage;
