#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, Coin, Uint128, WasmMsg, SubMsg, CosmosMsg, ReplyOn, Reply, ContractResult, SubMsgExecutionResponse};
use cw2::set_contract_version;

use crate::error::{ContractError, PaymentError};
use crate::msg::{ExecuteMsg, InstantiateMsg, EscrowMsg};
use crate::state::{State, STATE, USER_INFO, Pool};

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
        swap_contract: String::from(""),
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
        10 => deposit_more(deps, env, msg.result),
        11 => update_user_struct(deps, env, msg.result),
        _ => Err(ContractError::Unauthorized {}),
    }
}

fn update_user_struct(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut new_percentage = String::from("");
            let mut ust_amount = String::from("");
            let mut mint_amount = String::from("");

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "ust_amount" {
                        ust_amount = attrb.value;
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
            tokens.ust_amount = ust_amount;
            tokens.give_percentage = new_percentage;

            USER_INFO.save(deps.storage, &ust_depositor, &tokens)?;
            Ok(Response::default())
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

fn deposit_more(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {  
    match msg {
        ContractResult::Ok(subcall) => {
            let mut ust_depositor = String::from("");
            let mut percentage = String::from("");
            let mut deposit_amount = String::from("");
            let mut return_amount = String::from("");

            for event in subcall.events {
                for attrb in event.attributes {
                    if attrb.key == "ust_sent" {
                        deposit_amount = attrb.value;
                    } else if attrb.key == "return_amount" {
                        return_amount = attrb.value;
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
                return_amount, 
                percentage, 
                ust_depositor
            )
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

fn make_new_user_struct(
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

fn update_pool (
    deps: DepsMut,
    deposit_amount: String,
    return_amount: String,
    percentage: String,
    ust_depositor: String,
) -> Result<Response, ContractError> {
    let user_info = USER_INFO.load(deps.storage, &ust_depositor)?;
    let state = STATE.load(deps.storage)?;

    let parsed_ust_exchanged = return_amount.parse::<u64>().unwrap();
    let parsed_ust_amount = user_info.ust_amount.parse::<u64>().unwrap();
    let parsed_deposit_amount =  deposit_amount.parse::<u64>().unwrap();
    let parsed_prev_percentage = user_info.give_percentage.parse::<u64>().unwrap();
    let parsed_percentage =  percentage.parse::<u64>().unwrap();

    let to_angel = (parsed_ust_exchanged - parsed_ust_amount) * (parsed_percentage / 100);
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
        funds: vec![],
    };

    let escrow_execute = SubMsg {
        id: 11,
        msg: CosmosMsg::Wasm(swapback),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    return Ok(Response::new()
        .add_submessage(escrow_execute)
    )
}

fn make_new_pool (
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

    let ust_sent = must_pay(&info, "uusd")?;
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
        id: 10,
        msg: CosmosMsg::Wasm(native_swap_contact),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    return Ok(Response::new()
        .add_submessage(escrow_execute)
    )
}

// pub fn withdraw_pool(
//     deps: DepsMut,
//     info: MessageInfo,
// ) -> Result<Response, ContractError> {
//     let withdrawer = deps.api.addr_validate(&info.sender.as_str())?;
    
//     if !USER_INFO.has(deps.storage, withdrawer.as_str()) {
//         return Err(ContractError::NoDeposit {});
//     }

//     let deposit_pool = USER_INFO.load(deps.storage, withdrawer.as_str())?;
//     let state = STATE.load(deps.storage)?;
//     let epoch: EpochStateResponse = deps.querier.query_wasm_smart(
//         state.anchor_market_contract.to_string(),
//         &AnchorQueryMsg::EpochState {
//             block_height: None,
//             distributed_interest: None,
//         }
//     )?;

//     let aust_withdraw_amount = deposit_pool.aust_amount.parse::<u128>().unwrap();
//     let epoch_exchange_rate = convert_str_int(epoch.exchange_rate.to_string());
//     let exchange_rate_diff = epoch_exchange_rate - deposit_pool.epoch_exchange_rate_at_deposit.parse::<u128>().unwrap();
//     let to_angel = (aust_withdraw_amount / 1000000 * exchange_rate_diff) / (100 / deposit_pool.give_percentage.parse::<u128>().unwrap());
//     let to_user = aust_withdraw_amount - to_angel;
//     let ust_amount = to_user * epoch_exchange_rate / 1000000;

//     let convert_to_ust = WasmMsg::Execute {
//         contract_addr: state.aust_contract,
//         msg: to_binary(&Cw20ExecuteMsg::Send {
//             contract: state.anchor_market_contract,
//             msg: to_binary(&Cw20HookMsg::RedeemStable{}).unwrap(),
//             amount: Uint128::new(to_user)
//         }).unwrap(),
//         funds: Vec::new()
//     };

//     let send_to_withdrawer = BankMsg::Send { 
//         to_address: withdrawer.clone().to_string(),
//         amount: vec![coin(ust_amount, "uusd")] 
//     };

//     let cp = ANGEL_INFO.load(deps.storage)?;
//     let new_cp = CharityPool {
//         aust_amount: (cp.aust_amount.parse::<u128>().unwrap() + to_angel).to_string(),
//     };
//     ANGEL_INFO.save(deps.storage, &new_cp)?;
//     USER_INFO.remove(deps.storage, withdrawer.as_str());

//     Ok(Response::new()
//     .add_message(convert_to_ust)
//     .add_message(send_to_withdrawer)
//     .add_attribute("method", "execute")
//     .add_attribute("to_angel",to_angel.to_string())
//     .add_attribute("ex_diff", exchange_rate_diff.to_string())
//     )
// }

// pub fn withdraw_charity(
//     deps: DepsMut,
//     info: MessageInfo,
// ) -> Result<Response, ContractError> {
//     let withdrawer = deps.api.addr_validate(&info.sender.as_str())?.to_string();
//     let state = STATE.load(deps.storage)?;
//     let charity_pool = ANGEL_INFO.load(deps.storage)?;

//     if state.charity_address != withdrawer {
//         return Err(ContractError::NotAngelAddr {});
//     } else if charity_pool.aust_amount.parse::<u128>().unwrap() == 0 {
//         return Err(ContractError::NoBalance {});
//     }

//     let aust_withdraw_amount = charity_pool.aust_amount.parse::<u128>().unwrap();
//     let withdraw_charity_balance = WasmMsg::Execute {
//         contract_addr: state.aust_contract.to_string(),
//         msg: to_binary(&Cw20ExecuteMsg::Transfer {
//             recipient: withdrawer.clone().to_string(),
//             amount: Uint128::from(aust_withdraw_amount),
//         }).unwrap(),
//         funds: Vec::new(),
//     };

//     ANGEL_INFO.save(deps.storage, &CharityPool { aust_amount: String::from("0") })?;

//     Ok(Response::new()
//         .add_message(withdraw_charity_balance)
//         .add_attribute("method", "execute")
//     )
// }

pub fn coin(amount: u128, denom: impl Into<String>) -> Coin {
    Coin::new(amount, denom)
}

/// Requires exactly one denom sent, which matches the requested denom.
/// Returns the amount if only one denom and non-zero amount. Errors otherwise.
pub fn must_pay(info: &MessageInfo, denom: &str) -> Result<Uint128, PaymentError> {
    let coin = one_coin(info)?;
    if coin.denom != denom {
        Err(PaymentError::MissingDenom(denom.to_string()))
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
pub fn one_coin(info: &MessageInfo) -> Result<Coin, PaymentError> {
    match info.funds.len() {
        0 => Err(PaymentError::NoFunds {}),
        1 => {
            let coin = &info.funds[0];
            if coin.amount.is_zero() {
                Err(PaymentError::NoFunds {})
            } else {
                Ok(coin.clone())
            }
        }
        _ => Err(PaymentError::MultipleDenoms {}),
    }
}

#[cfg(test)]
mod tests {}
