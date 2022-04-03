use cosmwasm_std::{Response, WasmMsg, to_binary, coin, SubMsg, CosmosMsg, ReplyOn, Uint128, DepsMut, Env, ContractResult, SubMsgExecutionResponse, MessageInfo};
use crate::{ContractError, msg::EscrowMsg, state::{Pool, USER_INFO, STATE}, error::PaymentError};

pub fn make_new_deposit(
    swap_address: String,
    depositor: String,
    percentage: u16,
    ust_sent: u128,
) -> Result<Response, ContractError> {
    let execute_native_swap = EscrowMsg::DepositInitial {
        percentage,
        depositor: String::from(depositor),
    };

    let native_swap_contact = WasmMsg::Execute {
        contract_addr: swap_address,
        msg: to_binary(&execute_native_swap)?,
        funds: vec![coin(ust_sent, "uusd")],
    };

    let escrow_execute = SubMsg {
        id: 0,
        msg: CosmosMsg::Wasm(native_swap_contact),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    return Ok(Response::new()
        .add_submessage(escrow_execute)
    )
}

pub fn update_deposit(
    ust_sent: Uint128,
    swap_address: String,
    depositor: String,
    percentage: u16,
    aust_amount: String,
) -> Result<Response, ContractError> {
    let aust_ust_swap = EscrowMsg::DepositMore { 
        ust_sent,
        aust_amount,
        percentage, 
        depositor 
    };

    let native_swap_contact = WasmMsg::Execute {
        contract_addr: swap_address,
        msg: to_binary(&aust_ust_swap)?,
        funds: vec![],
    };

    let escrow_execute = SubMsg {
        id: 1,
        msg: CosmosMsg::Wasm(native_swap_contact),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    return Ok(Response::new()
        .add_submessage(escrow_execute)
    )
}

/* Submessage Reply Functions */
pub fn make_new_user_struct(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut percentage = String::from("");
            let mut deposit_amount = String::from("");
            let mut mint_amount = String::from("");

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "deposit_amount" {
                        deposit_amount = attrb.value;
                    } else if attrb.key == "mint_amount" {
                        mint_amount = attrb.value;
                    } else if attrb.key == "percentage" {
                        percentage = attrb.value;
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            make_new_pool(
                deps,
                deposit_amount, 
                mint_amount, 
                percentage, 
                ust_depositor
            )
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

pub fn make_new_pool (
    deps: DepsMut,
    deposit_amount: String,
    mint_amount: String,
    percentage: String,
    ust_depositor: String,
) -> Result<Response, ContractError> {
    let depositor_info = Pool {
        give_percentage: percentage.clone(),
        ust_amount: deposit_amount.clone(),
        aust_amount: mint_amount.clone(),
    };

    USER_INFO.save(deps.storage, &ust_depositor, &depositor_info)?;

    Ok(Response::new()
        .add_attribute("give_percentage", percentage)
        .add_attribute("ust_amount", deposit_amount)
        .add_attribute("aust_amount", mint_amount)
    )
}

pub fn deposit_more(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {  
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut percentage = String::from("");
            let mut deposit_amount = String::from("");
            let mut redeem_amount = String::from("");

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "ust_sent" {
                        deposit_amount = attrb.value;
                    } else if attrb.key == "redeem_amount" {
                        redeem_amount = attrb.value;
                    } else if attrb.key == "percentage" {
                        percentage = attrb.value;
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            update_pool(
                deps,
                deposit_amount, 
                redeem_amount, 
                percentage, 
                ust_depositor
            )
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

fn update_pool (
    deps: DepsMut,
    deposit_amount: String,
    redeem_amount: String,
    percentage: String,
    ust_depositor: String,
) -> Result<Response, ContractError> {
    let user_info = USER_INFO.load(deps.storage, &ust_depositor)?;
    let state = STATE.load(deps.storage)?;

    let parsed_ust_exchanged = redeem_amount.parse::<u64>().unwrap();
    let parsed_ust_amount = user_info.ust_amount.parse::<u64>().unwrap();
    let parsed_deposit_amount =  deposit_amount.parse::<u64>().unwrap();
    let parsed_prev_percentage = user_info.give_percentage.parse::<u64>().unwrap();
    let parsed_percentage =  percentage.parse::<u64>().unwrap();

    //what if i take the parsed, do ust_amount - ust_exchanged...?;
    let diff = parsed_ust_exchanged - parsed_ust_amount;
    let to_angel;
    if diff < 100 {
        to_angel = 0;
    } else {
        to_angel = diff * (parsed_prev_percentage / 100);
    }
    
    let new_ust_amount = parsed_ust_exchanged - to_angel + parsed_deposit_amount;
    let new_percentage = 
    ((parsed_ust_amount * parsed_prev_percentage) + (parsed_deposit_amount * parsed_percentage)) / 
    (parsed_ust_amount + parsed_deposit_amount);

    let ust_aust_swapback = EscrowMsg::SwapBackUpdate { 
        to_angel,
        charity_address: state.charity_address,
        ust_amount: new_ust_amount,
        new_percentage, 
        depositor: ust_depositor,
    };

    let swapback = WasmMsg::Execute {
        contract_addr: state.swap_contract,
        msg: to_binary(&ust_aust_swapback)?,
        funds: vec![coin(parsed_deposit_amount.into(), "uusd")],
    };

    let escrow_execute = SubMsg {
        id: 2,
        msg: CosmosMsg::Wasm(swapback),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    return Ok(Response::new()
        .add_submessage(escrow_execute)
    )
}

pub fn update_user_struct(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut new_percentage = String::from("");
            let mut deposit_amount = String::from("");
            let mut mint_amount = String::from("");

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "deposit_amount" {
                        deposit_amount = attrb.value;
                    } else if attrb.key == "mint_amount" {
                        mint_amount = attrb.value;
                    } else if attrb.key == "new_percentage" {
                        new_percentage = attrb.value;
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            let mut tokens = USER_INFO.load(deps.storage, &ust_depositor)?;
            tokens.aust_amount = mint_amount;
            tokens.ust_amount = deposit_amount;
            tokens.give_percentage = new_percentage;

            USER_INFO.save(deps.storage, &ust_depositor, &tokens)?;
            Ok(Response::default())
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

//Withdraws
pub fn withdraw_deposit(
    deps: DepsMut,
    withdraw_amount: Uint128,
    depositor: String,
) -> Result<Response, ContractError> {
    let user_info = USER_INFO.load(deps.storage, &depositor)?;
    let aust_amount = user_info.aust_amount;
    let ust_amount = user_info.ust_amount;
    let percentage = user_info.give_percentage;
    let swap_address = STATE.load(deps.storage)?.swap_contract;

    let aust_ust_swap = EscrowMsg::WithdrawInitial { 
        withdraw_amount,
        aust_amount,
        ust_amount, 
        percentage,
        depositor, 
    };

    let swap_function = WasmMsg::Execute {
        contract_addr: swap_address,
        msg: to_binary(&aust_ust_swap)?,
        funds: vec![],
    };

    let escrow_execute = SubMsg {
        id: 3,
        msg: CosmosMsg::Wasm(swap_function),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    return Ok(Response::new()
        .add_submessage(escrow_execute)
    )
}

pub fn get_new_user_state(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut redeem_amount = String::from("");
            let mut withdraw_amount = String::from("");
            let mut ust_amount = String::from("");

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "redeem_amount" {
                        redeem_amount = attrb.value;
                    } else if attrb.key == "withdraw_amount" {
                        withdraw_amount = attrb.value;
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    } else if attrb.key == "ust_amount" {
                        ust_amount = attrb.value;
                    }
                }
            }

            let parsed_redeem_amount = redeem_amount.parse::<u64>().unwrap();
            let parsed_withdraw_amount = withdraw_amount.parse::<u64>().unwrap();
            let parsed_ust_amount = ust_amount.parse::<u64>().unwrap();

            let percentage = USER_INFO.load(deps.storage, &ust_depositor)?.give_percentage;
            let parsed_percentage = percentage.parse::<u64>().unwrap();

            let diff = parsed_redeem_amount - parsed_ust_amount;
            let to_angel_amount;
            if diff < 100 {
                to_angel_amount = 0;
            } else {
                to_angel_amount = diff * (parsed_percentage / 100);
            };

            let new_ust_amount = parsed_redeem_amount - to_angel_amount - parsed_withdraw_amount;

            let state = STATE.load(deps.storage)?;
            let swap_address = state.swap_contract;
            let charity_address = state.charity_address;

            let withdraw_and_send = EscrowMsg::WithdrawSend { 
                withdraw_amount: parsed_withdraw_amount,
                new_ust_amount,
                to_angel_amount, 
                ust_depositor,
                charity_address
            };
        
            let withdraw_function = WasmMsg::Execute {
                contract_addr: swap_address,
                msg: to_binary(&withdraw_and_send)?,
                funds: vec![],
            };
        
            let escrow_execute = SubMsg {
                id: 4,
                msg: CosmosMsg::Wasm(withdraw_function),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            };

            Ok(Response::new().add_submessage(escrow_execute))
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

pub fn update_user_struct_after_withdraw(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut deposit_amount = String::from("");
            let mut mint_amount = String::from("");

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "deposit_amount" {
                        deposit_amount = attrb.value;
                    } else if attrb.key == "mint_amount" {
                        mint_amount = attrb.value;
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            let mut tokens = USER_INFO.load(deps.storage, &ust_depositor)?;
            tokens.aust_amount = mint_amount;
            tokens.ust_amount = deposit_amount;

            USER_INFO.save(deps.storage, &ust_depositor, &tokens)?;
            Ok(Response::default())
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

/* Helpers */
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
