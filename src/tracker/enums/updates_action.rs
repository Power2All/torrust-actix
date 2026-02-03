//! Database update action types.

use serde::Deserialize;

/// The type of pending database update operation.
///
/// Used to track what action should be performed when flushing pending
/// updates to the database. Updates are batched for efficiency and then
/// processed based on their action type.
///
/// # Variants
///
/// - **Add**: Insert a new record (e.g., new torrent, new user)
/// - **Remove**: Delete an existing record
/// - **Update**: Modify an existing record
///
/// # Batching Strategy
///
/// The tracker batches database operations for efficiency:
/// 1. Changes are recorded in memory with their action type
/// 2. Periodically, all pending updates are flushed to the database
/// 3. The action type determines the SQL operation (INSERT, DELETE, UPDATE)
///
/// # Example
///
/// ```rust
/// use torrust_actix::tracker::enums::updates_action::UpdatesAction;
///
/// let action = UpdatesAction::Add;
/// match action {
///     UpdatesAction::Add => { /* INSERT */ }
///     UpdatesAction::Update => { /* UPDATE */ }
///     UpdatesAction::Remove => { /* DELETE */ }
/// }
/// ```
#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum UpdatesAction {
    /// Insert a new record into the database.
    Add,

    /// Delete an existing record from the database.
    Remove,

    /// Modify an existing record in the database.
    Update,
}