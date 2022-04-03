#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Coin, Uint128, Reply};
use cw2::set_contract_version;

use crate::error::{ContractError, PaymentError};
use crate::execute::{make_new_deposit, update_deposit, deposit_more, update_user_struct, make_new_user_struct};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{State, STATE, USER_INFO};

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
        swap_contract: msg.swap_contract,
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
        // ExecuteMsg::WithdrawPool {}
        // => withdraw_pool(deps, info),
        // ExecuteMsg::WithdrawCharity {}
        // => withdraw_charity(deps, info),
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
    let depositor = deps.api.addr_validate(&info.sender.as_str())?;
    let swap_address = STATE.load(deps.storage)?.swap_contract;

    if !USER_INFO.has(deps.storage, depositor.as_str()) 
    || USER_INFO.load(deps.storage, depositor.as_str())?.aust_amount == "0" {
        make_new_deposit(swap_address, depositor.to_string(), percentage, ust_sent.u128())
    } else {
        let aust_amount = USER_INFO.load(deps.storage, depositor.as_str())?.aust_amount;

        update_deposit(
            ust_sent, 
            swap_address, 
            depositor.to_string(), 
            percentage, 
            aust_amount
        )
    }
}

pub fn coin(amount: u128, denom: impl Into<String>) -> Coin {
    Coin::new(amount, denom)
}

/// Requires exactly one native coin sent, which matches UUSD.
/// Returns the amount if only one denom and non-zero amount. Errors otherwise.
pub fn check_funds(info: &MessageInfo) -> Result<Uint128, PaymentError> {
    // check if only one coin was sent
    match info.funds.len() {
        0 => Err(PaymentError::NoFunds {}),
        1 => {
            let coin = info.funds[0].clone();
            // check that we rcv'd uusd
            if coin.denom != "uusd" {
                return Err(PaymentError::MissingDenom(coin.denom.to_string()));
            }
            // check amount is gte 0
            if coin.amount.is_zero() {
                return Err(PaymentError::NoFunds {});
            }
            Ok(coin.amount)
        }
        _ => Err(PaymentError::MultipleDenoms {}),
    }
}

#[cfg(test)]
mod tests {}
