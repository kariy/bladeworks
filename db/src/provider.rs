use katana_db::{
    mdbx::{self, tx::TransactionKind},
    tables,
};
use katana_primitives::{
    class::ClassHash,
    contract::{ContractAddress, Nonce, StorageKey, StorageValue},
};
use katana_provider::{traits::state::StateProvider, ProviderResult};

use crate::Db;
