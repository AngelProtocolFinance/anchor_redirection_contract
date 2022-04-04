#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Reply, Uint128};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Config, CONFIG, USER_INFO};
use crate::execute::{
    make_new_deposit, update_deposit, make_new_user_struct, 
    check_funds, withdraw_deposit, update_config, get_new_user_state_dep, 
    deposit_then_update_user, get_new_user_state_wit, withdraw_then_update_user, 
    send_dust_to_angel_then_make_new_deposit
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
    let state = Config {
        admin: msg.admin,
        escrow_controller: msg.escrow_controller,
        charity_address: msg.charity_address,
        theta: msg.theta,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) 
-> Result<Response, ContractError> {
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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config }
        => update_config(deps, info, config),
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
    let state = CONFIG.load(deps.storage)?;
    let escrow_controller = state.escrow_controller;
    let no_user = !USER_INFO.has(deps.storage, depositor.as_str());

    if no_user{
        make_new_deposit(
            escrow_controller, 
            depositor.to_string(), 
            percentage, 
            ust_sent.u128()
        )
    } else {
        let user_info = USER_INFO.load(deps.storage, depositor.as_str())?;
        let aust_amount = user_info.aust_amount.parse::<u64>().unwrap();
        if aust_amount == 0 {
            make_new_deposit(
                escrow_controller, 
                depositor.to_string(), 
                percentage, 
                ust_sent.u128()
            )
        } else if aust_amount <= state.theta {
            /*
            * Theta: Should be capped around 0.001 aUST.
            * When a user withdraws, it leaves tiny bits of dust
            * Triggering update deposit over < 0.001 aUST balance is a waste of gas
            * Added to save fees and keep escrow aUST balance as clean as possible.
            */
            send_dust_to_angel_then_make_new_deposit(
                deps,
                escrow_controller, 
                depositor.to_string(), 
                percentage, 
                ust_sent.u128(),
                user_info,
            )
        } else {
            update_deposit(
                ust_sent, 
                escrow_controller, 
                depositor.to_string(), 
                percentage, 
                user_info.aust_amount
            )
        }
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
