use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub collector_address: String,
    pub round_time: u64,
    pub limit_time: u64,
    pub denom: String,
    pub collector_fee: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Make a prediction on the current round
    MakePrediction { up: bool },
    /// Resolve will collect prize or refund if prediction fail
    ResolveGame { address: String, round: Vec<u64> },
    /// Finish round will start a new round
    ResolvePrediction {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Retrieve the state
    State {},
    /// Retrieve the config
    Config {},
    /// Retrieve game of an address and round
    Game { address: String, round: u64 },
    /// Retrieve a prediction for info
    Prediction { round: u64 },
    /// Retrieve all predictions for info
    Predictions {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Query player stats
    Player { address: String },
    /// Retrieve all games
    Games {
        player: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub round: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub pool_address: String,
    pub collector_address: String,
    pub round_time: u64,
    pub limit_time: u64,
    pub denom: String,
    pub collector_fee: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameResponse {
    pub up: Uint128,
    pub down: Uint128,
    pub prize: Uint128,
    pub resolved: bool,
    pub game_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PredictionResponse {
    pub up: Uint128,
    pub down: Uint128,
    pub locked_price: Uint128,
    pub resolved_price: Uint128,
    pub closing_time: u64,
    pub expire_time: u64,
    pub success: Option<bool>,
    pub is_up: Option<bool>,
    pub cumulative_last1: Option<Uint128>,
    pub block_time1: Option<u64>,
    pub cumulative_last2: Option<Uint128>,
    pub block_time2: Option<u64>,
    pub prediction_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
    pub update_collector_address: String,
    pub update_pool_address: String,
    pub update_round_time: u64,
    pub update_limit_time: u64,
    pub update_denom: String,
    pub update_collector_fee: Decimal,
}

// Astroport
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AstroportQueryMsg {
    CumulativePrices {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CumulativePricesResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
    pub price0_cumulative_last: Uint128,
    pub price1_cumulative_last: Uint128,
}
