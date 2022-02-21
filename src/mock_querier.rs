use crate::msg::{Asset, AssetInfo, CumulativePricesResponse};
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
    cumulative_price_response: CumulativePriceInfoResponse,
}

#[derive(Clone)]
pub struct CumulativePriceInfoResponse {
    pub asset: [Asset; 2],
    pub total_share: Uint128,
    pub price0_cumulative_last: Uint128,
    pub price1_cumulative_last: Uint128,
}
impl CumulativePriceInfoResponse {
    pub fn new(
        asset: [Asset; 2],
        total_share: Uint128,
        price0_cumulative_last: Uint128,
        price1_cumulative_last: Uint128,
    ) -> Self {
        CumulativePriceInfoResponse {
            asset,
            total_share,
            price0_cumulative_last,
            price1_cumulative_last,
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
                    let msg_state = CumulativePricesResponse {
                        assets: self.cumulative_price_response.clone().asset,
                        total_share: self.cumulative_price_response.total_share,
                        price0_cumulative_last: self
                            .cumulative_price_response
                            .price0_cumulative_last,
                        price1_cumulative_last: self
                            .cumulative_price_response
                            .price1_cumulative_last,
                    };
                    return SystemResult::Ok(ContractResult::from(to_binary(&msg_state)));
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
            cumulative_price_response: CumulativePriceInfoResponse::new(
                [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uusd".to_string(),
                        },
                        amount: Uint128::from(86931615534331u128),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                        amount: Uint128::from(1730767396012u128),
                    },
                ],
                Uint128::from(11838366329935u128),
                Uint128::from(73124950600u128),
                Uint128::from(631586305808862u128),
            ),
        }
    }
    // configure the mint whitelist mock querier
    pub fn pool_token(
        &mut self,
        asset: [Asset; 2],
        total_share: Uint128,
        price0_cumulative_last: Uint128,
        price1_cumulative_last: Uint128,
    ) {
        self.cumulative_price_response = CumulativePriceInfoResponse::new(
            asset,
            total_share,
            price0_cumulative_last,
            price1_cumulative_last,
        )
    }
}
