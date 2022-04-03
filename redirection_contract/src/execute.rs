use cosmwasm_std::{Response, WasmMsg, to_binary, coin, SubMsg, CosmosMsg, ReplyOn, Uint128, DepsMut, Env, ContractResult, SubMsgExecutionResponse};
use crate::{ContractError, msg::EscrowMsg, state::{Pool, USER_INFO, STATE}};

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

    let diff = parsed_ust_exchanged - parsed_ust_amount;
    let to_angel;
    if diff == 0 {
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