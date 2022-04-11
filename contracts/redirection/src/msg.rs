use cosmwasm_std::{Uint128, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    pub charity_address: Addr,
    pub anchor_market_address: Addr,
    pub aust_token_address: Addr,
    pub theta: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig(Config),
    DepositPool { percentage: u16 },
    WithdrawPool { withdraw_amount: Uint128 },
    InternalDepositInitial {
        ust_sent: Uint128,
        percentage: u16,
        depositor: String,
    },
    InternalDepositMore {
        ust_sent: Uint128,
        aust_amount: String,
        percentage: u16,
        depositor: String,
    },
    InternalSwapBackUpdate {
        to_angel: u64,
        charity_address: String,
        ust_amount: u64,
        new_percentage: u64,
        depositor: String,
    },
    InternalWithdrawInitial {
        withdraw_amount: Uint128,
        aust_amount: String,
        ust_amount: String,
        percentage: String,
        depositor: String,
    },
    InternalWithdrawSend {
        withdraw_amount: u64,
        new_ust_amount: u64,
        to_angel_amount: u64,
        ust_depositor: String,
        charity_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnchorExecuteMsg {
    DepositStable {},
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    RedeemStable {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    DepositInfo { address: String },
}
