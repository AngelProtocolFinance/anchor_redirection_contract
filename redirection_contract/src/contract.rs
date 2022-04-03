#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Reply, Uint128};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{State, STATE, USER_INFO};
use crate::execute::{
    make_new_deposit, update_deposit, deposit_more, 
    update_user_struct, make_new_user_struct, check_funds, withdraw_deposit, get_new_user_state, update_user_struct_after_withdraw
};

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
    let state = State {
        escrow_controller: msg.escrow_controller,
        charity_address: msg.charity_address,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        0 => make_new_user_struct(deps, env, msg.result),
        1 => deposit_more(deps, env, msg.result),
        2 => update_user_struct(deps, env, msg.result),
        3 => get_new_user_state(deps, env, msg.result),
        4 => update_user_struct_after_withdraw(deps, env, msg.result),
        _ => Err(ContractError::Unauthorized {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DepositPool { percentage,  } 
        => deposit_pool(deps, info, percentage),
        ExecuteMsg::WithdrawPool { withdraw_amount }
        => withdraw_pool(deps, info, withdraw_amount),
    }
}

pub fn deposit_pool(
    deps: DepsMut,
    info: MessageInfo,
    percentage: u16,
) -> Result<Response, ContractError> {
    if percentage < 5 || percentage > 100 {
        return Err(ContractError::WrongPercentageInput {});
    };

    let ust_sent = check_funds(&info)?;

    if ust_sent.u128() < 1000 {
        return Err(ContractError::MakeNewPoolError {})
    };

    let depositor = deps.api.addr_validate(&info.sender.as_str())?;
    let escrow_controller = STATE.load(deps.storage)?.escrow_controller;

    if !USER_INFO.has(deps.storage, depositor.as_str()) 
    || USER_INFO.load(deps.storage, depositor.as_str())?.aust_amount.parse::<u64>().unwrap() <= 1 {
        make_new_deposit(escrow_controller, depositor.to_string(), percentage, ust_sent.u128())
    } else {
        let aust_amount = USER_INFO.load(deps.storage, depositor.as_str())?.aust_amount;

        update_deposit(
            ust_sent, 
            escrow_controller, 
            depositor.to_string(), 
            percentage, 
            aust_amount
        )
    }
}

pub fn withdraw_pool(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let depositor = deps.api.addr_validate(&info.sender.as_str())?;

    if !USER_INFO.has(deps.storage, depositor.as_str())  {
        return Err(ContractError::NoDeposit {})
    } 

    withdraw_deposit(
        deps,
        amount,
        depositor.to_string()
    )
}

#[cfg(test)]
mod tests {}
