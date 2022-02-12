use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Wrong denom")]
    WrongDenom {},

    #[error("Multiple denom not allowed")]
    MultipleDenoms {},

    #[error("Prediction are still in progress")]
    PredictionStillInProgress {},

    #[error("No funds detected")]
    EmptyFunds {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
