use cosmwasm_std::{MessageInfo, Response, WasmMsg, to_binary, coin, Uint128, BankMsg, DepsMut};
use cw20::Cw20ExecuteMsg;
use crate::{ContractError, msg::{AnchorExecuteMsg, Cw20HookMsg}, state::CONFIG};

pub fn update_config(
    deps: DepsMut,
    redirection_contract: String
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?.clone();
    config.redirection_contract = redirection_contract;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

pub fn deposit_initial(
    deps: DepsMut,
    info: MessageInfo,
    percentage: u16,
    depositor: String,
) -> Result<Response, ContractError> {
    let redirection_contract = CONFIG.load(deps.storage)?.redirection_contract;
    let sender = deps.api.addr_validate(&info.sender.to_string())?.to_string();
    if sender != redirection_contract {
        return Err(ContractError::Unauthorized{})
    };

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

pub fn deposit_more(
    deps: DepsMut,
    info: MessageInfo,
    ust_sent: Uint128,
    aust_amount: String,
    percentage: u16,
    depositor: String,
) -> Result<Response, ContractError> {
    let redirection_contract = CONFIG.load(deps.storage)?.redirection_contract;
    let sender = deps.api.addr_validate(&info.sender.to_string())?.to_string();
    if sender != redirection_contract {
        return Err(ContractError::Unauthorized{})
    };
    let convert_to_ust = get_convert_to_ust(aust_amount);

    Ok(Response::new()
        .add_attribute("ust_sent", ust_sent)
        .add_attribute("percentage", percentage.to_string())
        .add_attribute("ust_depositor", depositor)
        .add_message(convert_to_ust)
    )
}

pub fn swap_back_aust(
    deps: DepsMut,
    info: MessageInfo,
    to_angel: u64, 
    charity_address: String,
    ust_amount: u64, 
    new_percentage: u64, 
    depositor: String,
) -> Result<Response, ContractError> {
    let redirection_contract = CONFIG.load(deps.storage)?.redirection_contract;
    let sender = deps.api.addr_validate(&info.sender.to_string())?.to_string();
    if sender != redirection_contract {
        return Err(ContractError::Unauthorized{})
    };
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

pub fn swap_aust_ust(
    deps: DepsMut,
    info: MessageInfo,
    withdraw_amount: Uint128,
    aust_amount: String,
    ust_amount: String, 
    percentage: String,
    depositor: String, 
) -> Result<Response, ContractError> {
    let redirection_contract = CONFIG.load(deps.storage)?.redirection_contract;
    let sender = deps.api.addr_validate(&info.sender.to_string())?.to_string();
    if sender != redirection_contract {
        return Err(ContractError::Unauthorized{})
    };
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

pub fn withdraw_send(
    deps: DepsMut,
    info: MessageInfo,
    withdraw_amount: u64,
    new_ust_amount: u64,
    to_angel_amount: u64,
    ust_depositor: String,
    charity_address: String,
) -> Result<Response, ContractError> {
    let redirection_contract = CONFIG.load(deps.storage)?.redirection_contract;
    let sender = deps.api.addr_validate(&info.sender.to_string())?.to_string();
    if sender != redirection_contract {
        return Err(ContractError::Unauthorized{})
    };
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