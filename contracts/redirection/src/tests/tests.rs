use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{attr, coins, CosmosMsg, Addr, Api};

use crate::msg::QueryMsg;
use crate::query::query;

#[test]
fn proper_initialization() {
    use crate::{msg::InstantiateMsg, contract::instantiate};
    let mut deps = mock_dependencies(&[]);
    let sample_addr = Addr::unchecked("terra1757tkx08n0cqrw7p86ny9lnxsqeth0wgp0em95");
    let instantiate_msg = InstantiateMsg {
        admin: sample_addr.clone(),
        charity_address: sample_addr.clone(),
        anchor_market_address: Addr::unchecked("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
        aust_token_address: Addr::unchecked("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
        theta: 1000,
    };
    let info = mock_info(sample_addr.as_ref(), &[]);
    let result = 
    instantiate(deps.as_mut(), mock_env(), info, instantiate_msg.clone()).unwrap();
    assert_eq!(0, result.messages.len());
}