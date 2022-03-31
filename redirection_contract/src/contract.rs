#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, Coin, Uint128, WasmMsg, BankMsg, SubMsg, CosmosMsg, ReplyOn, Reply, ContractResult, SubMsgExecutionResponse, StdResult};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::{ContractError, PaymentError};
use crate::msg::{EpochStateResponse, ExecuteMsg, InstantiateMsg, EscrowMsg};
use crate::state::{State, STATE, USER_INFO, Pool, CharityPool, ANGEL_INFO};

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
        escrow_contract: String::from(""),
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
        0 => moru_user_struct(deps, env, msg.result),
        _ => Err(ContractError::Unauthorized {}),
    }
}

fn moru_user_struct(
    deps: DepsMut,
    _env: Env,
    msg: ContractResult<SubMsgExecutionResponse>,
) -> Result<Response, ContractError> {
    match msg {
        ContractResult::Ok(subcall) => {
            let ust_depositor = String::from("");
            let percentage = String::from("");
            let deposit_amount = String::from("");
            let mint_amount = String::from("");

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

            if USER_INFO.has(deps.storage, ust_depositor.as_str()) {
                update_pool(
                    deps,
                    deposit_amount, 
                    mint_amount, 
                    percentage, 
                    ust_depositor
                )
            } else {
                make_new_pool(
                    deps,
                    deposit_amount, 
                    mint_amount, 
                    percentage, 
                    ust_depositor
                )
            }
        }
        ContractResult::Err(_) => Err(ContractError::Unauthorized {}),
    }
}

fn update_pool (
    deps: DepsMut,
    deposit_amount: String,
    mint_amount: String,
    percentage: String,
    ust_depositor: String,
) -> Result<Response, ContractError> {
    USER_INFO.update(
    deps.storage, 
    &ust_depositor, 
    |mut user_info| -> StdResult<_> {
        //update pools
    })?;

    Ok(Response::default())
}

fn make_new_pool (
    deps: DepsMut,
    deposit_amount: String,
    mint_amount: String,
    percentage: String,
    ust_depositor: String,
) -> Result<Response, ContractError> {
    let depositor_info = Pool {
        give_percentage: percentage,
        ust_amount: deposit_amount,
        aust_amount: mint_amount,
    };

    match USER_INFO.save(deps.storage, &ust_depositor, &depositor_info)? {
        () => Ok(Response::new()
            .add_attribute("give_percentage", percentage)
            .add_attribute("ust_amount", deposit_amount)
            .add_attribute("aust_amount", mint_amount)
        ),
        _ => Err(ContractError::MakeNewPoolError {})
    }
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
        => make_deposit_pool(deps, info, percentage),
        // ExecuteMsg::WithdrawPool {}
        // => withdraw_pool(deps, info),
        // ExecuteMsg::WithdrawCharity {}
        // => withdraw_charity(deps, info),
    }
}

pub fn make_deposit_pool(
    deps: DepsMut,
    info: MessageInfo,
    percentage: u16,
) -> Result<Response, ContractError> {
    if percentage < 5 || percentage > 100 {
        return Err(ContractError::WrongPercentageInput {});
    };

    let ust_sent = must_pay(&info, "uusd")?;
    let depositor = deps.api.addr_validate(&info.sender.as_str())?;
    let escrow_address = STATE.load(deps.storage)?.escrow_contract;

    let execute_swap = EscrowMsg::ExecuteSwap {
        percentage,
        depositor: String::from(depositor),
    };

    let escrow_contact = WasmMsg::Execute {
        contract_addr: escrow_address,
        msg: to_binary(&execute_swap)?,
        funds: vec![coin(ust_sent.u128(), "uusd")],
    };

    let escrow_execute = SubMsg {
        id: 0,
        msg: CosmosMsg::Wasm(escrow_contact),
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
