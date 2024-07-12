use katana_primitives::block::{BlockHash, BlockNumber, FinalityStatus, Header};
use katana_primitives::class::{ClassHash, CompiledClass, CompiledClassHash, FlattenedSierraClass};
use katana_primitives::contract::{ContractAddress, GenericContractInfo, StorageKey};
use katana_primitives::receipt::Receipt;
use katana_primitives::trace::TxExecInfo;
use katana_primitives::transaction::{Tx, TxHash, TxNumber};

use katana_db::codecs::{Decode, Encode};
use katana_db::error::CodecError;
use katana_db::models::block::StoredBlockBodyIndices;
use katana_db::models::contract::{
    ContractClassChange, ContractInfoChangeList, ContractNonceChange,
};
use katana_db::models::list::BlockList;
use katana_db::models::storage::{ContractStorageEntry, ContractStorageKey, StorageEntry};
use katana_db::tables::{DupSort, Table};
use katana_db::{dupsort, tables};

#[derive(Debug, Clone)]
struct TenantKey<K> {
    id: u64,
    key: K,
}

impl<K: Encode> Encode for TenantKey<K> {
    type Encoded = Vec<u8>;
    fn encode(self) -> Vec<u8> {
        let mut key = self.id.to_be_bytes().to_vec();
        key.extend_from_slice(self.key.encode().as_ref());
        key
    }
}

impl<K: Decode> Decode for TenantKey<K> {
    fn decode<B: AsRef<[u8]>>(bytes: B) -> Result<Self, CodecError> {
        let bytes = bytes.as_ref();
        let id = u64::decode(&bytes[..8])?;
        let key = K::decode(&bytes[8..])?;
        Ok(Self { id, key })
    }
}

tables! {
    /// Store canonical block headers
    Headers: (TenantKey<BlockNumber>) => Header,
    /// Stores block hashes according to its block number
    BlockHashes: (TenantKey<BlockNumber>) => BlockHash,
    /// Stores block numbers according to its block hash
    BlockNumbers: (TenantKey<BlockHash>) => BlockNumber,
    /// Stores block finality status according to its block number
    BlockStatusses: (TenantKey<BlockNumber>) => FinalityStatus,
    /// Block number to its body indices which stores the tx number of
    /// the first tx in the block and the number of txs in the block.
    BlockBodyIndices: (TenantKey<BlockNumber>) => StoredBlockBodyIndices,
    /// Transaction number based on its hash
    TxNumbers: (TenantKey<TxHash>) => TxNumber,
    /// Transaction hash based on its number
    TxHashes: (TenantKey<TxNumber>) => TxHash,
    /// Store canonical transactions
    Transactions: (TenantKey<TxNumber>) => Tx,
    /// Stores the block number of a transaction.
    TxBlocks: (TenantKey<TxNumber>) => BlockNumber,
    /// Stores the transaction's traces.
    TxTraces: (TenantKey<TxNumber>) => TxExecInfo,
    /// Store transaction receipts
    Receipts: (TenantKey<TxNumber>) => Receipt,
    /// Store compiled classes
    CompiledClassHashes: (TenantKey<ClassHash>) => CompiledClassHash,
    /// Store compiled contract classes according to its compiled class hash
    CompiledClasses: (TenantKey<ClassHash>) => CompiledClass,
    /// Store Sierra classes according to its class hash
    SierraClasses: (TenantKey<ClassHash>) => FlattenedSierraClass,
    /// Store contract information according to its contract address
    ContractInfo: (TenantKey<ContractAddress>) => GenericContractInfo,
    /// Store contract storage
    ContractStorage: (TenantKey<ContractAddress>, StorageKey) => StorageEntry,


    /// Stores the block number where the class hash was declared.
    ClassDeclarationBlock: (TenantKey<ClassHash>) => BlockNumber,
    /// Stores the list of class hashes according to the block number it was declared in.
    ClassDeclarations: (TenantKey<BlockNumber>, ClassHash) => ClassHash,

    /// Generic contract info change set.
    ///
    /// Stores the list of blocks where the contract info (nonce / class hash) has changed.
    ContractInfoChangeSet: (TenantKey<ContractAddress>) => ContractInfoChangeList,
    /// Contract nonce changes by block.
    NonceChangeHistory: (TenantKey<BlockNumber>, ContractAddress) => ContractNonceChange,
    /// Contract class hash changes by block.
    ClassChangeHistory: (TenantKey<BlockNumber>, ContractAddress) => ContractClassChange,

    /// storage change set
    StorageChangeSet: (TenantKey<ContractStorageKey>) => BlockList,
    /// Account storage change set
    StorageChangeHistory: (TenantKey<BlockNumber>, ContractStorageKey) => ContractStorageEntry

}
