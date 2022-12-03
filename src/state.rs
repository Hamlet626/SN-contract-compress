use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, CanonicalAddr, from_binary, HumanAddr, ReadonlyStorage, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton, PrefixedStorage, ReadonlyPrefixedStorage};

pub static CONFIG_KEY: &[u8] = b"config";
pub static STORE_KEY: &[u8] = b"store";

pub const PREFIX_PERMITS: &str = "revoke";
pub const SUFFIX_ED_KEY: &str = "edkk";
pub const SUFFIX_IP_KEY: &str = "edkk"; //todo:change to ip's key
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub ed_nft_contract: CanonicalAddr,
    pub ed_code_hash: String,
    pub ip_nft_contract: CanonicalAddr,
    pub ip_code_hash: String,
    pub contract_addr: HumanAddr,
    pub viewing_key: String,
    pub owner: CanonicalAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StoreNftInfo {
    pub owner: HumanAddr,
    pub price: u128,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn store<S: Storage>(storage: &mut S) -> PrefixedStorage<S> {
    PrefixedStorage::new(STORE_KEY, storage)
}

pub fn store_read<S: Storage>(storage: &S,tokenid:&String) -> StdResult<String> {
    let d=ReadonlyPrefixedStorage::new(STORE_KEY, storage).get(tokenid.as_bytes()).unwrap_or_default();
    let r=from_binary(&Binary::from(d))?;
    r
}
