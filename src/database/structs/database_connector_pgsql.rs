use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct DatabaseConnectorPgSQL {
    pub(crate) pool: Pool<Postgres>,
}
