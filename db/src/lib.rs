use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, path::Path};

use anyhow::Result;
use katana_db::{abstraction::Database, error::DatabaseError};
use katana_db::{init_db, mdbx};
use katana_provider::providers::db::DbProvider;
use parking_lot::Mutex;

#[derive(Debug)]
pub struct Db<D: Database> {
    inner: Arc<Inner<D>>,
}

#[derive(Debug)]
struct Inner<D: Database> {
    // mapping from tenant id to their database environment
    databases: Mutex<HashMap<u8, D>>,
    // path to the root directory containing the tenant databases
    root_path: PathBuf,
}

impl<D: Database> Db<D> {
    // createa a provider for the given tenant
    pub fn provider(&self, tenant: u8) -> Result<Option<DbProvider<D::Tx>>, DatabaseError> {
        let dbs = self.inner.databases.lock();
        let Some(env) = dbs.get(&tenant) else {
            return Ok(None);
        };
        Ok(Some(DbProvider::new(env.tx()?)))
    }

    // create a mutable provider for the given tenant
    pub fn provider_mut(&self, tenant: u8) -> Result<Option<DbProvider<D::TxMut>>, DatabaseError> {
        let dbs = self.inner.databases.lock();
        let Some(env) = dbs.get(&tenant) else {
            return Ok(None);
        };
        Ok(Some(DbProvider::new(env.tx_mut()?)))
    }
}

impl Db<mdbx::DbEnv> {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let root_path = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&root_path)?;
        let mut databases = HashMap::new();

        for entry in root_path.read_dir()? {
            let entry = entry?;
            let tenant = entry.file_name().to_string_lossy().parse::<u8>()?;
            databases.insert(tenant, init_db(entry.path())?);
        }

        Ok(Self {
            inner: Arc::new(Inner {
                root_path,
                databases: Mutex::new(databases),
            }),
        })
    }

    pub fn create_tenant(&self, tenant: u8) -> Result<()> {
        let path = self.inner.root_path.join(tenant.to_string());
        self.inner.databases.lock().insert(tenant, init_db(path)?);
        Ok(())
    }

    pub fn drop_tenant(&self, tenant: u8) -> Result<()> {
        // get the path to the tenant's database directory
        let path = self.inner.root_path.join(tenant.to_string());
        self.inner.databases.lock().remove(&tenant);
        std::fs::remove_dir_all(path)?;
        Ok(())
    }
}
