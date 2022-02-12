#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmQuery,
};
use cw2::set_contract_version;
use terraswap::asset::AssetInfo;
use terraswap::pair::PoolResponse;

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg, TerraSwapQueryMsg};

use crate::state::{Config, Game, Prediction, State, CONFIG, GAMES, PREDICTIONS, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:space-wager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State { round: 0 };

    let config = Config {
        pool_address: deps.api.addr_canonicalize(msg.pool_address.as_str())?,
        round_time: msg.round_time,
        limit_time: msg.limit_time,
        denom: msg.denom,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MakePrediction { up } => try_make_prediction(deps, env, info, up),
        ExecuteMsg::ResolveGame { address, round } => {
            try_resolve_game(deps, env, info, address, round)
        }
        ExecuteMsg::ResolvePrediction {} => try_resolve_prediction(deps, env, info),
    }
}

pub fn try_make_prediction(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    up: bool,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let raw_sender = deps.api.addr_canonicalize(&info.sender.as_str())?;
    let sent = match info.funds.len() {
        0 => Err(ContractError::EmptyFunds {}),
        1 => {
            if info.funds[0].denom != config.denom {
                return Err(ContractError::WrongDenom {});
            }
            Ok(info.funds[0].amount)
        }
        _ => Err(ContractError::MultipleDenoms {}),
    }?;

    match GAMES.may_load(
        deps.storage,
        (&raw_sender.as_slice(), &state.round.to_be_bytes()),
    )? {
        None => {
            if up {
                GAMES.save(
                    deps.storage,
                    (&raw_sender.as_slice(), &state.round.to_be_bytes()),
                    &Game {
                        up: sent,
                        down: Uint128::zero(),
                        prize: Uint128::zero(),
                        resolved: false,
                    },
                )
            } else {
                GAMES.save(
                    deps.storage,
                    (&raw_sender.as_slice(), &state.round.to_be_bytes()),
                    &Game {
                        up: Uint128::zero(),
                        down: sent,
                        prize: Uint128::zero(),
                        resolved: false,
                    },
                )
            }
        }
        Some(_) => {
            if up {
                GAMES.update(
                    deps.storage,
                    (&raw_sender.as_slice(), &state.round.to_be_bytes()),
                    |game| -> Result<Game, ContractError> {
                        let mut update_game = game?;
                        update_game.up += sent;
                        Ok(update_game)
                    },
                )
            } else {
                GAMES.update(
                    deps.storage,
                    (&raw_sender.as_slice(), &state.round.to_be_bytes()),
                    |game| -> Result<Game, ContractError> {
                        let mut update_game = game?;
                        update_game.down += sent;
                        Ok(update_game)
                    },
                )
            }
        }
    };

    Ok(Response::new().add_attribute("action", "make_prediction"))
}

pub fn try_resolve_game(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    round: Vec<u64>,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn try_resolve_prediction(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let prediction = PREDICTIONS.load(deps.storage, &state.round.to_be_bytes())?;

    // Check if the round is open to be resolved
    if prediction.closing_time > env.block.time.seconds() {
        return Err(ContractError::PredictionStillInProgress {});
    }

    /*
        TODO: Probably good to cancel the auction if nobody have played
     */

    //Query the pool LUNA-UST Terraswap Calculate the current price pool
    let pool_info_msg = terraswap::pair::QueryMsg::Pool {};
    let query = WasmQuery::Smart {
        contract_addr: deps.api.addr_humanize(&config.pool_address)?.to_string(),
        msg: to_binary(&pool_info_msg)?,
    };
    let pool_info: PoolResponse = deps.querier.query(&query.into())?;

    let luna_asset = pool_info
        .assets
        .iter()
        .find(|&a| match a.info {
            AssetInfo::Token { .. } => false,
            AssetInfo::NativeToken { .. } => {
                if denom == "uluna" {
                    true
                }
            }
        })
        .unwrap();

    let ust_asset = pool_info
        .assets
        .iter()
        .find(|&a| match a.info.clone() {
            AssetInfo::Token { .. } => false,
            AssetInfo::NativeToken { denom } => {
                if denom == "uusd" {
                    true
                }
            }
        })
        .unwrap();

    let predicted_price =
        Uint128::from(1_000_000_u128).multiply_ratio(ust_asset.amount, luna_asset.amount);

    // Update the current prediction
    let is_success = env.block.time.seconds() > prediction.expire_time;
    let is_up = predicted_price > prediction.locked_price;

    PREDICTIONS.update(
        deps.storage,
        &state.round.to_be_bytes(),
        |prediction| -> Result<_, ContractError> {
            let mut update_prediction = prediction?;
            update_prediction.is_up = Some(is_up);
            update_prediction.success = is_success;
            Ok(update_prediction)
        },
    )?;

    // Increment the round
    state.round += 1;
    STATE.save(deps.storage, &state)?;

    // Create a new prediction with incremented round
    PREDICTIONS.save(
        deps.storage,
        &state.round.to_be_bytes(),
        &Prediction {
            up: Uint128::zero(),
            down: Uint128::zero(),
            locked_price: predicted_price,
            closing_time: env.block.time.plus_seconds(config.round_time).seconds(),
            expire_time: env
                .block
                .time
                .plus_seconds(config.round_time)
                .plus_seconds(config.limit_time)
                .seconds(),
            success: false,
            is_up: None,
        },
    )?;

    Ok(Response::new().add_attribute("method", "reset"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
    }
}

fn query_count(deps: Deps) -> StdResult<CountResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(CountResponse { count: state.count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 5
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }
}
