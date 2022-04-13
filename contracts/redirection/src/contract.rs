#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Reply, Response};
use cw2::set_contract_version;

use crate::execute::{update_config, deposit_pool, withdraw_pool};
use crate::internal_calls::{
    deposit_initial, deposit_more, swap_back_aust, 
    swap_aust_ust, withdraw_send
};
use crate::replies::{
    make_new_user_struct, deposit_then_update_user, 
    get_new_user_state_dep, get_new_user_state_wit, 
    withdraw_then_update_user
};

use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Config, CONFIG};
use crate::error::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:give";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            admin: deps.api.addr_validate(&msg.admin.to_string())?,
            charity_address: deps.api.addr_validate(&msg.charity_address.to_string())?,
            anchor_market_address: deps.api.addr_validate(&msg.anchor_market_address.to_string())?,
            aust_token_address: deps.api.addr_validate(&msg.aust_token_address.to_string())?,
            theta: msg.theta,
        },
    )?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        0 => make_new_user_struct(deps, env, msg.result),
        1 => get_new_user_state_dep(deps, env, msg.result),
        2 => deposit_then_update_user(deps, env, msg.result),
        3 => get_new_user_state_wit(deps, env, msg.result),
        4 => withdraw_then_update_user(deps, env, msg.result),
        _ => Err(ContractError::Unauthorized {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        /* Three Entry Points */
        ExecuteMsg::UpdateConfig(msg) => update_config(deps, info, msg),
        ExecuteMsg::DepositPool { percentage } => deposit_pool(deps, env, info, percentage),
        ExecuteMsg::WithdrawPool { withdraw_amount } => withdraw_pool(deps, env, info, withdraw_amount),
        /* Three Entry Points */
        
        /* Internal Contract Calls */
        ExecuteMsg::InternalDepositInitial {
            ust_sent,
            percentage,
            depositor,
        } => deposit_initial(deps, env, info, ust_sent, percentage, depositor),
        ExecuteMsg::InternalDepositMore {
            ust_sent,
            aust_amount,
            percentage,
            depositor,
        } => deposit_more(deps, env, info, ust_sent, aust_amount, percentage, depositor),
        ExecuteMsg::InternalSwapBackUpdate {
            to_angel,
            charity_address,
            ust_amount,
            new_percentage,
            depositor,
        } => swap_back_aust(
            deps,
            env,
            info,
            to_angel,
            charity_address,
            ust_amount,
            new_percentage,
            depositor,
        ),
        ExecuteMsg::InternalWithdrawInitial {
            withdraw_amount,
            aust_amount,
            ust_amount,
            percentage,
            depositor,
        } => swap_aust_ust(
            deps,
            env,
            info,
            withdraw_amount,
            aust_amount,
            ust_amount,
            percentage,
            depositor,
        ),
        ExecuteMsg::InternalWithdrawSend {
            withdraw_amount,
            new_ust_amount,
            to_angel_amount,
            ust_depositor,
            charity_address,
        } => withdraw_send(
            deps,
            env,
            info,
            withdraw_amount,
            new_ust_amount,
            to_angel_amount,
            ust_depositor,
            charity_address,
        ),
        /* Internal Contract Calls */
    }
}

#[cfg(test)]
mod tests {}
