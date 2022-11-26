use std::borrow::Borrow;
use std::cell::RefCell;
use std::slice::Iter;
use std::vec::IntoIter;
use cosmwasm_std::{debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError, StdResult, Storage, HumanAddr};
use secret_toolkit::permit::{Permit, validate};
use secret_toolkit::snip721::{AccessLevel, Metadata, nft_dossier_query, NftDossier, register_receive_nft_msg, set_whitelisted_approval_msg, tokens_query, Trait};

use crate::msg::{ConfigResponse, HandleMsg, InitMsg, NftResponse, QueryMsg};
use crate::state::{config, config_read, PREFIX_PERMITS, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        ed_nft_contract: deps.api.canonical_address(&msg.ed_ctr)?,
        ed_code_hash: msg.ed_code_hash.to_string(),
        ip_nft_contract: deps.api.canonical_address(&msg.ip_ctr)?,
        ip_code_hash: msg.ip_code_hash,
        owner: deps.api.canonical_address(&env.message.sender)?,
        contract_addr: env.contract.address.clone()
    };

    register_receive_nft_msg(msg.ed_code_hash, None,
                             None, 0, env.contract_code_hash, env.contract.address)?;


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
        HandleMsg::Increment {} => try_increment(deps, env),
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
                                 Option::from(AccessLevel::ApproveToken), None, None, None, 0,
                                 config.ed_code_hash, deps.api.human_address(&config.ed_nft_contract)?)
        .map(|m| HandleResponse{
            messages: vec![m],
            log: vec![],
            data: None })
}

pub fn try_increment<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
) -> StdResult<HandleResponse> {
    config(&mut deps.storage).update(|mut state| {
        // state.count += 1;
        // debug_print!("count = {}", state.count);
        Ok(state)
    })?;

    debug_print("count incremented successfully");
    Ok(HandleResponse::default())
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
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn check_view_nft<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>,tokenid:String,permit:Option<Permit>)->StdResult<NftDossier>{
    let state=&config_read(&deps.storage).load()?;

    let ednft=&nft_dossier_query(&deps.querier, tokenid, None,
                                Option::Some(true), 0,
                                state.ed_code_hash.to_owned(),
                                deps.api.human_address(&state.ed_nft_contract)?)?;

    if permit.is_some() {
        let sender=HumanAddr(validate(deps, PREFIX_PERMITS, &permit.unwrap(), state.contract_addr.to_owned(), None)?);
        let ipnfts=tokens_query(&deps.querier, sender, None,
                               None, None, Option::Some(100),0,
                               state.ip_code_hash.to_owned(),
                               deps.api.human_address(&state.ip_nft_contract)?)?;

        let ed_traits =findTrait(ednft.to_owned().public_metadata).unwrap_or_else(||vec![].into_iter()).find(
                                 |tr| tr.trait_type.is_some()&&"agc"==tr.trait_type.as_ref().unwrap());

        let ed_agc =&ed_traits.unwrap().value;
        let ip_contr_addr =&deps.api.human_address(&state.ip_nft_contract)?;

        let view=ipnfts.tokens.iter().find(|&ipnft|{
            let detail=nft_dossier_query(&deps.querier, String::from(ipnft), None,
                                         Option::Some(true), 0,
                                         state.ip_code_hash.to_owned(),
                                         ip_contr_addr.to_owned());
            if detail.is_err(){false}
            else{
                let data=detail.unwrap().public_metadata;
                findTrait(data).unwrap_or_else(||vec![].into_iter()).find(
                    |t| t.trait_type.is_some()&&t.trait_type.as_ref().unwrap()==ed_agc).is_some()
            }
        }).is_some();
    }

    //todo:verify by view
    Ok(ednft.to_owned()
       //     NftDossier {
       //     owner: None,
       //     public_metadata: None,
       //     private_metadata: None,
       //     display_private_metadata_error: None,
       //     owner_is_public: false,
       //     public_ownership_expiration: None,
       //     private_metadata_is_public: false,
       //     private_metadata_is_public_expiration: None,
       //     token_approvals: None,
       //     inventory_approvals: None
       // }
    )
}

fn findTrait(metadata:Option<Metadata>) ->Option<IntoIter<Trait>>{
    Some(metadata?.extension?.attributes?.into_iter())
}

fn query_config<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<ConfigResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(ConfigResponse {
        ed_nft_contract: deps.api.human_address(&state.ed_nft_contract)?,
        ed_code_hash: state.ed_code_hash,
        ip_nft_contract: deps.api.human_address(&state.ip_nft_contract)?,
        ip_code_hash: state.ip_code_hash,
        owner: deps.api.human_address(&state.owner)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    static ipCAddr: &str ="secret";
    static ipCHash: &str ="7be15101bd6dc6c991213f6b108c8626a1feb63312f8622cbe3e2243305a27bd";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            ed_ctr: HumanAddr(String::from(ipCAddr)),
            ed_code_hash: String::from(ipCHash),
            ip_ctr: HumanAddr(String::from(ipCAddr)),
            ip_code_hash: String::from(ipCHash)
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        println!("{}",env.message.sender);
        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetConfig {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        println!("{}", value.ip_code_hash);
        println!("{}", value.ip_nft_contract);
        assert_eq!(HumanAddr(String::from("creator")), value.owner);
        assert_eq!(ipCHash, value.ed_code_hash);
    }

    // #[test]
    // fn increment() {
    //     let mut deps = mock_dependencies(20, &coins(2, "token"));
    //
    //     let msg = InitMsg { count: 17 };
    //     let env = mock_env("creator", &coins(2, "token"));
    //     let _res = init(&mut deps, env, msg).unwrap();
    //
    //     // anyone can increment
    //     let env = mock_env("anyone", &coins(2, "token"));
    //     let msg = HandleMsg::Increment {};
    //     let _res = handle(&mut deps, env, msg).unwrap();
    //
    //     // should increase counter by 1
    //     let res = query(&deps, QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(18, value.count);
    // }
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
