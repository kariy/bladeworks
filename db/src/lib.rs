use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use anyhow::Result;
use katana_db::{abstraction::Database, error::DatabaseError};
use katana_db::{init_db, mdbx};
use katana_provider::providers::db::DbProvider;

#[derive(Debug)]
pub struct Db<D: Database> {
    // mapping from tenant id to their database environment
    databases: HashMap<u8, D>,
    // path to the root directory containing the tenant databases
    root_path: PathBuf,
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

impl Db<mdbx::DbEnv> {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let root_path = path.as_ref().to_path_buf();
        let mut databases = HashMap::new();

        for entry in root_path.read_dir()? {
            let entry = entry?;
            let tenant = entry.file_name().to_string_lossy().parse::<u8>()?;
            databases.insert(tenant, init_db(entry.path())?);
        }

        Ok(Self {
            databases,
            root_path,
        })
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let root_path = path.as_ref().to_path_buf();
        let databases = HashMap::new();

        std::fs::create_dir_all(&root_path)?;

        Ok(Self {
            databases,
            root_path,
        })
    }

    pub fn create_tenant(&mut self, tenant: u8) -> Result<()> {
        let path = self.root_path.join(tenant.to_string());
        self.databases.insert(tenant, init_db(path)?);
        Ok(())
    }

    pub fn drop_tenant(&mut self, tenant: u8) -> Result<()> {
        let path = self.root_path.join(tenant.to_string());
        self.databases.remove(&tenant);
        std::fs::remove_dir_all(path)?;
        Ok(())
    }
}
