use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct DatabaseConnectorSQLite {
    pub(crate) pool: Pool<Sqlite>,
}