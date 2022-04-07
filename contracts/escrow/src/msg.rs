use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    pub redirection_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig(UpdateConfigMsg),
    SendDust {
        charity_address: String,
        aust_amount: u64,
    },
    DepositInitial {
        percentage: u16,
        depositor: String,
    },
    DepositMore {
        ust_sent: Uint128,
        aust_amount: String,
        percentage: u16,
        depositor: String,
    },
    SwapBackUpdate {
        to_angel: bool,
        charity_address: String,
        ust_amount: u64,
        new_percentage: u64,
        depositor: String,
    },
    WithdrawInitial {
        withdraw_amount: Uint128,
        aust_amount: String,
        ust_amount: String,
        percentage: String,
        depositor: String,
    },
    WithdrawSend {
        withdraw_amount: u64,
        new_ust_amount: u64,
        to_angel_amount: u64,
        ust_depositor: String,
        charity_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateConfigMsg {
    pub redirection_contract: String,
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnchorExecuteMsg {
    DepositStable {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Return stable coins to a user
    /// according to exchange rate
    RedeemStable {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
