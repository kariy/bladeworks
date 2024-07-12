use std::collections::HashMap;

use katana_db::{abstraction::Database, error::DatabaseError};
use katana_provider::providers::db::DbProvider;

#[derive(Debug)]
pub struct Db<D: Database> {
    // mapping from tenant id to their database environment
    databases: HashMap<u8, D>,
}

impl<D: Database> Db<D> {
    pub fn provider(&self, tenant: u8) -> Result<Option<DbProvider<D::Tx>>, DatabaseError> {
        let Some(env) = self.databases.get(&tenant) else {
            return Ok(None);
        };

        Ok(Some(DbProvider::new(env.tx()?)))
    }

    pub fn provider_mut(&self, tenant: u8) -> Result<Option<DbProvider<D::TxMut>>, DatabaseError> {
        let Some(env) = self.databases.get(&tenant) else {
            return Ok(None);
        };

        Ok(Some(DbProvider::new(env.tx_mut()?)))
    }
}

impl<D: Database> Default for Db<D> {
    fn default() -> Self {
        Self {
            databases: HashMap::new(),
        }
    }
}
