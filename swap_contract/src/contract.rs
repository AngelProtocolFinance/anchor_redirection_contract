#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, Coin, Uint128, WasmMsg, coin, Reply, BankMsg};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, AnchorExecuteMsg, InstantiateMsg, Cw20HookMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:aust-swapper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DepositInitial { percentage, depositor } => deposit_initial(info, percentage, depositor),
        ExecuteMsg::DepositMore { ust_sent, aust_amount, percentage, depositor } 
        => deposit_more(ust_sent, aust_amount, percentage, depositor),
        ExecuteMsg::SwapBackUpdate { to_angel, charity_address, ust_amount, new_percentage, depositor }
        => swap_back_aust(to_angel, charity_address, ust_amount, new_percentage, depositor)
    }
}

fn deposit_initial(
    info: MessageInfo,
    percentage: u16,
    depositor: String,
) -> Result<Response, ContractError> {
    let ust_sent = must_pay(&info, "uusd")?;

    let deposit_stable = AnchorExecuteMsg::DepositStable {};
    let anchor_deposit = WasmMsg::Execute {
        contract_addr: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
        msg: to_binary(&deposit_stable)?,
        funds: vec![coin(ust_sent.u128(), "uusd")],
    };
    
    Ok(Response::new()
        .add_attribute("percentage", percentage.to_string())
        .add_attribute("ust_depositor", depositor)
        .add_message(anchor_deposit)
    )
}

fn deposit_more(
    ust_sent: Uint128,
    aust_amount: String,
    percentage: u16,
    depositor: String,
) -> Result<Response, ContractError> {
    let convert_to_ust = WasmMsg::Execute {
        contract_addr: String::from("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
            msg: to_binary(&Cw20HookMsg::RedeemStable{}).unwrap(),
            amount: Uint128::new(aust_amount.parse::<u128>().unwrap())
        }).unwrap(),
        funds: Vec::new()
    };

    Ok(Response::new()
        .add_attribute("ust_sent", ust_sent)
        .add_attribute("percentage", percentage.to_string())
        .add_attribute("ust_depositor", depositor)
        .add_message(convert_to_ust)
    )
}

pub fn swap_back_aust(
    to_angel: u64, 
    charity_address: String,
    ust_amount: u64, 
    new_percentage: u64, 
    depositor: String,
) -> Result<Response, ContractError> {
    let send_to_charity = BankMsg::Send { 
        to_address: charity_address, 
        amount: vec![coin(to_angel.into(), "uusd")]
    };
    let deposit_stable = AnchorExecuteMsg::DepositStable {};
    let anchor_deposit = WasmMsg::Execute {
        contract_addr: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
        msg: to_binary(&deposit_stable)?,
        funds: vec![coin(ust_amount.into(), "uusd")],
    };
    
    Ok(Response::new()
        .add_attribute("new_percentage", new_percentage.to_string())
        .add_attribute("ust_depositor", depositor)
        .add_attribute("ust_amount", ust_amount.to_string())
        .add_message(send_to_charity)
        .add_message(anchor_deposit)
    )
}

/// Requires exactly one denom sent, which matches the requested denom.
/// Returns the amount if only one denom and non-zero amount. Errors otherwise.
pub fn must_pay(info: &MessageInfo, denom: &str) -> Result<Uint128, ContractError> {
    let coin = one_coin(info)?;
    if coin.denom != denom {
        Err(ContractError::MissingDenom(denom.to_string()))
    } else {
        Ok(coin.amount)
    }
}

pub fn convert_str_int(str: String)
    ->u128
{
    let bytes = str.into_bytes();
    let mut res: u128 = 0;
    let mut dot = false;
    let mut dotbelow = 0;

    for i in 0..bytes.len(){
        if bytes[i] < 48{
            dot = true;
        }
        else if dotbelow < 6 {
            res = res * 10 + (bytes[i] - 48) as u128;
            if dot {
                dotbelow += 1;
            }
        }
    }
    return res;
}

/// If exactly one coin was sent, returns it regardless of denom.
/// Returns error if 0 or 2+ coins were sent
pub fn one_coin(info: &MessageInfo) -> Result<Coin, ContractError> {
    match info.funds.len() {
        0 => Err(ContractError::NoFunds {}),
        1 => {
            let coin = &info.funds[0];
            if coin.amount.is_zero() {
                Err(ContractError::NoFunds {})
            } else {
                Ok(coin.clone())
            }
        }
        _ => Err(ContractError::MultipleDenoms {}),
    }
}

#[cfg(test)]
mod tests {}