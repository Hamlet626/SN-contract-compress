use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, CanonicalAddr, from_binary, HumanAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton, PrefixedStorage, ReadonlyPrefixedStorage};
use secret_toolkit::serialization::{Json, Serde};

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

impl StoreNftInfo {
    //msg_bytes:"price  owner  ..."
    // e.g. "1000 secret19kl6c3lml882eyzagf6z0sh7pvsj8tndcfus3k  other info"
    pub fn from(msg_bytes:Binary) ->StdResult<StoreNftInfo>{
        let msg=String::from_utf8(msg_bytes.into())
            .or_else(|_e|Err(StdError::serialize_err("StoreNftInfo","invalid binary")))?;
        let mut r =msg.split_whitespace();
        let price=r.next().ok_or_else(||StdError::serialize_err("StoreNftInfo","no price provided"))?.parse::<u128>()
            .or_else(|_e|Err(StdError::serialize_err("StoreNftInfo","invalid price")))?;
        Ok(StoreNftInfo{
            owner: HumanAddr::from(r.next().ok_or_else(||StdError::serialize_err("StoreNftInfo","no owner provided"))?),
            price
        })
    }
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn store_set<S: Storage>(storage: &mut S,token_id:&String,info: &StoreNftInfo) -> StdResult<()> {
    PrefixedStorage::new(STORE_KEY, storage).set(token_id.as_bytes(),&Json::serialize(info)?);
    Ok(())
}
pub fn store_remove<S: Storage>(storage: &mut S,token_id:&String){
    PrefixedStorage::new(STORE_KEY, storage).remove(token_id.as_bytes());
}

pub fn store_read<S: Storage>(storage: &S,tokenid:&String) -> StdResult<StoreNftInfo> {
    Json::deserialize(
        &ReadonlyPrefixedStorage::new(STORE_KEY, storage)
            .get(tokenid.as_bytes())
            .ok_or_else(|| StdError::not_found(tokenid))?,
    )
}
