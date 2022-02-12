use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, CanonicalAddr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub pool_address: CanonicalAddr,
    pub round_time: u64,
    pub limit_time: u64,
    pub denom: String,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub round: u64,
}

pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Prediction {
    pub up: Uint128,
    pub down: Uint128,
    pub locked_price: Uint128,
    pub closing_time: u64,
    pub expire_time: u64,
    pub success: bool,
    pub is_up: Option<bool>,
}

pub const PREDICTIONS: Map<&[u8], Prediction> = Map::new("predictions");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Game {
    pub up: Uint128,
    pub down: Uint128,
    pub prize: Uint128,
    pub resolved: bool,
}

pub const GAMES: Map<(&[u8], &[u8]), Game> = Map::new("games");
