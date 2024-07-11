pub mod provider;

use bladeworks_context::Ctx;
use katana_db::codecs::{Compress, Encode};
use katana_db::error::DatabaseError;
use katana_db::mdbx;
use katana_db::mdbx::libmdbx::ffi::DBI;
use katana_db::mdbx::libmdbx::TransactionKind;
use katana_db::mdbx::libmdbx::RW;

use katana_db::mdbx::DbEnv;
use katana_db::tables;
use katana_db::tables::Table;
use katana_primitives::{
    class::ClassHash,
    contract::{ContractAddress, Nonce, StorageKey, StorageValue},
};
use katana_provider::{traits::state::StateProvider, ProviderResult};

use std::str::FromStr;

pub struct Db<K: TransactionKind>(Tx<K>);

pub struct Tx<K: TransactionKind> {
    ctx: Ctx,
    inner: katana_db::mdbx::tx::Tx<K>,
}

impl<K: TransactionKind> Tx<K> {
    pub fn new(inner: katana_db::mdbx::tx::Tx<K>, ctx: Ctx) -> Self {
        Self { inner, ctx }
    }

    fn tenant_key<T: AsRef<[u8]>>(&self, key: T) -> Vec<u8> {
        let mut tenant_key = self.ctx.tenant_id.to_be_bytes().to_vec();
        tenant_key.extend_from_slice(key.as_ref());
        tenant_key
    }

    pub fn cursor<T: Table>(&self) -> Result<katana_db::mdbx::cursor::Cursor<K, T>, DatabaseError> {
        self.inner.cursor::<T>()
    }

    pub fn get_dbi<T: Table>(&self) -> Result<DBI, DatabaseError> {
        self.inner.get_dbi::<T>()
    }

    pub fn get<T: Table>(&self, key: T::Key) -> Result<Option<<T as Table>::Value>, DatabaseError> {
        let tenant_key = self.tenant_key(Encode::encode(key));
        self.inner
            .inner
            .get(self.inner.get_dbi::<T>()?, tenant_key.as_ref())
            .map_err(DatabaseError::Read)?
            .map(decode_one::<T>)
            .transpose()
    }

    pub fn entries<T: Table>(&self) -> Result<usize, DatabaseError> {
        self.inner.entries::<T>()
    }

    pub fn commit(self) -> Result<bool, DatabaseError> {
        self.inner.commit()
    }
}

impl<K: TransactionKind> Db<K> {
    pub fn new(tx: katana_db::mdbx::tx::Tx<K>, ctx: Ctx) -> Self {
        Self(Tx::new(tx, ctx))
    }
}

impl Tx<RW> {
    pub fn put<T: Table>(&self, key: T::Key, value: T::Value) -> Result<(), DatabaseError> {
        let key = key.encode();
        let value = value.compress();
        self.inner
            .put(self.get_dbi::<T>()?, key, value, WriteFlags::UPSERT)
            .unwrap();
        Ok(())
    }

    pub fn delete<T: Table>(
        &self,
        key: T::Key,
        value: Option<T::Value>,
    ) -> Result<bool, DatabaseError> {
        let tenant_key = self.tenant_key(Encode::encode(key));
        self.inner.delete::<T>(tenant_key, value)
    }

    pub fn clear<T: Table>(&self) -> Result<(), DatabaseError> {
        self.inner.clear::<T>()
    }

    pub fn abort(self) {
        self.inner.abort()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use katana_db::mdbx::test_utils::create_test_db;
//     use katana_db::mdbx::DbEnvKind;
//     use katana_primitives::FieldElement;

//     #[test]
//     fn test_tenant_isolation() {
//         let db_env = create_test_db(DbEnvKind::RW);

//         // Create two different tenant contexts
//         let ctx1 = Ctx { tenant_id: 1 };
//         let ctx2 = Ctx { tenant_id: 2 };

//         // Insert data for tenant 1
//         {
//             let tx = db_env.tx_mut().unwrap();
//             let db = Db::new(tx, ctx1);
//             db.0.put::<tables::BlockHashes>(0, FieldElement::from(1u64))
//                 .unwrap();
//             db.0.commit().unwrap();
//         }

//         // Insert data for tenant 2
//         {
//             let tx = db_env.tx_mut().unwrap();
//             let db = Db::new(tx, ctx2);
//             db.0.put::<tables::BlockHashes>(0, FieldElement::from(2u64))
//                 .unwrap();
//             db.0.commit().unwrap();
//         }

//         // Verify tenant 1 data
//         {
//             let tx = db_env.tx().unwrap();
//             let db = Db::new(tx, ctx1);
//             let result = db.0.get::<tables::BlockHashes>(0).unwrap();
//             assert_eq!(result, Some(FieldElement::from(1u64)));
//         }

//         // Verify tenant 2 data
//         {
//             let tx = db_env.tx().unwrap();
//             let db = Db::new(tx, ctx2);
//             let result = db.0.get::<tables::BlockHashes>(0).unwrap();
//             assert_eq!(result, Some(FieldElement::from(2u64)));
//         }

//         // Verify tenant isolation
//         {
//             let tx = db_env.tx().unwrap();
//             let db = Db::new(tx, ctx1);
//             let result = db.0.get::<tables::BlockHashes>(0).unwrap();
//             assert_ne!(result, Some(FieldElement::from(2u64)));
//         }
//     }
// }
