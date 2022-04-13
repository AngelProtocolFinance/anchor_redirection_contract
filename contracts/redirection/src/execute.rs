use crate::{
    msg::ExecuteMsg,
    state::{Pool, CONFIG, USER_INFO, Config},
    ContractError, helpers::check_funds,
};
use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env,
    MessageInfo, ReplyOn, Response,
    SubMsg, Uint128, WasmMsg, Addr,
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
            info.sender,
            percentage,
            ust_sent,
        )
    } else {
        let user_info = USER_INFO.load(deps.storage, info.sender.as_str())?;
        let aust_amount = user_info.aust_amount;
        if aust_amount == 0 {
            make_new_deposit(
                env,
                info.sender,
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
                info.sender,
                percentage,
                ust_sent,
                user_info,
            )
        } else {
            update_deposit(
                env,
                ust_sent,
                info.sender,
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

    withdraw_deposit(deps, env, amount, depositor)
}

pub fn make_new_deposit(
    env: Env,
    depositor: Addr,
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
    depositor: Addr,
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
    new_user_info.aust_amount = 0u64;
    new_user_info.ust_amount = 0u64;

    USER_INFO.save(deps.storage, &depositor.to_string(), &new_user_info)?;

    Ok(Response::new()
        .add_message(WasmMsg::Execute {
        contract_addr: config.aust_token_address.to_string(), // Should not hardcode this! Move to config.
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: config.charity_address.to_string(),
            amount: Uint128::from(user_info.aust_amount),
        })
        .unwrap(),
        funds: Vec::new(),
        })
        .add_submessage(escrow_execute))
}

pub fn update_deposit(
    env: Env,
    ust_sent: Uint128,
    depositor: Addr,
    percentage: u16,
    aust_amount: u64,
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

pub fn withdraw_deposit(
    deps: DepsMut,
    env: Env,
    withdraw_amount: Uint128,
    depositor: Addr,
) -> Result<Response, ContractError> {
    let user_info = USER_INFO.load(deps.storage, &depositor.to_string())?;
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