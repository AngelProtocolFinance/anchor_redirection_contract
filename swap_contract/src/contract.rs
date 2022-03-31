#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, Coin, Uint128, WasmMsg, coin, Reply, SubMsgExecutionResponse, ContractResult};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, AnchorExecuteMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:aust-swapper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) 
-> Result<Response, ContractError> {
    match msg.id {
        0 => send_aust(deps, env, msg.result),
        _ => Err(ContractError::NotSwap {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Swap { percentage } => swap(info, percentage),
    }
}

fn swap(
    info: MessageInfo,
    percentage: u16,
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
    .add_message(anchor_deposit))
}

fn send_aust(
    _deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
        match msg {
        ContractResult::Ok(subcall) => {
            let mut mint_amount = String::from("");
            let mut depositor = String::from("");
            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "mint_amount" {
                        mint_amount = attrb.value;
                    } else if attrb.key == "depositor" {
                        depositor = attrb.value;
                    }
                }
            }

            let send_aust = WasmMsg::Execute {
                contract_addr: String::from("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: depositor,
                    amount: mint_amount.parse::<Uint128>().unwrap(),
                }).unwrap(),
                funds: Vec::new(),
            };
            
            Ok(Response::new().add_message(send_aust))
        }
        ContractResult::Err(_) => Err(ContractError::NotSwap {}),
    }
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