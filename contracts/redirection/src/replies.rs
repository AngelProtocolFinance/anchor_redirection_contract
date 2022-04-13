use crate::{
    msg::ExecuteMsg,
    state::{Pool, CONFIG, USER_INFO},
    ContractError,
};
use cosmwasm_std::{
    coin, to_binary, ContractResult, CosmosMsg, DepsMut, Env, ReplyOn, Response,
    SubMsg, SubMsgExecutionResponse, WasmMsg, Addr,
};

pub fn make_new_user_struct(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut percentage = 0u16;
            let mut deposit_amount = 0u64;
            let mut mint_amount = 0u64;

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "deposit_amount" {
                        deposit_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "mint_amount" {
                        mint_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "percentage" {
                        percentage = attrb.value.parse::<u16>().unwrap();
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            let depositor_info;

            if !USER_INFO.has(deps.storage, &ust_depositor) {
                depositor_info = Pool {
                    give_percentage: percentage.clone(),
                    ust_amount: deposit_amount.clone(),
                    aust_amount: mint_amount.clone(),
                    total_donated: 0,
                };
            } else {
                let total_donated =
                USER_INFO.load(deps.storage, &ust_depositor)?.total_donated;
                depositor_info = Pool {
                    give_percentage: percentage.clone(),
                    ust_amount: deposit_amount.clone(),
                    aust_amount: mint_amount.clone(),
                    total_donated,
                };
            }

            USER_INFO.save(deps.storage, &ust_depositor, &depositor_info)?;

            Ok(Response::new()
                .add_attribute("give_percentage", percentage.to_string())
                .add_attribute("ust_amount", deposit_amount.to_string())
                .add_attribute("aust_amount", mint_amount.to_string())
            )
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

pub fn get_new_user_state_dep(
    deps: DepsMut,
    env: Env,
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
            let config = CONFIG.load(deps.storage)?;
            let ust_amount = user_info.ust_amount;
            let prev_percentage = user_info.give_percentage as u64;

            let diff;
            if ust_amount > redeem_amount {
                diff = 0;
            } else {
                diff = redeem_amount - ust_amount;
            }

            let to_angel = (diff * prev_percentage) / 100;

            let new_ust_amount = redeem_amount + deposit_amount - to_angel;
            let new_percentage = (((ust_amount * prev_percentage) + (deposit_amount * percentage))
                / (ust_amount + deposit_amount)) as u16;

            Ok(Response::new()
                .add_submessage(SubMsg {
                    id: 2,
                    msg: CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: env.contract.address.to_string(),
                        msg: to_binary(&ExecuteMsg::InternalSwapBackUpdate {
                            to_angel,
                            charity_address: config.charity_address,
                            ust_amount: new_ust_amount,
                            new_percentage,
                            depositor: Addr::unchecked(ust_depositor),
                        })?,
                        funds: vec![coin(deposit_amount.into(), "uusd")],
                    }),
                    gas_limit: None,
                    reply_on: ReplyOn::Success,
                }))
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
            let mut new_percentage = 0u16;
            let mut deposit_amount = 0u64;
            let mut mint_amount = 0u64;
            let mut to_angel = 0u64;

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "deposit_amount" {
                        deposit_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "mint_amount" {
                        mint_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "new_percentage" {
                        new_percentage = attrb.value.parse::<u16>().unwrap();
                    } else if attrb.key == "to_angel" {
                        to_angel = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            let mut tokens = USER_INFO.load(deps.storage, &ust_depositor)?;
            tokens.aust_amount = mint_amount;
            tokens.ust_amount = deposit_amount;
            tokens.give_percentage = new_percentage;
            tokens.total_donated += to_angel;

            USER_INFO.save(deps.storage, &ust_depositor, &tokens)?;
            Ok(Response::default())
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

pub fn get_new_user_state_wit(
    deps: DepsMut,
    env: Env,
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

            let percentage = USER_INFO
                .load(deps.storage, &ust_depositor)?
                .give_percentage as u64;

            let diff;
            if ust_amount > redeem_amount {
                diff = 0;
            } else {
                diff = redeem_amount - ust_amount;
            }

            let to_angel_amount = (diff * percentage) / 100;
            let max_withdrawable = redeem_amount - to_angel_amount;

            if withdraw_amount > max_withdrawable {
                withdraw_amount = max_withdrawable;
            };

            let new_ust_amount = redeem_amount - to_angel_amount - withdraw_amount;
            let config = CONFIG.load(deps.storage)?;

            Ok(Response::new().add_submessage(SubMsg {
                id: 4,
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    msg: to_binary(&ExecuteMsg::InternalWithdrawSend {
                        withdraw_amount,
                        new_ust_amount,
                        to_angel_amount,
                        ust_depositor: Addr::unchecked(ust_depositor),
                        charity_address: config.charity_address,
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
            let mut deposit_amount = 0u64; //in case the anchor_deposit doesn't trigger
            let mut mint_amount = 0u64; //in case the anchor_deposit doesn't trigger
            let mut to_angel = 0u64;

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "deposit_amount" {
                        deposit_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "mint_amount" {
                        mint_amount = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "to_angel" {
                        to_angel = attrb.value.parse::<u64>().unwrap();
                    } else if attrb.key == "ust_depositor" {
                        ust_depositor = attrb.value;
                    }
                }
            }

            let mut tokens = USER_INFO.load(deps.storage, &ust_depositor)?;
            let config = CONFIG.load(deps.storage)?;
            if mint_amount < config.theta {
                tokens.give_percentage = 0u16;
            }

            tokens.aust_amount = mint_amount;
            tokens.ust_amount = deposit_amount;
            tokens.total_donated += to_angel;

            USER_INFO.save(deps.storage, &ust_depositor, &tokens)?;
            Ok(Response::default())
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}
