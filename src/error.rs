use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;
use cw20_base;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Cw20Error(#[from] cw20_base::ContractError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    #[error("MinLiquidityError")]
    MinLiquidityError { min_liquidity: Uint128, liquidity_available: Uint128},

    #[error("MaxTokenError")]
    MaxTokenError { max_token: Uint128, tokens_required: Uint128},
}
