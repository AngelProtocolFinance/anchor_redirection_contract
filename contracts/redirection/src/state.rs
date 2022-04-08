use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: String,
    pub escrow_controller: String,
    pub charity_address: String,
    pub theta: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Pool {
    pub give_percentage: String,
    pub ust_amount: String,
    pub aust_amount: String,
}

pub const CONFIG: Item<Config> = Item::new("state");
pub const USER_INFO: Map<&str, Pool> = Map::new("user_pool");