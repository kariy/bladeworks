pub mod schema;

use katana_db::{
    abstraction::DbTx,
    codecs::Encode,
    error::DatabaseError,
    mdbx::{libmdbx::TransactionKind, DbEnv},
    tables::Table,
};
use katana_provider::traits::{Provider, ProviderMut};

struct Db {
    env: DbEnv,
}

impl Db {
    fn provider(&self, tenant: u64) -> Box<dyn Provider> {
        todo!()
    }

    fn provider_mut(&self, tenant: u64) -> Box<dyn ProviderMut> {
        todo!()
    }
}

struct Tx<K: TransactionKind> {
    tenant: u64,
    tx: katana_db::mdbx::tx::Tx<K>,
}

impl<K: TransactionKind> Tx<K> {
    fn new(tenant: u64, tx: katana_db::mdbx::tx::Tx<K>) -> Self {
        Self { tenant, tx }
    }
}

impl<K: TransactionKind> DbTx for Tx<K> {
    fn get<T: Table>(&self, key: T::Key) -> Result<Option<T::Value>, DatabaseError> {
        let key = self.tenant_key::<T>(key);
        todo!()
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
