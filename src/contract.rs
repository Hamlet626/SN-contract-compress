use std::ops::Add;
use std::vec::IntoIter;
use cosmwasm_std::{debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError, StdResult, Storage, HumanAddr, CosmosMsg};
use secret_toolkit::permit::{Permit, validate};
use secret_toolkit::snip721::{AccessLevel, Metadata, nft_dossier_query, NftDossier, register_receive_nft_msg, set_viewing_key_msg, set_whitelisted_approval_msg, tokens_query, Trait, ViewerInfo};
use snafu::{Backtrace, GenerateBacktrace};

use crate::msg::{ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, PREFIX_PERMITS, State, SUFFIX_ED_KEY, SUFFIX_IP_KEY};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        ed_nft_contract: deps.api.canonical_address(&msg.ed_ctr)?,
        ed_code_hash: msg.ed_code_hash,
        ip_nft_contract: deps.api.canonical_address(&msg.ip_ctr)?,
        ip_code_hash: msg.ip_code_hash,
        owner: deps.api.canonical_address(&env.message.sender)?,
        contract_addr: env.contract.address.clone(),
        viewing_key: None
    };


    config(&mut deps.storage).save(&state)?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::ReceiveNft { sender,token_id,msg } =>
            set_sender_auth(deps, sender, token_id, msg),
        HandleMsg::SetUp { view_key,permit } => set_up(deps, env,view_key,permit),
        HandleMsg::Reset { count } => try_reset(deps, env, count),
    }
}

pub fn set_sender_auth<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: HumanAddr ,
    token_id: String,
    msg: Option<Binary>, )->StdResult<HandleResponse>{
    let config=config_read(&deps.storage).load()?;
    set_whitelisted_approval_msg(sender, Option::from(token_id),
                                 Option::from(AccessLevel::ApproveToken),
                                 Option::from(AccessLevel::ApproveToken), None, None, None, 256,
                                 config.ed_code_hash, deps.api.human_address(&config.ed_nft_contract)?)
        .map(|m| HandleResponse{
            messages: vec![m],
            log: vec![],
            data: None })
}

pub fn set_up<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    vkey:String,
    permit:Permit
) -> StdResult<HandleResponse> {
    let sender=HumanAddr(validate(&deps, PREFIX_PERMITS, &permit, env.contract.address.clone(), None)?);
    let api=&deps.api.clone();
    let mut res_msg:Vec<CosmosMsg>=vec![];
    config(&mut deps.storage).update(|mut state| {
        if sender!=api.human_address(&state.owner)? { Err(StdError::Unauthorized { backtrace: Some(Backtrace::generate()) }) }
        else {
            res_msg=vec![set_viewing_key_msg(vkey.clone().add(SUFFIX_IP_KEY), None, 256,
                                state.ip_code_hash.to_owned(), api.human_address(&state.ip_nft_contract)?)?,
            set_viewing_key_msg(vkey.clone().add(SUFFIX_ED_KEY), None, 256,
                                state.ed_code_hash.to_owned(), api.human_address(&state.ed_nft_contract)?)?,
            register_receive_nft_msg(env.contract_code_hash, None, None,
                                     256, state.ed_code_hash.to_owned(), api.human_address(&state.ed_nft_contract)?)?];
            state.viewing_key=Some(vkey);
            Ok(state) }
    })?;

    debug_print("count incremented successfully");
    Ok(HandleResponse{
        messages: res_msg,
        log: vec![],
        data: None
    })
}

pub fn try_reset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    count: i32,
) -> StdResult<HandleResponse> {
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    config(&mut deps.storage).update(|mut state| {
        if sender_address_raw != state.owner {
            return Err(StdError::Unauthorized { backtrace: None });
        }
        // state.count = count;
        Ok(state)
    })?;
    debug_print("count reset successfully");
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::ViewNft {token_id,permit}=>to_binary(&check_view_nft(deps,token_id,permit)?),
        QueryMsg::GetConfig {permit} => to_binary(&query_config(deps,permit)?),
    }
}

fn check_view_nft<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>,tokenid:String,permit:Option<Permit>)->StdResult<NftDossier>{
    let state=&config_read(&deps.storage).load()?;
    let vkey=state.viewing_key.clone().unwrap();
    let ed_viewer =Some(ViewerInfo{ address: state.contract_addr.to_owned(),
        viewing_key: vkey.clone().add(SUFFIX_ED_KEY) });
    let ednft=&nft_dossier_query(&deps.querier, tokenid, ed_viewer,
                                 Some(true), 256,
                                 state.ed_code_hash.to_owned(),
                                 deps.api.human_address(&state.ed_nft_contract)?)?;

    if permit.is_some() {
        let sender=HumanAddr(validate(deps, PREFIX_PERMITS, &permit.unwrap(), state.contract_addr.to_owned(), None)?);
        let ip_viewer =Some(ViewerInfo{ address: state.contract_addr.to_owned(),
            viewing_key: vkey.clone().add(SUFFIX_IP_KEY) });
        let ipnfts=tokens_query(&deps.querier, sender, Some(state.contract_addr.clone()),
                               Some(ip_viewer.to_owned().unwrap().viewing_key),
                                None, Option::Some(100),256,
                               state.ip_code_hash.to_owned(),
                               deps.api.human_address(&state.ip_nft_contract)?)?;

        let ed_traits = find_trait(ednft.to_owned().public_metadata).unwrap_or_else(||vec![].into_iter()).find(
                                 |tr| tr.trait_type.is_some()&&"agc"==tr.trait_type.as_ref().unwrap());

        //todo:uncomment unwrap().. below,
        // currently will panic since ipNft haven't standard "agc" trait, later should have
        let ed_agc =&String::from("test");//&ed_traits.unwrap().value;

        let ip_contr_addr =&deps.api.human_address(&state.ip_nft_contract)?;

        let view=ipnfts.tokens.iter().find(|&ipnft|{
            let detail=nft_dossier_query(&deps.querier, String::from(ipnft), ip_viewer.to_owned(),
                                         Option::Some(true), 256,
                                         state.ip_code_hash.to_owned(),
                                         ip_contr_addr.to_owned());

            if detail.is_err(){false}
            else{
                let data=detail.unwrap().public_metadata;
                find_trait(data).unwrap_or_else(||vec![].into_iter()).find(
                    |t| t.trait_type.is_some()&&t.trait_type.as_ref().unwrap()==ed_agc).is_some()
            }
        }).is_some();

    }

    //todo:verify by view
    // let r=ednft.to_owned();
    Ok(ednft.to_owned())
}

fn find_trait(metadata:Option<Metadata>) ->Option<IntoIter<Trait>>{
    Some(metadata?.extension?.attributes?.into_iter())
}

fn query_config<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>,permit:Option<Permit>) -> StdResult<ConfigResponse> {
    let state = config_read(&deps.storage).load()?;
    let mut r=ConfigResponse {
        ed_nft_contract: deps.api.human_address(&state.ed_nft_contract)?,
        ed_code_hash: state.ed_code_hash,
        ip_nft_contract: deps.api.human_address(&state.ip_nft_contract)?,
        ip_code_hash: state.ip_code_hash,
        owner: deps.api.human_address(&state.owner)?,
        view_key: None
    };
    if permit.is_some() {
        let sender=HumanAddr(validate(deps, PREFIX_PERMITS, &permit.unwrap(), state.contract_addr.to_owned(), None)?);
        if sender==r.owner { r.view_key= state.viewing_key; }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};
    use secret_toolkit::permit::{PermitParams, PermitSignature, PubKey, SignedPermit, TokenPermissions};

    static ipCAddr: &str ="secret";
    static ipCHash: &str ="7be15101bd6dc6c991213f6b108c8626a1feb63312f8622cbe3e2243305a27bd";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            ed_ctr: HumanAddr(String::from(ipCAddr)),
            ed_code_hash: String::from(ipCHash),
            ip_ctr: HumanAddr(String::from(ipCAddr)),
            ip_code_hash: String::from(ipCHash),
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        println!("{}",env.message.sender);
        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetConfig { permit: None }).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        println!("{}", value.ip_code_hash);
        println!("{}", value.ip_nft_contract);
        assert_eq!(HumanAddr(String::from("creator")), value.owner);
        assert_eq!(ipCHash, value.ed_code_hash);
    }

    #[test]
    fn view() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {
            ed_ctr: HumanAddr(String::from(ipCAddr)),
            ed_code_hash: String::from(ipCHash),
            ip_ctr: HumanAddr(String::from(ipCAddr)),
            ip_code_hash: String::from(ipCHash),
        };
        let env = mock_env("creator", &coins(1000, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let token = HumanAddr("secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq".to_string());
        let permit: Permit = Permit{
            params: PermitParams {
                allowed_tokens: vec![token.clone()],
                permit_name: "memo_secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq".to_string(),
                chain_id: "pulsar-2".to_string(),
                permissions: vec![TokenPermissions::History]
            },
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("A5M49l32ZrV+SDsPnoRv8fH7ivNC4gEX9prvd4RwvRaL").unwrap(),
                },
                signature: Binary::from_base64("hw/Mo3ZZYu1pEiDdymElFkuCuJzg9soDHw+4DxK7cL9rafiyykh7VynS+guotRAKXhfYMwCiyWmiznc6R+UlsQ==").unwrap()
            }
        };
        let env = mock_env("anyone", &coins(2, "token"));

        let msg = QueryMsg::ViewNft { token_id: "0".to_string(), permit: Option::from(permit) };
        let res = query(&mut deps, msg).unwrap();

        let value: NftDossier = from_binary(&res).unwrap();
    }
    //
    // #[test]
    // fn reset() {
    //     let mut deps = mock_dependencies(20, &coins(2, "token"));
    //
    //     let msg = InitMsg { count: 17 };
    //     let env = mock_env("creator", &coins(2, "token"));
    //     let _res = init(&mut deps, env, msg).unwrap();
    //
    //     // not anyone can reset
    //     let unauth_env = mock_env("anyone", &coins(2, "token"));
    //     let msg = HandleMsg::Reset { count: 5 };
    //     let res = handle(&mut deps, unauth_env, msg);
    //     match res {
    //         Err(StdError::Unauthorized { .. }) => {}
    //         _ => panic!("Must return unauthorized error"),
    //     }
    //
    //     // only the original creator can reset the counter
    //     let auth_env = mock_env("creator", &coins(2, "token"));
    //     let msg = HandleMsg::Reset { count: 5 };
    //     let _res = handle(&mut deps, auth_env, msg).unwrap();
    //
    //     // should now be 5
    //     let res = query(&deps, QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(5, value.count);
    // }
}
