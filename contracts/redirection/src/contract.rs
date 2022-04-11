#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Reply, Response, Uint128};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute::{
    check_funds, deposit_then_update_user, get_new_user_state_dep, get_new_user_state_wit,
    make_new_deposit, make_new_user_struct, send_dust_to_angel_then_make_new_deposit,
    update_config, update_deposit, withdraw_deposit, withdraw_then_update_user, deposit_initial, deposit_more, swap_back_aust, swap_aust_ust, withdraw_send,
};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Config, CONFIG, USER_INFO};

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            admin: deps.api.addr_validate(&msg.admin)?,
            charity_address: deps.api.addr_validate(&msg.charity_address.to_string())?,
            anchor_market_address: deps.api.addr_validate(&msg.anchor_market_address.to_string())?,
            aust_token_address: deps.api.addr_validate(&msg.aust_token_address.to_string())?,
            theta: msg.theta,
        },
    )?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        0 => make_new_user_struct(deps, env, msg.result),
        1 => get_new_user_state_dep(deps, env, msg.result),
        2 => deposit_then_update_user(deps, env, msg.result),
        3 => get_new_user_state_wit(deps, env, msg.result),
        4 => withdraw_then_update_user(deps, env, msg.result),
        _ => Err(ContractError::Unauthorized {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig(msg) => update_config(deps, info, msg),
        ExecuteMsg::DepositPool { percentage } => deposit_pool(deps, env, info, percentage),
        ExecuteMsg::WithdrawPool { withdraw_amount } => withdraw_pool(deps, env, info, withdraw_amount),
        ExecuteMsg::InternalDepositInitial {
            ust_sent,
            percentage,
            depositor,
        } => deposit_initial(deps, env, info, ust_sent, percentage, depositor),
        ExecuteMsg::InternalDepositMore {
            ust_sent,
            aust_amount,
            percentage,
            depositor,
        } => deposit_more(deps, env, info, ust_sent, aust_amount, percentage, depositor),
        ExecuteMsg::InternalSwapBackUpdate {
            to_angel,
            charity_address,
            ust_amount,
            new_percentage,
            depositor,
        } => swap_back_aust(
            deps,
            env,
            info,
            to_angel,
            charity_address,
            ust_amount,
            new_percentage,
            depositor,
        ),
        ExecuteMsg::InternalWithdrawInitial {
            withdraw_amount,
            aust_amount,
            ust_amount,
            percentage,
            depositor,
        } => swap_aust_ust(
            deps,
            env,
            info,
            withdraw_amount,
            aust_amount,
            ust_amount,
            percentage,
            depositor,
        ),
        ExecuteMsg::InternalWithdrawSend {
            withdraw_amount,
            new_ust_amount,
            to_angel_amount,
            ust_depositor,
            charity_address,
        } => withdraw_send(
            deps,
            env,
            info,
            withdraw_amount,
            new_ust_amount,
            to_angel_amount,
            ust_depositor,
            charity_address,
        ),
    }
}

pub fn deposit_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    percentage: u16,
) -> Result<Response, ContractError> {
    if percentage < 5 || percentage > 100 {
        return Err(ContractError::WrongPercentageInput {});
    };

    let ust_sent = check_funds(&info)?;
    if ust_sent.u128() < 1000 {
        return Err(ContractError::MakeNewPoolError {});
    };

    let config = CONFIG.load(deps.storage)?;

    // If no user exists, create a new deposit for them
    if !USER_INFO.has(deps.storage, info.sender.as_str()) {
        make_new_deposit(
            env,
            info.sender.to_string(),
            percentage,
            ust_sent,
        )
    } else {
        let user_info = USER_INFO.load(deps.storage, info.sender.as_str())?;
        let aust_amount = user_info.aust_amount.parse::<u64>().unwrap();
        if aust_amount == 0 {
            make_new_deposit(
                env,
                info.sender.to_string(),
                percentage,
                ust_sent,
            )
        } else if aust_amount <= config.theta {
            /*
             * Theta: Should be capped around 0.001 aUST.
             * When a user withdraws, it leaves tiny bits of dust
             * Triggering update deposit over < 0.001 aUST balance is a waste of gas
             * Added to save fees and keep escrow aUST balance as clean as possible.
             */
            send_dust_to_angel_then_make_new_deposit(
                deps,
                env,
                info.sender.to_string(),
                percentage,
                ust_sent,
                user_info,
            )
        } else {
            update_deposit(
                env,
                ust_sent,
                info.sender.to_string(),
                percentage,
                user_info.aust_amount,
            )
        }
    }
}

pub fn withdraw_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let depositor = deps.api.addr_validate(&info.sender.as_str())?;
    if !USER_INFO.has(deps.storage, depositor.as_str()) {
        return Err(ContractError::NoDeposit {});
    }

    withdraw_deposit(deps, env, amount, depositor.to_string())
}

#[cfg(test)]
mod tests {}
