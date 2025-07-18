use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub collector_address: String,
    pub round_time: u64,
    pub limit_time: u64,
    pub denom: String,
    pub collector_ratio: Decimal,
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
    // /// Retrieve all games
    // Games { start_after: Option<u64>, limit: Option<u64> },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OraclePriceFeedQueryMsg {
    State{},
    GetListPriceFeed {
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
    pub round_time: u64,
    pub limit_time: u64,
    pub denom: String,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OraclePriceFeedStateResponse {
    pub pool_address: String,
    pub round: u64,
    pub denom_one: String,
    pub denom_two: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OraclePriceFeedResponse {
    pub timestamp: u64,
    pub price: Uint128,
    pub worker: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleListPriceFeedResponse {
    pub list: Vec<OraclePriceFeedResponse>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PredictionInfo {
    pub up: Uint128,
    pub down: Uint128,
    pub locked_price: Uint128,
    pub resolved_price: Uint128,
    pub closing_time: u64,
    pub expire_time: u64,
    pub success: bool,
    pub is_up: Option<bool>,
    pub oracle_price_worker: Option<String>
}
