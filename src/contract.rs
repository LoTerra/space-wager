#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdResult, SubMsg, Uint128, WasmQuery,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use std::convert::TryInto;
use std::ops::{Add, Mul, Sub};
use terraswap::asset::AssetInfo;
use terraswap::pair::PoolResponse;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, OracleListPriceFeedResponse, OraclePriceFeedQueryMsg, OraclePriceFeedResponse, OraclePriceFeedStateResponse, QueryMsg, StateResponse};

use crate::state::{Config, Game, Prediction, State, CONFIG, GAMES, PREDICTIONS, STATE};
use crate::taxation::deduct_tax;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:space-wager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State { round: 0 };

    let config = Config {
        pool_address: deps.api.addr_canonicalize(msg.pool_address.as_str())?,
        collector_address: deps.api.addr_canonicalize(msg.collector_address.as_str())?,
        round_time: msg.round_time,
        limit_time: msg.limit_time,
        denom: msg.denom,
        collector_fee: msg.collector_ratio,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    CONFIG.save(deps.storage, &config)?;

    PREDICTIONS.save(
        deps.storage,
        &state.round.to_be_bytes(),
        &Prediction {
            up: Uint128::zero(),
            down: Uint128::zero(),
            locked_price: Uint128::zero(),
            resolved_price: Uint128::zero(),
            closing_time: env.block.time.plus_seconds(msg.round_time).seconds(),
            expire_time: env
                .block
                .time
                .plus_seconds(msg.round_time)
                .plus_seconds(msg.round_time)
                .plus_seconds(msg.limit_time)
                .seconds(),
            success: false,
            is_up: None,
            oracle_price_worker: None
        },
    )?;

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
    _env: Env,
    info: MessageInfo,
    up: bool,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
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
            let game = if up {
                Game {
                    up: sent,
                    down: Uint128::zero(),
                    prize: Uint128::zero(),
                    resolved: false,
                }
            } else {
                Game {
                    up: Uint128::zero(),
                    down: sent,
                    prize: Uint128::zero(),
                    resolved: false,
                }
            };
            GAMES.save(
                deps.storage,
                (&raw_sender.as_slice(), &state.round.to_be_bytes()),
                &game,
            )?;
        }
        Some(_) => {
            GAMES.update(
                deps.storage,
                (&raw_sender.as_slice(), &state.round.to_be_bytes()),
                |game| -> Result<Game, ContractError> {
                    let mut update_game = game.unwrap();
                    if up {
                        update_game.up += sent;
                    } else {
                        update_game.down += sent;
                    }
                    Ok(update_game)
                },
            )?;
        }
    };

    PREDICTIONS.update(
        deps.storage,
        &state.round.to_be_bytes(),
        |prediction| -> Result<_, ContractError> {
            let mut update_prediction = prediction.unwrap();
            if up {
                update_prediction.up += sent;
            } else {
                update_prediction.down += sent
            }
            Ok(update_prediction)
        },
    )?;

    let direction = match up {
        true => "up",
        false => "down",
    };

    Ok(Response::new()
        .add_attribute("action", "make_prediction")
        .add_attribute("entered", direction.to_string())
        .add_attribute("committed", sent.to_string())
        .add_attribute("prediction_id", state.round.to_string()))
}

pub fn try_resolve_game(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    address: String,
    round: Vec<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let raw_address = deps.api.addr_canonicalize(&address)?;
    let mut prize_amount = Uint128::zero();
    let mut refund_amount = Uint128::zero();
    //let fee = Uint128::from(1).checked_sub(config.collector_fee);
    let fee = Decimal::one().sub(config.collector_fee);

    for round_number in round {
        let prediction = PREDICTIONS.load(deps.storage, &round_number.to_be_bytes())?;
        if prediction.expire_time > env.block.time.seconds() {
            return Err(ContractError::PredictionStillInProgress {});
        }

        let game = GAMES.load(deps.storage, (&raw_address, &round_number.to_be_bytes()))?;
        if game.resolved {
            return Err(ContractError::AlreadyResolved {});
        }

        let mut round_prize = Uint128::zero();
        if prediction.success {
            if let Some(is_up) = prediction.is_up {
                if is_up {
                    if !game.up.is_zero() {
                        let up_ratio = Decimal::from_ratio(
                            prediction.up.checked_add(prediction.down).unwrap(),
                            prediction.up,
                        );
                        let payout = game.up.mul(up_ratio);
                        round_prize = payout;
                        prize_amount += payout;
                    }
                } else {
                    if !game.down.is_zero() {
                        let down_ratio = Decimal::from_ratio(
                            prediction.up.checked_add(prediction.down).unwrap(),
                            prediction.down,
                        );
                        let payout = game.down.mul(down_ratio);
                        round_prize = payout;
                        prize_amount += payout;
                    }
                }
            }
        } else {
            round_prize = game.down.checked_add(game.up).unwrap();
            // Refund
            refund_amount += game.down.checked_add(game.up).unwrap();
        }

        // Update game as resolved
        GAMES.update(
            deps.storage,
            (&raw_address, &round_number.to_be_bytes()),
            |game| -> Result<_, ContractError> {
                let mut update_game = game.unwrap();
                update_game.resolved = true;
                update_game.prize = round_prize;
                Ok(update_game)
            },
        )?;
    }

    let mut collector_fee = Uint128::zero();
    let net_prize_amount = if !prize_amount.is_zero() {
        let net_amount = prize_amount.mul(fee);
        collector_fee = prize_amount.wrapping_sub(net_amount);
        net_amount
    } else {
        prize_amount
    };
    let final_amount = net_prize_amount.add(refund_amount);

    let mut res = Response::new();
    if !final_amount.is_zero() {
        let exec_msg_prize = CosmosMsg::Bank(BankMsg::Send {
            to_address: address.clone(),
            amount: vec![deduct_tax(
                &deps.querier,
                Coin {
                    denom: config.denom.clone(),
                    amount: final_amount,
                },
            )?],
        });
        res.messages.push(SubMsg::new(exec_msg_prize));
    }

    if !collector_fee.is_zero() {
        let exec_msg_collector_fee = CosmosMsg::Bank(BankMsg::Send {
            to_address: deps
                .api
                .addr_humanize(&config.collector_address)?
                .to_string(),
            amount: vec![deduct_tax(
                &deps.querier,
                Coin {
                    denom: config.denom,
                    amount: collector_fee,
                },
            )?],
        });

        res.messages.push(SubMsg::new(exec_msg_collector_fee));
    }

    res.attributes
        .push(Attribute::new("action", "resolve_game"));
    res.attributes.push(Attribute::new("recipient", address));
    Ok(res)
}

const MAX_LIMIT_QUERY: u32 = 10;
pub fn try_resolve_prediction(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let prediction_now = PREDICTIONS.load(deps.storage, &state.round.to_be_bytes())?;

    // Check if the round is open to be resolved
    if prediction_now.closing_time > env.block.time.seconds() {
        return Err(ContractError::PredictionStillInProgress {});
    }
    let query_state_price_feed = OraclePriceFeedQueryMsg::State {};
    let query_wasm_state = WasmQuery::Smart { contract_addr: deps.api.addr_humanize(&config.pool_address)?.to_string(), msg: to_binary(&query_state_price_feed)?};
    let state_info: OraclePriceFeedStateResponse =  deps.querier.query(&query_wasm_state.into())?;

    let query_list_price_feed = OraclePriceFeedQueryMsg::GetListPriceFeed { start_after: Some(state_info.round), limit: Some(MAX_LIMIT_QUERY) };
    let query_wasm_list_price_feed = WasmQuery::Smart { contract_addr: deps.api.addr_humanize(&config.pool_address)?.to_string(), msg: to_binary(&query_list_price_feed)?};
    let list_price_feed_info: OracleListPriceFeedResponse = deps.querier.query(&query_wasm_list_price_feed.into())?;

    let mut data_price_feed = list_price_feed_info.list.iter().filter(
        |&price_feed| price_feed.timestamp > env.block.time.seconds()
    ).collect::<Vec<OraclePriceFeedResponse>>();
    data_price_feed.sort_by(|a, b| a.price.cmp(&b.price));

    let data_price_feed_average = data_price_feed[5].clone();

    // //Query the pool LUNA-UST Terraswap Calculate the current price pool
    // let pool_info_msg = terraswap::pair::QueryMsg::Pool {};
    // let query = WasmQuery::Smart {
    //     contract_addr: deps.api.addr_humanize(&config.pool_address)?.to_string(),
    //     msg: to_binary(&pool_info_msg)?,
    // };
    // let pool_info: PoolResponse = deps.querier.query(&query.into())?;
    //
    // let luna_asset = pool_info
    //     .assets
    //     .iter()
    //     .find(|&a| match a.info.clone() {
    //         AssetInfo::Token { .. } => false,
    //         AssetInfo::NativeToken { denom } => denom == "uluna",
    //     })
    //     .unwrap();
    //
    // let ust_asset = pool_info
    //     .assets
    //     .iter()
    //     .find(|&a| match a.info.clone() {
    //         AssetInfo::Token { .. } => false,
    //         AssetInfo::NativeToken { denom } => denom == "uusd",
    //     })
    //     .unwrap();
    //
    // let predicted_price =
    //     Uint128::from(1_000_000_u128).multiply_ratio(ust_asset.amount, luna_asset.amount);

    let predicted_price = data_price_feed_average.price;

    let mut res = Response::new();
    // Resolve the past prediction
    if state.round != 0 {
        let prediction = PREDICTIONS.load(deps.storage, &(state.round - 1).to_be_bytes())?;
        // Check if not expired and prediction up and down are not zero
        let is_success = env.block.time.seconds() < prediction.expire_time
            && !prediction.up.is_zero()
            && !prediction.down.is_zero()
            && prediction.locked_price != predicted_price;

        let is_up = predicted_price > prediction.locked_price;
        // Update the current prediction
        PREDICTIONS.update(
            deps.storage,
            &(state.round - 1).to_be_bytes(),
            |prediction| -> Result<_, ContractError> {
                let mut update_prediction = prediction.unwrap();
                if is_success {
                    update_prediction.is_up = Some(is_up);
                    update_prediction.resolved_price = predicted_price;
                }
                update_prediction.success = is_success;
                Ok(update_prediction)
            },
        )?;

        let direction = match is_up {
            true => "up",
            false => "down",
        };
        res.attributes.push(Attribute::new(
            "prediction_id",
            (state.round - 1).to_string(),
        ));

        res.attributes.push(Attribute::new(
            "locked_price",
            prediction.locked_price.to_string(),
        ));
        res.attributes
            .push(Attribute::new("is_success", is_success.to_string()));

        if is_success {
            res.attributes
                .push(Attribute::new("resolved", direction.to_string()));
            res.attributes.push(Attribute::new(
                "resolved_price",
                predicted_price.to_string(),
            ));
        }
    }
    res.attributes
        .push(Attribute::new("action", "resolve_prediction"));

    // Update locked price of the current prediction
    PREDICTIONS.update(
        deps.storage,
        &state.round.to_be_bytes(),
        |prediction| -> Result<_, ContractError> {
            let mut update_prediction = prediction.unwrap();
            update_prediction.locked_price = predicted_price;
            update_prediction.oracle_price_worker = Some(deps.api.addr_canonicalize(data_price_feed_average.worker.as_str())?);
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
            locked_price: Uint128::zero(),
            resolved_price: Uint128::zero(),
            closing_time: env.block.time.plus_seconds(config.round_time).seconds(),
            expire_time: env
                .block
                .time
                .plus_seconds(config.round_time)
                .plus_seconds(config.round_time)
                .plus_seconds(config.limit_time)
                .seconds(),
            success: false,
            is_up: None,
            oracle_price_worker: None
        },
    )?;

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Game { address, round } => to_binary(&query_game(deps, address, round)?),
        QueryMsg::Prediction { round } => to_binary(&query_prediction(deps, round)?),
        QueryMsg::Predictions { start_after, limit } => {
            to_binary(&query_predictions(deps, start_after, limit)?)
        }
    }
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { round: state.round })
}
fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        pool_address: deps.api.addr_humanize(&config.pool_address)?.to_string(),
        round_time: config.round_time,
        limit_time: config.limit_time,
        denom: config.denom,
    })
}
fn query_game(deps: Deps, address: String, round: u64) -> StdResult<Game> {
    let raw_address = deps.api.addr_canonicalize(&address)?;
    let game = GAMES.load(
        deps.storage,
        (&raw_address.as_slice(), &round.to_be_bytes()),
    )?;
    Ok(game)
}
fn query_prediction(deps: Deps, round: u64) -> StdResult<Prediction> {
    let prediction = PREDICTIONS.load(deps.storage, &round.to_be_bytes())?;
    Ok(prediction)
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;
fn query_predictions(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<(u64, Prediction)>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|d| Bound::Exclusive(d.to_be_bytes().to_vec()));

    let prediction = PREDICTIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|pair| {
            pair.and_then(|(k, prediction)| {
                Ok((u64::from_be_bytes(k.try_into().unwrap()), prediction))
            })
        })
        .collect::<StdResult<Vec<(u64, Prediction)>>>()?;
    Ok(prediction)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::mock_querier::mock_dependencies_custom;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coins, from_binary, Addr, Api, Attribute, Coin};
    use std::ops::Add;
    use std::str::FromStr;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_custom(&[]);
        deps.querier.pool_token(
            Uint128::new(10_250_000_000u128),
            Uint128::new(955_000_000u128),
        );
        let msg = InstantiateMsg {
            pool_address: "terraswap".to_string(),
            collector_address: "collector".to_string(),
            round_time: 300,
            limit_time: 30,
            denom: "uusd".to_string(),
            collector_ratio: Decimal::from_str("0.05").unwrap(),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let prediction = PREDICTIONS
            .load(deps.as_ref().storage, &0_u64.to_be_bytes())
            .unwrap();
        assert_eq!(prediction.down, Uint128::zero());
        assert_eq!(prediction.up, Uint128::zero());
        assert_eq!(
            prediction.closing_time,
            mock_env().block.time.plus_seconds(300).seconds()
        );
        assert_eq!(
            prediction.expire_time,
            mock_env()
                .block
                .time
                .plus_seconds(300)
                .plus_seconds(300)
                .plus_seconds(30)
                .seconds()
        );
        assert!(!prediction.success);
        assert_eq!(prediction.is_up, None);
        assert_eq!(prediction.locked_price, Uint128::zero());
        // it worked, let's query the state
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        // let value: CountResponse = from_binary(&res).unwrap();
        // assert_eq!(17, value.count);
    }

    #[test]
    fn proper_make_prediction() {
        let mut deps = mock_dependencies_custom(&[]);
        deps.querier.pool_token(
            Uint128::new(10_250_000_000u128),
            Uint128::new(955_000_000u128),
        );
        let msg = InstantiateMsg {
            pool_address: "terraswap".to_string(),
            collector_address: "collector".to_string(),
            round_time: 300,
            limit_time: 30,
            denom: "uusd".to_string(),
            collector_ratio: Decimal::from_str("0.05").unwrap(),
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Player1 Enter up
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info("player1", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::EmptyFunds {});

        let info = mock_info(
            "player1",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("action", "make_prediction"),
                Attribute::new("entered", "down"),
                Attribute::new("committed", "100000000"),
                Attribute::new("prediction_id", "0")
            ]
        );
        // Query the game
        let sender = deps
            .api
            .addr_canonicalize(Addr::unchecked("player1").as_str())
            .unwrap();
        let game = GAMES
            .load(
                deps.as_ref().storage,
                (&sender.as_slice(), &0_u64.to_be_bytes()),
            )
            .unwrap();
        assert_eq!(game.up, Uint128::zero());
        assert_eq!(game.down, Uint128::from(100_000_000u128));
        assert_eq!(game.resolved, false);
        assert_eq!(game.prize, Uint128::zero());

        // Player2 Enter up
        let msg = ExecuteMsg::MakePrediction { up: true };
        let info = mock_info(
            "player2",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("action", "make_prediction"),
                Attribute::new("entered", "up"),
                Attribute::new("committed", "500000000"),
                Attribute::new("prediction_id", "0")
            ]
        );

        // Query the game
        let sender = deps
            .api
            .addr_canonicalize(Addr::unchecked("player2").as_str())
            .unwrap();
        let game = GAMES
            .load(
                deps.as_ref().storage,
                (&sender.as_slice(), &0_u64.to_be_bytes()),
            )
            .unwrap();
        assert_eq!(game.up, Uint128::from(500_000_000u128));
        assert_eq!(game.down, Uint128::zero());
        assert_eq!(game.resolved, false);
        assert_eq!(game.prize, Uint128::zero());

        // Player2 Enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player2",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("action", "make_prediction"),
                Attribute::new("entered", "down"),
                Attribute::new("committed", "100000000"),
                Attribute::new("prediction_id", "0")
            ]
        );

        // Query the game
        let sender = deps
            .api
            .addr_canonicalize(Addr::unchecked("player2").as_str())
            .unwrap();
        let game = GAMES
            .load(
                deps.as_ref().storage,
                (&sender.as_slice(), &0_u64.to_be_bytes()),
            )
            .unwrap();
        assert_eq!(game.up, Uint128::from(500_000_000u128));
        assert_eq!(game.down, Uint128::from(100_000_000u128));
        assert_eq!(game.resolved, false);
        assert_eq!(game.prize, Uint128::zero());

        // Query prediction
        let prediction = PREDICTIONS
            .load(deps.as_ref().storage, &0_u64.to_be_bytes())
            .unwrap();
        assert_eq!(prediction.down, Uint128::from(200_000_000u128));
        assert_eq!(prediction.up, Uint128::from(500_000_000u128));
    }

    #[test]
    fn proper_resolve_prediction() {
        let mut deps = mock_dependencies_custom(&[]);
        deps.querier.pool_token(
            Uint128::new(10_250_000_000u128),
            Uint128::new(955_000_000u128),
        );

        let msg = InstantiateMsg {
            pool_address: "terraswap".to_string(),
            collector_address: "collector".to_string(),
            round_time: 300,
            limit_time: 30,
            denom: "uusd".to_string(),
            collector_ratio: Decimal::from_str("0.05").unwrap(),
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        deps.querier.pool_token(
            Uint128::new(15_250_000_000u128),
            Uint128::new(555_000_000u128),
        );
        // Player1 enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player1",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Player2 Enter up
        let msg = ExecuteMsg::MakePrediction { up: true };
        let info = mock_info(
            "player2",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Player2 Enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player2",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Resolve prediction
        let msg = ExecuteMsg::ResolvePrediction {};
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bot", &[]),
            msg.clone(),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::PredictionStillInProgress {});

        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(config.round_time);
        let res = execute(deps.as_mut(), env.clone(), mock_info("bot", &[]), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![Attribute::new("action", "resolve_prediction")]
        );
        // assert_eq!(
        //     res.attributes,
        //     vec![
        //         Attribute::new("action", "resolve_prediction"),
        //         Attribute::new("locked_price", "27477477"),
        //         Attribute::new("is_success", "true"),
        //         Attribute::new("prediction_id", "0"),
        //         Attribute::new("resolved", "up")
        //     ]
        // );

        /*
           Check state
        */
        let state = query_state(deps.as_ref()).unwrap();
        assert_eq!(state.round, 1);

        deps.querier.pool_token(
            Uint128::new(16_250_000_000u128),
            Uint128::new(455_000_000u128),
        );
        // Player1 enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player1",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        println!("{:?}", res);

        // Resolve
        env.block.time = env.block.time.plus_seconds(config.round_time);
        let msg = ExecuteMsg::ResolvePrediction {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("bot", &[]), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("prediction_id", "0"),
                Attribute::new("locked_price", "27477477"),
                Attribute::new("is_success", "true"),
                Attribute::new("resolved", "up"),
                Attribute::new("resolved_price", "35714285"),
                Attribute::new("action", "resolve_prediction")
            ]
        );

        deps.querier.pool_token(
            Uint128::new(56_250_000_000u128),
            Uint128::new(255_000_000u128),
        );
        // Player1 enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player1",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        let info = mock_info(
            "player2",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        println!("{:?}", res);

        // Resolve
        env.block.time = env.block.time.plus_seconds(config.round_time);
        let msg = ExecuteMsg::ResolvePrediction {};
        let res = execute(deps.as_mut(), env, mock_info("bot", &[]), msg).unwrap();
        println!("{:?}", res);
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("prediction_id", "1"),
                Attribute::new("locked_price", "35714285"),
                Attribute::new("is_success", "false"),
                Attribute::new("action", "resolve_prediction")
            ]
        );
    }

    #[test]
    fn proper_resolve_game() {
        let mut deps = mock_dependencies_custom(&[]);
        deps.querier.pool_token(
            Uint128::new(10_250_000_000u128),
            Uint128::new(955_000_000u128),
        );

        let msg = InstantiateMsg {
            pool_address: "terraswap".to_string(),
            collector_address: "collector".to_string(),
            round_time: 300,
            limit_time: 30,
            denom: "uusd".to_string(),
            collector_ratio: Decimal::from_str("0.05").unwrap(),
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        deps.querier.pool_token(
            Uint128::new(15_250_000_000u128),
            Uint128::new(555_000_000u128),
        );

        // Player1 enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player1",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Player2 Enter up
        let msg = ExecuteMsg::MakePrediction { up: true };
        let info = mock_info(
            "player2",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Player2 Enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player2",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::ResolvePrediction {};
        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(config.round_time);
        let res = execute(deps.as_mut(), env.clone(), mock_info("bot", &[]), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![Attribute::new("action", "resolve_prediction")]
        );

        let msg = ExecuteMsg::ResolveGame {
            address: "player1".to_string(),
            round: vec![0],
        };
        let err = execute(deps.as_mut(), env.clone(), mock_info("bot", &[]), msg).unwrap_err();
        assert_eq!(err, ContractError::PredictionStillInProgress {});

        // Resolve success
        deps.querier.pool_token(
            Uint128::new(1_250_000_000u128),
            Uint128::new(955_000_000u128),
        );
        let msg = ExecuteMsg::ResolvePrediction {};
        env.block.time = env.block.time.plus_seconds(config.round_time);
        let res = execute(deps.as_mut(), env.clone(), mock_info("bot", &[]), msg).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(config.round_time)
            .plus_seconds(config.limit_time);

        let msg = ExecuteMsg::ResolveGame {
            address: "player1".to_string(),
            round: vec![0],
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("bot", &[]),
            msg.clone(),
        )
        .unwrap();

        let msg_bank = BankMsg::Send {
            to_address: "player1".to_string(),
            amount: vec![Coin::new(331_500_000, "uusd")],
        };
        let msg_bank_fee = BankMsg::Send {
            to_address: "collector".to_string(),
            amount: vec![Coin::new(17_326_732, "uusd")],
        };

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Bank(msg_bank)),
                SubMsg::new(CosmosMsg::Bank(msg_bank_fee))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("action", "resolve_game"),
                Attribute::new("recipient", "player1")
            ]
        );

        // Handle resolve multiple times
        let err = execute(deps.as_mut(), env.clone(), mock_info("bot", &[]), msg).unwrap_err();
        assert_eq!(err, ContractError::AlreadyResolved {});

        /*
           New Prediction with failed and refund player 1
        */
        let state = STATE.load(deps.as_ref().storage).unwrap();
        println!("{:?}", state);

        deps.querier.pool_token(
            Uint128::new(15_250_000_000u128),
            Uint128::new(555_000_000u128),
        );

        // Player1 enter down
        let msg = ExecuteMsg::MakePrediction { up: false };
        let info = mock_info(
            "player1",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::ResolvePrediction {};
        //env.block.time = env.block.time.plus_seconds(config.round_time);
        let res = execute(deps.as_mut(), env.clone(), mock_info("bot", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("prediction_id", "1"),
                Attribute::new("locked_price", "1308900"),
                Attribute::new("is_success", "false"),
                Attribute::new("action", "resolve_prediction")
            ]
        );

        let msg = ExecuteMsg::ResolveGame {
            address: "player1".to_string(),
            round: vec![2],
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("bot", &[]),
            msg.clone(),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::PredictionStillInProgress {});
        env.block.time = env.block.time.plus_seconds(config.round_time);
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("bot", &[]),
            msg.clone(),
        )
        .unwrap();
        let bank_msg = BankMsg::Send {
            to_address: "player1".to_string(),
            amount: vec![Coin::new(99_009_900, "uusd")],
        };
        assert_eq!(res.messages, vec![SubMsg::new(CosmosMsg::Bank(bank_msg))]);
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("action", "resolve_game"),
                Attribute::new("recipient", "player1")
            ]
        );
    }
}
