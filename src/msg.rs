use cosmwasm_std::{Binary, HumanAddr};
use schemars::JsonSchema;
use secret_toolkit::permit::Permit;
use secret_toolkit::snip721::{Expiration, Metadata, NftDossier, Snip721Approval};
use serde::{Deserialize, Serialize};
use crate::state::StoreNftInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub ed_ctr: HumanAddr,
    pub ed_code_hash: String,
    pub ip_ctr: HumanAddr,
    pub ip_code_hash: String,
    pub view_key: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ReceiveNft {
        sender: HumanAddr ,
        token_id: String,
        msg: Option<Binary>,
    },
    Reset {
        view_key: String},
    Transfer {
        token_id:String,
        receipient: Option<HumanAddr>}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetConfig {permit:Option<Permit>},
    ViewNft {
        token_id: String,
        permit:Option<Permit>}
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub ed_nft_contract: HumanAddr,
    pub ed_code_hash: String,
    pub ip_nft_contract: HumanAddr,
    pub ip_code_hash: String,
    pub owner: HumanAddr,
    pub view_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftResponse {
    pub dossier: NftDossier,
    pub store_info: String
}
