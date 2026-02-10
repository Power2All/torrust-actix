use sqlx::{
    MySql,
    Pool
};

#[derive(Debug, Clone)]
pub struct DatabaseConnectorMySQL {
    pub(crate) pool: Pool<MySql>,
}