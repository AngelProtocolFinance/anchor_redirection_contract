use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub escrow_contract: String,
    pub charity_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Pool {
    pub give_percentage: String,
    pub ust_amount: String,
    pub aust_amount: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CharityPool {
    pub aust_amount: String,
}

pub const STATE: Item<State> = Item::new("state");
pub const ANGEL_INFO: Item<CharityPool> = Item::new("angel_pool");
pub const USER_INFO: Map<&str, Pool> = Map::new("user_pool");