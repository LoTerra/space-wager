use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub pool_address: String,
    pub round_time: u64,
    pub limit_time: u64,
    pub denom: String,
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
