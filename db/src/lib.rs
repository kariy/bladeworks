use bladeworks_context::Ctx;
use katana_db::mdbx::DbEnv;
use katana_primitives::{
    class::ClassHash,
    contract::{ContractAddress, Nonce, StorageKey, StorageValue},
};
use katana_provider::{traits::state::StateProvider, ProviderResult};

pub struct Db {
    ctx: Ctx,
    db: DbEnv,
}

// actual_storage_key = tenant_id + key

impl StateProvider for Db {
    fn storage(
        &self,
        address: ContractAddress,
        storage_key: StorageKey,
    ) -> ProviderResult<Option<StorageValue>> {
        todo!()
    }

    fn class_hash_of_contract(
        &self,
        address: ContractAddress,
    ) -> ProviderResult<Option<ClassHash>> {
        todo!()
    }

    fn nonce(&self, address: ContractAddress) -> ProviderResult<Option<Nonce>> {
        todo!()
    }
}
