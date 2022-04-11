use crate::{
    error::PaymentError,
    msg::{AnchorExecuteMsg, Cw20HookMsg, ExecuteMsg},
    state::{Pool, CONFIG, USER_INFO, Config},
    ContractError,
};
use cosmwasm_std::{
    coin, to_binary, ContractResult, CosmosMsg, DepsMut, Env, MessageInfo, ReplyOn, Response,
    SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg, BankMsg,
};
use cw20::Cw20ExecuteMsg;

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    msg: Config,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender.ne(&config.admin) {
        return Err(ContractError::Unauthorized {});
    };

    CONFIG.save(deps.storage, &msg)?;

    Ok(Response::default())
}

pub fn make_new_deposit(
    env: Env,
    depositor: String,
    percentage: u16,
    ust_sent: Uint128,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_submessage(SubMsg {
        id: 0,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::InternalDepositInitial {
                ust_sent,
                percentage,
                depositor,
            })?,
            funds: vec![],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }))
}

pub fn send_dust_to_angel_then_make_new_deposit(
    deps: DepsMut,
    env: Env,
    depositor: String,
    percentage: u16,
    ust_sent: Uint128,
    user_info: Pool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let escrow_execute = SubMsg {
        id: 0,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::InternalDepositInitial {
                ust_sent,
                percentage,
                depositor: depositor.clone(),
            })?,
            funds: vec![],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    let mut new_user_info = user_info.clone();
    new_user_info.aust_amount = String::from("0");
    new_user_info.ust_amount = String::from("0");

    USER_INFO.save(deps.storage, &depositor, &new_user_info)?;

    Ok(Response::new()
        .add_message(WasmMsg::Execute {
        contract_addr: config.aust_token_address.to_string(), // Should not hardcode this! Move to config.
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: config.charity_address.to_string(),
            amount: Uint128::from(user_info.aust_amount.parse::<u64>().unwrap()),
        })
        .unwrap(),
        funds: Vec::new(),
        })
        .add_submessage(escrow_execute))
}

pub fn update_deposit(
    env: Env,
    ust_sent: Uint128,
    depositor: String,
    percentage: u16,
    aust_amount: String,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_submessage(SubMsg {
        id: 1,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::InternalDepositMore {
                ust_sent,
                aust_amount,
                percentage,
                depositor,
            })?,
            funds: vec![],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }))
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
                .add_attribute("aust_amount", mint_amount))
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
            let new_percentage = ((ust_amount * prev_percentage) + (deposit_amount * percentage))
                / (ust_amount + deposit_amount);

            Ok(Response::new()
                .add_attribute("diff", diff.to_string())
                .add_attribute("to_angel", to_angel.to_string())
                .add_submessage(SubMsg {
                    id: 2,
                    msg: CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: env.contract.address.to_string(),
                        msg: to_binary(&ExecuteMsg::InternalSwapBackUpdate {
                            to_angel,
                            charity_address: config.charity_address.to_string(),
                            ust_amount: new_ust_amount,
                            new_percentage,
                            depositor: ust_depositor,
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
    env: Env,
    withdraw_amount: Uint128,
    depositor: String,
) -> Result<Response, ContractError> {
    let user_info = USER_INFO.load(deps.storage, &depositor)?;
    let aust_amount = user_info.aust_amount;
    let ust_amount = user_info.ust_amount;
    let percentage = user_info.give_percentage;

    Ok(Response::new().add_submessage(SubMsg {
        id: 3,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::InternalWithdrawInitial {
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
    }))
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
                        ust_depositor,
                        charity_address: config.charity_address.to_string(),
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
            let mut deposit_amount = String::from("0"); //in case the anchor_deposit doesn't trigger
            let mut mint_amount = String::from("0"); //in case the anchor_deposit doesn't trigger

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
            let config = CONFIG.load(deps.storage)?;
            if mint_amount.parse::<u64>().unwrap() < config.theta {
                tokens.give_percentage = String::from("0");
            }

            tokens.aust_amount = mint_amount;
            tokens.ust_amount = deposit_amount;

            USER_INFO.save(deps.storage, &ust_depositor, &tokens)?;
            Ok(Response::default())
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

/* Internal Functions */
pub fn deposit_initial(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ust_sent: Uint128,
    percentage: u16,
    depositor: String,
) -> Result<Response, ContractError> {
    if info.sender.ne(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    };

    let config = CONFIG.load(deps.storage)?;

    let deposit_stable = AnchorExecuteMsg::DepositStable {};
    let anchor_deposit = WasmMsg::Execute {
        contract_addr: config.anchor_market_address.to_string(),
        msg: to_binary(&deposit_stable)?,
        funds: vec![coin(ust_sent.u128(), "uusd")],
    };

    Ok(Response::new()
        .add_attribute("percentage", percentage.to_string())
        .add_attribute("ust_depositor", depositor)
        .add_message(anchor_deposit))
}

pub fn deposit_more(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ust_sent: Uint128,
    aust_amount: String,
    percentage: u16,
    depositor: String,
) -> Result<Response, ContractError> {
    if info.sender.ne(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    };

    let config = CONFIG.load(deps.storage)?;
    let anchor_market_address = config.anchor_market_address.to_string();
    let aust_token_address = config.aust_token_address.to_string();

    let convert_to_ust = 
    get_convert_to_ust(
        anchor_market_address, 
        aust_token_address, 
        aust_amount.clone()
    );

    Ok(Response::new()
        .add_attribute("ust_sent", ust_sent)
        .add_attribute("percentage", percentage.to_string())
        .add_attribute("ust_depositor", depositor)
        .add_message(convert_to_ust))
}

pub fn swap_back_aust(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_angel: u64,
    charity_address: String,
    ust_amount: u64,
    new_percentage: u64,
    depositor: String,
) -> Result<Response, ContractError> {
    if info.sender.ne(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    };

    let config = CONFIG.load(deps.storage)?;

    let mut res = Response::new()
        .add_attribute("new_percentage", new_percentage.to_string())
        .add_attribute("ust_depositor", depositor);
    // if going to an Angel Charity add the bank msg
    if to_angel != 0 {
        res = res.add_message(BankMsg::Send {
            to_address: charity_address,
            amount: vec![coin(to_angel.into(), "uusd")],
        });
    }
    // add the anchor deposit message last in all cases
    res = res.add_message(WasmMsg::Execute {
        contract_addr: config.anchor_market_address.to_string(),
        msg: to_binary(&AnchorExecuteMsg::DepositStable {})?,
        funds: vec![coin(ust_amount.into(), "uusd")],
    });

    Ok(res)
}

pub fn swap_aust_ust(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdraw_amount: Uint128,
    aust_amount: String,
    ust_amount: String,
    percentage: String,
    depositor: String,
) -> Result<Response, ContractError> {
    if info.sender.ne(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    };

    let config = CONFIG.load(deps.storage)?;
    let anchor_market_address = config.anchor_market_address.to_string();
    let aust_token_address = config.aust_token_address.to_string();

    let convert_to_ust = 
    get_convert_to_ust(
        anchor_market_address, 
        aust_token_address, 
        aust_amount.clone()
    );

    Ok(Response::new()
        .add_attribute("withdraw_amount", withdraw_amount)
        .add_attribute("percentage", percentage)
        .add_attribute("ust_depositor", depositor)
        .add_attribute("ust_amount", ust_amount)
        .add_attribute("aust_amount", aust_amount)
        .add_message(convert_to_ust))
}

pub fn withdraw_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdraw_amount: u64,
    new_ust_amount: u64,
    to_angel_amount: u64,
    ust_depositor: String,
    charity_address: String,
) -> Result<Response, ContractError> {
    if info.sender.ne(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    };

    let config = CONFIG.load(deps.storage)?;

    let withdraw_to_user = BankMsg::Send {
        to_address: ust_depositor.clone(),
        amount: vec![coin(withdraw_amount.into(), "uusd")],
    };
    let send_to_charity = BankMsg::Send {
        to_address: charity_address,
        amount: vec![coin(to_angel_amount.into(), "uusd")],
    };
    let anchor_deposit = WasmMsg::Execute {
        contract_addr: config.anchor_market_address.to_string(),
        msg: to_binary(&AnchorExecuteMsg::DepositStable {})?,
        funds: vec![coin(new_ust_amount.into(), "uusd")],
    };

    let mut res = Response::new()
        .add_attribute("ust_depositor", ust_depositor)
        .add_message(withdraw_to_user);
    if to_angel_amount != 0 {
        res = res.add_message(send_to_charity);
    }
    if new_ust_amount != 0 {
        res = res.add_message(anchor_deposit);
    }

    Ok(res)
}

//Helpers
fn get_convert_to_ust(
    anchor_market_address: String,
    aust_token_address: String,
    aust_amount: String
) -> WasmMsg {
    return WasmMsg::Execute {
        contract_addr: aust_token_address, // Should not hardcode this! Move to config.
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: anchor_market_address, // Should not hardcode this! Move to config.
            msg: to_binary(&Cw20HookMsg::RedeemStable {}).unwrap(),
            amount: Uint128::new(aust_amount.parse::<u128>().unwrap()),
        })
        .unwrap(),
        funds: Vec::new(),
    };
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
