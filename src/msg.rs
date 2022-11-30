use cosmwasm_std::{Binary, HumanAddr};
use schemars::JsonSchema;
use secret_toolkit::permit::Permit;
use secret_toolkit::snip721::{Expiration, Metadata, NftDossier, Snip721Approval};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub ed_ctr: HumanAddr,
    pub ed_code_hash: String,
    pub ip_ctr: HumanAddr,
    pub ip_code_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ReceiveNft {
        sender: HumanAddr ,
        token_id: String,
        msg: Option<Binary>,
    },
    SetUp {
        view_key: String,
        permit: Permit},
    Reset { count: i32 },
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
    /// owner of the token if permitted to view it
    pub owner: Option<HumanAddr>,
    /// the token's public metadata
    pub public_metadata: Option<Metadata>,
    /// the token's private metadata if permitted to view it
    pub private_metadata: Option<Metadata>,
    /// description of why private metadata is not displayed (if applicable)
    pub display_private_metadata_error: Option<String>,
    /// true if the owner is publicly viewable
    pub owner_is_public: bool,
    /// expiration of public display of ownership (if applicable)
    pub public_ownership_expiration: Option<Expiration>,
    /// true if private metadata is publicly viewable
    pub private_metadata_is_public: bool,
    /// expiration of public display of private metadata (if applicable)
    pub private_metadata_is_public_expiration: Option<Expiration>,
    /// approvals for this token (only viewable if queried by the owner)
    pub token_approvals: Option<Vec<Snip721Approval>>,
    /// approvals that apply to this token because they apply to all of
    /// the owner's tokens (only viewable if queried by the owner)
    pub inventory_approvals: Option<Vec<Snip721Approval>>,
}

impl From<NftDossier> for NftResponse {
    fn from(r: NftDossier) -> Self {
        NftResponse{
            owner: r.owner,
            public_metadata: r.public_metadata.clone(),
            private_metadata: r.private_metadata.clone(),
            display_private_metadata_error: r.display_private_metadata_error,
            owner_is_public: r.owner_is_public,
            public_ownership_expiration: r.public_ownership_expiration,
            private_metadata_is_public: r.private_metadata_is_public,
            private_metadata_is_public_expiration: r.private_metadata_is_public_expiration,
            token_approvals: r.token_approvals,
            inventory_approvals: r.inventory_approvals
        }
    }
}
