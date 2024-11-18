use sqlx::{Pool, Postgres};

#[derive(Debug, Clone)]
pub struct DatabaseConnectorPgSQL {
    pub(crate) pool: Pool<Postgres>,
}