#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, Game, State, CONFIG, GAMES, STATE};

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
        ExecuteMsg::Resolve { address, round } => {
            try_collect_prize(deps, env, info, address, round)
        }
        ExecuteMsg::FinishAndStartNewRound {} => try_finish_and_start_new_round(deps, env, info),
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
pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if info.sender != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.count = count;
        Ok(state)
    })?;
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
