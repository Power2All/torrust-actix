use sqlx::{MySql, Pool};

#[derive(Clone)]
pub struct DatabaseConnectorMySQL {
    pub(crate) pool: Pool<MySql>,
}
