use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_slice, to_binary, Addr, Api, BalanceResponse, BankQuery, Binary, Coin, ContractResult,
    Decimal, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, Response, StdError, StdResult,
    SystemError, SystemResult, Uint128, WasmQuery,
};
use std::str::FromStr;
use terraswap::asset::Asset;
use terraswap::asset::AssetInfo::{NativeToken, Token};
use terraswap::pair::PoolResponse;

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
    base: MockQuerier<Empty>,
    pool_response: PoolInfoResponse,
}

#[derive(Clone, Default)]
pub struct PoolInfoResponse {
    pub amount_token: Uint128,
    pub amount_native: Uint128,
}
impl PoolInfoResponse {
    pub fn new(amount_native: Uint128, amount_token: Uint128) -> Self {
        PoolInfoResponse {
            amount_native,
            amount_token,
        }
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
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
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                println!("{}", contract_addr);
                if contract_addr == "terraswap" {
                    println!("{:?}", request);
                    let msg_pool = PoolResponse {
                        assets: [
                            Asset {
                                info: NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                amount: self.pool_response.amount_native,
                            },
                            Asset {
                                info: NativeToken {
                                    denom: "uluna".to_string(),
                                },
                                amount: self.pool_response.amount_token,
                            },
                        ],
                        total_share: Uint128::from(12949588085_u128),
                    };
                    return SystemResult::Ok(ContractResult::from(to_binary(&msg_pool)));
                }
                panic!("DO NOT ENTER HERE")
            }

            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier {
            base,
            pool_response: PoolInfoResponse::default(),
        }
    }
    // configure the mint whitelist mock querier
    pub fn pool_token(&mut self, amount_native: Uint128, amount_token: Uint128) {
        self.pool_response = PoolInfoResponse::new(amount_native, amount_token)
    }
}
