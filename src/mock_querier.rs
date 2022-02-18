use crate::msg::{
    OracleListPriceFeedResponse, OraclePriceFeedQueryMsg, OraclePriceFeedResponse,
    OraclePriceFeedStateResponse,
};
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_slice, to_binary, Addr, Api, BalanceResponse, BankQuery, Binary, Coin, ContractResult,
    Decimal, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, Response, StdError, StdResult,
    SystemError, SystemResult, Uint128, WasmQuery,
};
use std::str::FromStr;
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper};

pub const MOCK_HUB_CONTRACT_ADDR: &str = "hub";
pub const MOCK_CW20_CONTRACT_ADDR: &str = "lottery";
//pub const MOCK_REWARD_CONTRACT_ADDR: &str = "reward";
pub const MOCK_TOKEN_CONTRACT_ADDR: &str = "token";

pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier =
        WasmMockQuerier::new(MockQuerier::new(&[(&MOCK_CONTRACT_ADDR, contract_balance)]));
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    oracle_price_feed_response: OraclePriceFeedInfoResponse,
}

#[derive(Clone, Default)]
pub struct OraclePriceFeedInfoResponse {
    pub amount_token: Uint128,
    pub amount_native: Uint128,
}
impl OraclePriceFeedInfoResponse {
    pub fn new(amount_native: Uint128, amount_token: Uint128) -> Self {
        OraclePriceFeedInfoResponse {
            amount_native,
            amount_token,
        }
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                println!("{}", contract_addr);
                if contract_addr == "oracle" {
                    return if String::from_utf8(msg.to_vec()).unwrap() == "{\"state\":{}}" {
                        let msg_state = OraclePriceFeedStateResponse {
                            pool_address: "astroport".to_string(),
                            round: 14,
                            denom_one: "uusd".to_string(),
                            denom_two: "uluna".to_string(),
                        };
                        SystemResult::Ok(ContractResult::from(to_binary(&msg_state)))
                    } else {
                        let msg_pool = OracleListPriceFeedResponse {
                            list: vec![
                                OraclePriceFeedResponse {
                                    timestamp: 1571797330,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string()
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797340,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string()
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797350,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797360,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797370,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797380,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797390,
                                    price: Uint128::from(55_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797400,
                                    price: Uint128::from(56_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797409,
                                    price: Uint128::from(50_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797419,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797429,
                                    price: Uint128::from(51_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797439,
                                    price: Uint128::from(59_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797449,
                                    price: Uint128::from(65_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797459,
                                    price: Uint128::from(45_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797469,
                                    price: Uint128::from(90_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797479,
                                    price: Uint128::from(58_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797489,
                                    price: Uint128::from(59_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797499,
                                    price: Uint128::from(52_000_000u128),
                                    worker: "rico".to_string(),
                                },
                                OraclePriceFeedResponse {
                                    timestamp: 1571797719,
                                    price: Uint128::from(45_000_000u128),
                                    worker: "rico".to_string(),
                                },
                            ],
                        };
                        SystemResult::Ok(ContractResult::from(to_binary(&msg_pool)))
                    };
                }
                panic!("DO NOT ENTER HERE")
            }
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => match query_data {
                TerraQuery::TaxRate {} => {
                    let res = TaxRateResponse {
                        rate: Decimal::percent(1),
                    };
                    SystemResult::Ok(ContractResult::from(to_binary(&res)))
                }
                TerraQuery::TaxCap { denom: _ } => {
                    let cap = Uint128::from(1000000u128);
                    let res = TaxCapResponse { cap };
                    SystemResult::Ok(ContractResult::from(to_binary(&res)))
                }
                _ => panic!("DO NOT ENTER HERE"),
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            oracle_price_feed_response: OraclePriceFeedInfoResponse::default(),
        }
    }
    // configure the mint whitelist mock querier
    pub fn pool_token(&mut self, amount_native: Uint128, amount_token: Uint128) {
        self.oracle_price_feed_response =
            OraclePriceFeedInfoResponse::new(amount_native, amount_token)
    }
}
