use crate::{
    msg::AnchorExecuteMsg,
    state::CONFIG,
    ContractError, helpers::get_convert_to_ust,
};
use cosmwasm_std::{
    coin, to_binary, DepsMut, Env, 
    MessageInfo, Response, Uint128, 
    WasmMsg, BankMsg, Addr,
};

pub fn deposit_initial(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ust_sent: Uint128,
    percentage: u16,
    depositor: Addr,
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
    aust_amount: u64,
    percentage: u16,
    depositor: Addr,
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
    charity_address: Addr,
    ust_amount: u64,
    new_percentage: u16,
    depositor: Addr,
) -> Result<Response, ContractError> {
    if info.sender.ne(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    };

    let config = CONFIG.load(deps.storage)?;

    let mut res = Response::new()
        .add_attribute("to_angel", to_angel.to_string())
        .add_attribute("new_percentage", new_percentage.to_string())
        .add_attribute("ust_depositor", depositor);
    // if going to an Angel Charity add the bank msg
    if to_angel != 0 {
        res = res.add_message(BankMsg::Send {
            to_address: charity_address.to_string(),
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
    aust_amount: u64,
    ust_amount: u64,
    percentage: u16,
    depositor: Addr,
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
        aust_amount
    );

    Ok(Response::new()
        .add_attribute("withdraw_amount", withdraw_amount)
        .add_attribute("percentage", percentage.to_string())
        .add_attribute("ust_depositor", depositor)
        .add_attribute("ust_amount", ust_amount.to_string())
        .add_attribute("aust_amount", aust_amount.to_string())
        .add_message(convert_to_ust))
}

pub fn withdraw_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdraw_amount: u64,
    new_ust_amount: u64,
    to_angel_amount: u64,
    ust_depositor: Addr,
    charity_address: Addr,
) -> Result<Response, ContractError> {
    if info.sender.ne(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    };

    let config = CONFIG.load(deps.storage)?;

    let withdraw_to_user = BankMsg::Send {
        to_address: ust_depositor.to_string(),
        amount: vec![coin(withdraw_amount.into(), "uusd")],
    };
    let send_to_charity = BankMsg::Send {
        to_address: charity_address.to_string(),
        amount: vec![coin(to_angel_amount.into(), "uusd")],
    };
    let anchor_deposit = WasmMsg::Execute {
        contract_addr: config.anchor_market_address.to_string(),
        msg: to_binary(&AnchorExecuteMsg::DepositStable {})?,
        funds: vec![coin(new_ust_amount.into(), "uusd")],
    };

    let mut res = Response::new()
        .add_attribute("to_angel", to_angel_amount.to_string())
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