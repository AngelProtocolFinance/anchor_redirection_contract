#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, Coin, Uint128, WasmMsg, BankMsg};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::{ContractError, PaymentError};
use crate::msg::{EpochStateResponse, ExecuteMsg, InstantiateMsg, AnchorQueryMsg, AnchorExecuteMsg, Cw20HookMsg};
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
        anchor_market_contract: msg.anchor_market_contract,
        aust_contract: msg.aust_contract,
        charity_address: msg.charity_address,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    let charity = CharityPool {
        aust_amount: String::from("0"),
    };

    ANGEL_INFO.save(deps.storage, &charity)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("instantiate_value", state.anchor_market_contract))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DepositPool { percentage } 
        => make_deposit_pool(deps, info, percentage),
        ExecuteMsg::WithdrawPool {}
        => withdraw_pool(deps, info),
        ExecuteMsg::WithdrawCharity {}
        => withdraw_charity(deps, info),
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
    
    if USER_INFO.has(deps.storage, depositor.as_str()) {
        return Err(ContractError::MaxPooled {});
    }

    let state = STATE.load(deps.storage)?;

    let epoch: EpochStateResponse = deps.querier.query_wasm_smart(
        state.anchor_market_contract.to_string(),
        &AnchorQueryMsg::EpochState {
            block_height: None,
            distributed_interest: None,
        }
    )?;
    let deposit_stable = AnchorExecuteMsg::DepositStable {};
    let anchor_deposit = WasmMsg::Execute {
        contract_addr: state.anchor_market_contract.to_string(),
        msg: to_binary(&deposit_stable)?,
        funds: vec![coin(ust_sent.u128(), "uusd")],
    };
    // +1 on exchange rate because u256 -> u128 conversion needs a round up
    //the conversion buffers will take off 0.000003 ~ 0.0000009% of initial deposit when withdrawing to UST
    //Ex: If a user deposits 10 UST, then roughly 0.000003 UST to 0.000009 UST will be taken off at withdraw
    let epoch_exchange_rate = convert_str_int(epoch.exchange_rate.to_string()) + 1;
    let aust_converted = ust_sent.u128() * 1000000 / epoch_exchange_rate;

    let depositor_info = Pool {
        give_percentage: percentage.clone().to_string(),
        ust_amount: ust_sent.clone().u128().to_string(),
        aust_amount: Uint128::from(aust_converted.clone()).to_string(),
        epoch_exchange_rate_at_deposit: (epoch_exchange_rate - 1).to_string(),
    };

    USER_INFO.save(deps.storage, depositor.clone().as_str(), &depositor_info.clone())?;

    return Ok(Response::new()
        .add_message(anchor_deposit)
        .add_attribute("method", "execute")
        .add_attribute("executed", "make_deposit_pool")
        .add_attribute("ust_deposited", ust_sent)
        .add_attribute("aust_converted", Uint128::from(aust_converted))
    )
}

pub fn withdraw_pool(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let withdrawer = deps.api.addr_validate(&info.sender.as_str())?;
    
    if !USER_INFO.has(deps.storage, withdrawer.as_str()) {
        return Err(ContractError::NoDeposit {});
    }

    let deposit_pool = USER_INFO.load(deps.storage, withdrawer.as_str())?;
    let state = STATE.load(deps.storage)?;
    let epoch: EpochStateResponse = deps.querier.query_wasm_smart(
        state.anchor_market_contract.to_string(),
        &AnchorQueryMsg::EpochState {
            block_height: None,
            distributed_interest: None,
        }
    )?;

    let aust_withdraw_amount = deposit_pool.aust_amount.parse::<u128>().unwrap();
    let epoch_exchange_rate = convert_str_int(epoch.exchange_rate.to_string());
    let exchange_rate_diff = epoch_exchange_rate - deposit_pool.epoch_exchange_rate_at_deposit.parse::<u128>().unwrap();
    let to_angel = (aust_withdraw_amount / 1000000 * exchange_rate_diff) / (100 / deposit_pool.give_percentage.parse::<u128>().unwrap());
    let to_user = aust_withdraw_amount - to_angel;
    let ust_amount = to_user * epoch_exchange_rate / 1000000;

    let convert_to_ust = WasmMsg::Execute {
        contract_addr: state.aust_contract,
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: state.anchor_market_contract,
            msg: to_binary(&Cw20HookMsg::RedeemStable{}).unwrap(),
            amount: Uint128::new(to_user)
        }).unwrap(),
        funds: Vec::new()
    };

    let send_to_withdrawer = BankMsg::Send { 
        to_address: withdrawer.clone().to_string(),
        amount: vec![coin(ust_amount, "uusd")] 
    };

    let cp = ANGEL_INFO.load(deps.storage)?;
    let new_cp = CharityPool {
        aust_amount: (cp.aust_amount.parse::<u128>().unwrap() + to_angel).to_string(),
    };
    ANGEL_INFO.save(deps.storage, &new_cp)?;
    USER_INFO.remove(deps.storage, withdrawer.as_str());

    Ok(Response::new()
    .add_message(convert_to_ust)
    .add_message(send_to_withdrawer)
    .add_attribute("method", "execute")
    .add_attribute("to_angel",to_angel.to_string())
    .add_attribute("ex_diff", exchange_rate_diff.to_string())
    )
}

pub fn withdraw_charity(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let withdrawer = deps.api.addr_validate(&info.sender.as_str())?.to_string();
    let state = STATE.load(deps.storage)?;
    let charity_pool = ANGEL_INFO.load(deps.storage)?;

    if state.charity_address != withdrawer {
        return Err(ContractError::NotAngelAddr {});
    } else if charity_pool.aust_amount.parse::<u128>().unwrap() == 0 {
        return Err(ContractError::NoBalance {});
    }

    let aust_withdraw_amount = charity_pool.aust_amount.parse::<u128>().unwrap();
    let withdraw_charity_balance = WasmMsg::Execute {
        contract_addr: state.aust_contract.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: withdrawer.clone().to_string(),
            amount: Uint128::from(aust_withdraw_amount),
        }).unwrap(),
        funds: Vec::new(),
    };

    ANGEL_INFO.save(deps.storage, &CharityPool { aust_amount: String::from("0") })?;

    Ok(Response::new()
        .add_message(withdraw_charity_balance)
        .add_attribute("method", "execute")
    )
}

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
