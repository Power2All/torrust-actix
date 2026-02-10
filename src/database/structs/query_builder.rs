use crate::database::enums::database_drivers::DatabaseDrivers;

#[derive(Debug, Clone)]
pub struct QueryBuilder {
    pub engine: DatabaseDrivers,
}