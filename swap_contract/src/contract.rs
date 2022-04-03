#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg, coin, BankMsg};
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
        ExecuteMsg::DepositInitial { 
            percentage, 
            depositor 
        } => deposit_initial(info, percentage, depositor),
        ExecuteMsg::DepositMore { 
            ust_sent, 
            aust_amount, 
            percentage, 
            depositor 
        } => deposit_more(ust_sent, aust_amount, percentage, depositor),
        ExecuteMsg::SwapBackUpdate { 
            to_angel, 
            charity_address, 
            ust_amount, 
            new_percentage, 
            depositor 
        } => swap_back_aust(to_angel, charity_address, ust_amount, new_percentage, depositor),
        ExecuteMsg::WithdrawInitial { 
            withdraw_amount, 
            aust_amount, 
            ust_amount, 
            percentage, 
            depositor 
        } => swap_aust_ust( withdraw_amount, aust_amount, ust_amount, percentage, depositor ),
        ExecuteMsg::WithdrawSend { 
            withdraw_amount, 
            new_ust_amount, 
            to_angel_amount,
            ust_depositor,
            charity_address
        } => withdraw_send(withdraw_amount, new_ust_amount, to_angel_amount, ust_depositor, charity_address)
    }
}

fn withdraw_send(
    withdraw_amount: u64,
    new_ust_amount: u64,
    to_angel_amount: u64,
    ust_depositor: String,
    charity_address: String,
) -> Result<Response, ContractError> {
    let withdraw_to_user = BankMsg::Send { 
        to_address: ust_depositor.clone(), 
        amount: vec![coin(withdraw_amount.into(), "uusd")]
    };
    let send_to_charity = BankMsg::Send { 
        to_address: charity_address, 
        amount: vec![coin(to_angel_amount.into(), "uusd")]
    };
    let deposit_stable = AnchorExecuteMsg::DepositStable {};
    let anchor_deposit = WasmMsg::Execute {
        contract_addr: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
        msg: to_binary(&deposit_stable)?,
        funds: vec![coin(new_ust_amount.into(), "uusd")],
    };

    if to_angel_amount == 0 {
        Ok(Response::new()
            .add_attribute("ust_depositor", ust_depositor)
            .add_message(withdraw_to_user)
            .add_message(anchor_deposit)
        )
    } else {
        Ok(Response::new()
            .add_attribute("ust_depositor", ust_depositor)
            .add_message(withdraw_to_user)
            .add_message(send_to_charity)
            .add_message(anchor_deposit)
        )
    }
}

fn swap_aust_ust(
    withdraw_amount: Uint128,
    aust_amount: String,
    ust_amount: String, 
    percentage: String,
    depositor: String, 
) -> Result<Response, ContractError> {
    let convert_to_ust = get_convert_to_ust(aust_amount.clone());

    Ok(Response::new()
        .add_attribute("withdraw_amount", withdraw_amount)
        .add_attribute("percentage", percentage)
        .add_attribute("ust_depositor", depositor)
        .add_attribute("ust_amount", ust_amount)
        .add_attribute("aust_amount", aust_amount)
        .add_message(convert_to_ust)
    )
}

fn deposit_initial(
    info: MessageInfo,
    percentage: u16,
    depositor: String,
) -> Result<Response, ContractError> {
    let ust_sent = check_funds(&info)?;

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
    let convert_to_ust = get_convert_to_ust(aust_amount);

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

    if to_angel == 0 {
        Ok(Response::new()
            .add_attribute("new_percentage", new_percentage.to_string())
            .add_attribute("ust_depositor", depositor)
            .add_message(anchor_deposit)
        )
    } else {
        Ok(Response::new()
            .add_attribute("new_percentage", new_percentage.to_string())
            .add_attribute("ust_depositor", depositor)
            .add_message(send_to_charity)
            .add_message(anchor_deposit)
        )
    }
}

//Helpers
fn get_convert_to_ust(aust_amount: String) -> WasmMsg {
    return WasmMsg::Execute {
        contract_addr: String::from("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
            msg: to_binary(&Cw20HookMsg::RedeemStable{}).unwrap(),
            amount: Uint128::new(aust_amount.parse::<u128>().unwrap())
        }).unwrap(),
        funds: Vec::new()
    }
}
/// Requires exactly one native coin sent, which matches UUSD.
/// Returns the amount if only one denom and non-zero amount. Errors otherwise.
pub fn check_funds(info: &MessageInfo) -> Result<Uint128, ContractError> {
    // check if only one coin was sent
    match info.funds.len() {
        0 => Err(ContractError::NoFunds {}),
        1 => {
            let coin = info.funds[0].clone();
            // check that we rcv'd uusd
            if coin.denom != "uusd" {
                return Err(ContractError::MissingDenom(coin.denom.to_string()));
            }
            // check amount is gte 0
            if coin.amount.is_zero() {
                return Err(ContractError::NoFunds {});
            }
            Ok(coin.amount)
        }
        _ => Err(ContractError::MultipleDenoms {}),
    }
}

#[cfg(test)]
mod tests {}