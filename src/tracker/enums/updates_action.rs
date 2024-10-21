use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum UpdatesAction {
    Add,
    Remove,
    Update,
}