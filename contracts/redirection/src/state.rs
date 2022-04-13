use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub charity_address: Addr,
    pub anchor_market_address: Addr,
    pub aust_token_address: Addr,
    pub theta: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Pool {
    pub give_percentage: u16,
    pub ust_amount: u64,
    pub aust_amount: u64,
    pub total_donated: u64,
}

pub const CONFIG: Item<Config> = Item::new("state");
pub const USER_INFO: Map<&str, Pool> = Map::new("user_pool");
