use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone)]
pub struct DatabaseConnectorSQLite {
    pub(crate) pool: Pool<Sqlite>,
}