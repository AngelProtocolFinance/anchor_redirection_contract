#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute::{
    deposit_initial, deposit_more, send_dust, swap_aust_ust, swap_back_aust, update_config,
    withdraw_send,
};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:aust-swapper";
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
            redirection_contract: deps.api.addr_validate(&msg.redirection_contract)?,
        },
    )?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig(msg) => update_config(deps, info, msg),
        ExecuteMsg::SendDust {
            charity_address,
            aust_amount,
        } => send_dust(deps, info, charity_address, aust_amount),
        ExecuteMsg::DepositInitial {
            percentage,
            depositor,
        } => deposit_initial(deps, info, percentage, depositor),
        ExecuteMsg::DepositMore {
            ust_sent,
            aust_amount,
            percentage,
            depositor,
        } => deposit_more(deps, info, ust_sent, aust_amount, percentage, depositor),
        ExecuteMsg::SwapBackUpdate {
            to_angel,
            charity_address,
            ust_amount,
            new_percentage,
            depositor,
        } => swap_back_aust(
            deps,
            info,
            to_angel,
            charity_address,
            ust_amount,
            new_percentage,
            depositor,
        ),
        ExecuteMsg::WithdrawInitial {
            withdraw_amount,
            aust_amount,
            ust_amount,
            percentage,
            depositor,
        } => swap_aust_ust(
            deps,
            info,
            withdraw_amount,
            aust_amount,
            ust_amount,
            percentage,
            depositor,
        ),
        ExecuteMsg::WithdrawSend {
            withdraw_amount,
            new_ust_amount,
            to_angel_amount,
            ust_depositor,
            charity_address,
        } => withdraw_send(
            deps,
            info,
            withdraw_amount,
            new_ust_amount,
            to_angel_amount,
            ust_depositor,
            charity_address,
        ),
    }
}

#[cfg(test)]
mod tests {}
