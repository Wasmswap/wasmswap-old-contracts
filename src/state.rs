use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: Addr,
    pub nativeSupply: Coin,

    pub tokenAddress: Addr,
    pub tokenSupply: Uint128,
}

pub const STATE: Item<State> = Item::new("state");
