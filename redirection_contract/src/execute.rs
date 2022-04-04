use cosmwasm_std::{
    Response, WasmMsg, to_binary, coin, 
    SubMsg, CosmosMsg, ReplyOn, Uint128, DepsMut, 
    Env, ContractResult, SubMsgExecutionResponse, MessageInfo
};
use crate::{ContractError, msg::{EscrowMsg}, 
state::{Pool, USER_INFO, CONFIG, Config}, error::PaymentError};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
) -> Result<Response, ContractError> {
    let admin = CONFIG.load(deps.storage)?.admin;
    let sender = deps.api.addr_validate(&info.sender.to_string())?.to_string();

    if admin != sender {
        return Err(ContractError::Unauthorized {})
    };
    
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

pub fn make_new_deposit(
    escrow_controller: String,
    depositor: String,
    percentage: u16,
    ust_sent: u128,
) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_submessage(SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: escrow_controller,
            msg: to_binary(&EscrowMsg::DepositInitial {
            percentage,
            depositor: String::from(depositor),
        })?,
            funds: vec![coin(ust_sent, "uusd")],
        }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        })
    )
}

pub fn send_dust_to_angel_then_make_new_deposit(
    deps: DepsMut,
    escrow_controller: String,
    depositor: String,
    percentage: u16,
    ust_sent: u128,
    user_info: Pool,
) -> Result<Response, ContractError> {
    let charity_address = CONFIG.load(deps.storage)?.charity_address;

    let send_dust = WasmMsg::Execute {
        contract_addr: escrow_controller.clone(),
        msg: to_binary(&EscrowMsg::SendDust { 
        charity_address, 
        aust_amount: user_info.aust_amount.parse::<u64>().unwrap(),
    })?,
        funds: vec![],
    };

    let escrow_execute = SubMsg {
        id: 0,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: escrow_controller,
        msg: to_binary(&EscrowMsg::DepositInitial {
        percentage,
        depositor: depositor.clone(),
    })?,
        funds: vec![coin(ust_sent, "uusd")],
    }),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    let mut new_user_info = user_info.clone();
    new_user_info.aust_amount = String::from("0");
    new_user_info.ust_amount = String::from("0");

    USER_INFO.save(deps.storage, &depositor, &new_user_info)?;

    Ok(Response::new()
        .add_message(send_dust)
        .add_submessage(escrow_execute)
    )
}

pub fn update_deposit(
    ust_sent: Uint128,
    escrow_controller: String,
    depositor: String,
    percentage: u16,
    aust_amount: String,
) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_submessage(SubMsg {
        id: 1,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: escrow_controller,
        msg: to_binary(&EscrowMsg::DepositMore { 
            ust_sent,
            aust_amount,
            percentage, 
            depositor 
        })?,
        funds: vec![],
    }),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    })
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
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

pub fn get_new_user_state_dep(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {  
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut percentage = 0;
            let mut deposit_amount = 0;
            let mut redeem_amount = 0;

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "ust_sent" {
                        deposit_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "redeem_amount" {
                        redeem_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "percentage" {
                        percentage = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            let user_info = USER_INFO.load(deps.storage, &ust_depositor)?;
            let state = CONFIG.load(deps.storage)?;
            let ust_amount = user_info.ust_amount.parse::<u64>().unwrap();
            let prev_percentage = user_info.give_percentage.parse::<u64>().unwrap();

            let diff;
            if ust_amount > redeem_amount {
                diff = 0;
            } else {
                diff = redeem_amount - ust_amount;
            }
        
            let to_angel = (diff * prev_percentage) / 100;

            let new_ust_amount = redeem_amount + deposit_amount - to_angel;
            let new_percentage = ((ust_amount * prev_percentage) + 
                                      (deposit_amount * percentage)) / 
                                      (ust_amount + deposit_amount);
    
            Ok(Response::new()
                .add_attribute("diff", diff.to_string())
                .add_attribute("to_angel", to_angel.to_string())
                .add_submessage(SubMsg {
                    id: 2,
                    msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: state.escrow_controller,
                    msg: to_binary(&EscrowMsg::SwapBackUpdate { 
                    to_angel,
                    charity_address: state.charity_address,
                    ust_amount: new_ust_amount,
                    new_percentage, 
                    depositor: ust_depositor,
                })?,
                    funds: vec![coin(deposit_amount.into(), "uusd")],
                }),
                    gas_limit: None,
                    reply_on: ReplyOn::Success,
                })
            )
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

pub fn deposit_then_update_user(
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
    let escrow_controller = CONFIG.load(deps.storage)?.escrow_controller;

    Ok(Response::new()
        .add_submessage(SubMsg {
            id: 3,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: escrow_controller,
            msg: to_binary(&EscrowMsg::WithdrawInitial { 
                withdraw_amount,
                aust_amount,
                ust_amount, 
                percentage,
                depositor, 
            })?,
            funds: vec![],
        }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        })
    )
}

pub fn get_new_user_state_wit(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut redeem_amount = 0;
            let mut withdraw_amount = 0;
            let mut ust_amount = 0;

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "redeem_amount" {
                        redeem_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "withdraw_amount" {
                        withdraw_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    } else if attrb.key == "ust_amount" {
                        ust_amount = attrb.value.parse::<u64>().unwrap();
                    }
                }
            }

            let percentage = 
            USER_INFO.load(deps.storage, &ust_depositor)?
            .give_percentage
            .parse::<u64>()
            .unwrap();

            let diff;
            if ust_amount > redeem_amount {
                diff = 0;
            } else {
                diff = redeem_amount - ust_amount;
            }

            let to_angel_amount = (diff * percentage) / 100;
            let new_ust_amount = redeem_amount - to_angel_amount - withdraw_amount;
            let state = CONFIG.load(deps.storage)?;

            Ok(Response::new().add_submessage(SubMsg {
                id: 4,
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.escrow_controller,
                msg: to_binary(&EscrowMsg::WithdrawSend { 
                withdraw_amount,
                new_ust_amount,
                to_angel_amount, 
                ust_depositor,
                charity_address: state.charity_address
            })?,
                funds: vec![],
            }),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            }))
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

pub fn withdraw_then_update_user(
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
