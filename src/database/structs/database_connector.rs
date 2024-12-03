use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector_mysql::DatabaseConnectorMySQL;
use crate::database::structs::database_connector_pgsql::DatabaseConnectorPgSQL;
use crate::database::structs::database_connector_sqlite::DatabaseConnectorSQLite;

#[derive(Debug, Clone)]
pub struct DatabaseConnector {
    pub(crate) mysql: Option<DatabaseConnectorMySQL>,
    pub(crate) sqlite: Option<DatabaseConnectorSQLite>,
    pub(crate) pgsql: Option<DatabaseConnectorPgSQL>,
    pub(crate) engine: Option<DatabaseDrivers>,
}