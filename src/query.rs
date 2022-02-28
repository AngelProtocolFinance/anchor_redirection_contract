#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, Env, StdResult
};
use crate::{state::{USER_INFO, Pool, CharityPool, ANGEL_INFO}, msg::QueryMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DepositInfo { address } => to_binary(&get_deposit_info(deps, address)?),
        QueryMsg::AngelInfo {} => to_binary(&get_angel_info(deps)?),
    }
}

pub fn get_deposit_info(
    deps: Deps,
    address: String,
) -> StdResult<Pool> {
    let deposit_info = USER_INFO.load(deps.storage, &address)?;
    Ok(deposit_info)
}

pub fn get_angel_info(
    deps: Deps,
) -> StdResult<CharityPool> {
    Ok(ANGEL_INFO.load(deps.storage)?)
}