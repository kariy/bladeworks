use std::collections::HashMap;

use bladeworks_db::Db;
use katana_db::mdbx;
use katana_primitives::contract::{StorageKey, StorageValue};

struct StateManager {
    db: Db,
}

type Version = u64;

struct State {
    inner: HashMap<(StorageKey, Version), StorageValue>,
}
