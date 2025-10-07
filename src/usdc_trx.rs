use alloy::{
    signers::icp::IcpSigner,
    transports::icp::{RpcApi, RpcService as AlloyRpcService},
};

use alloy::{
    primitives::{address as alloyAddress, aliases::U24, Address as AlloyPrimitivesAddress},
    sol,
};
use ic_cdk::export_candid;
use ic_cdk_timers::TimerId;
use serde::{Serialize};
use std::cell::RefCell;

use alloy::{
    network::EthereumWallet as AllowNetworkEthereumWallet,
    primitives::{U160},
    providers::ProviderBuilder,
    transports::icp::IcpConfig,
};

pub const EVM_RPC_CANISTER_ID: Principal =
    Principal::from_slice(b"\x00\x00\x00\x00\x02\x30\x00\xCC\x01\x01"); // 7hfb6-caaaa-aaaar-qadga-cai
pub const EVM_RPC: EvmRpcCanister = EvmRpcCanister(EVM_RPC_CANISTER_ID);

#[init]
pub fn init(maybe_init: Option<InitArg>) {
    if let Some(init_arg) = maybe_init {
        init_state(init_arg)
    }
}

const USDC_ADDRESS: &str = "0xf08a50178dfcde18524640ea6618a1f965821715";

pub const UNISWAP_V3_SWAP_ROUTER: AlloyPrimitivesAddress = alloyAddress!("3bFA4769FB09eefC5a80d6E87c3B9C650f7Ae48E");
pub const UNISWAP_V3_FACTORY: AlloyPrimitivesAddress = alloyAddress!("0227628f3F023bb0B980b67D528571c95c6DaC1c");

pub const MAX_ALLOWANCE: U256 = U256::MAX;

sol!(
    #[sol(rpc)]
    "sol/IUniswapV3SwapRouter.sol"
);

sol!(
    #[sol(rpc)]
    "sol/IUniswapV3Factory.sol"
);

sol!(
    #[sol(rpc)]
    "sol/IUniswapV3PoolState.sol"
);

sol!(
    #[sol(rpc)]
    "sol/IERC20.sol"
);

#[derive(Serialize, Deserialize, CandidType)]
pub struct CanisterSettingsDto {
    pub owner: String,
    pub token_in_address: String,
    pub token_in_name: String,
    pub token_out_address: String,
    pub token_out_name: String,
    pub fee: u64,
    pub amount_in: u64,
    pub slippage: u64,
    pub interval: u64,
}

#[derive(Default)]
pub struct State {
    // Settings
    owner: String,
    token_in_address: AlloyPrimitivesAddress,
    token_in_name: String,
    token_out_address: AlloyPrimitivesAddress,
    token_out_name: String,
    fee: U24,
    amount_in: U256,
    slippage: U256,
    interval: u64,

    // Runtime
    timer_id: Option<TimerId>,
    signer: Option<IcpSigner>,
    canister_eth_address: Option<AlloyPrimitivesAddress>,
    uniswap_v3_pool_address: Option<AlloyPrimitivesAddress>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

export_candid!();

pub fn get_rpc_service() -> AlloyRpcService {
    AlloyRpcService::Custom(RpcApi {
        url: "https://ic-alloy-evm-rpc-proxy.kristofer-977.workers.dev/eth-sepolia".to_string(),
        headers: None,
    })
}

pub fn get_signer() -> (IcpSigner, AlloyPrimitivesAddress) {
    STATE.with_borrow(|state| {
        (
            state.signer.as_ref().unwrap().clone(),
            state.canister_eth_address.unwrap(),
        )
    })
}

pub async fn swap(
    token_in: AlloyPrimitivesAddress,
    token_out: AlloyPrimitivesAddress,
    fee: U24,
    amount_in: U256,
    amount_out_minimum: U256,
) -> Result<String, String> {
    let (signer, recipient) = get_signer();
    let wallet = AllowNetworkEthereumWallet::from(signer);
    let rpc_service = get_rpc_service();
    let config = IcpConfig::new(rpc_service);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_icp(config);

    let args = IUniswapV3SwapRouter::ExactInputSingleParams {
        tokenIn: token_in,
        tokenOut: token_out,
        fee,
        recipient,
        amountIn: amount_in,
        amountOutMinimum: amount_out_minimum,
        sqrtPriceLimitX96: U160::from(0),
    };

    let v3_swap_router = IUniswapV3SwapRouter::new(UNISWAP_V3_SWAP_ROUTER, provider.clone());

    match v3_swap_router.exactInputSingle(args).send().await {
        Ok(res) => Ok(format!("{}", res.tx_hash())),
        Err(e) => Err(e.to_string()),
    }
}

#[update]
pub async fn swap_eth_to_usdc() -> Result<String, String> {
    let fee: U24 = U24::from(3000);
    let amount_in: U256 = U256::from(1_000_000_000_000_000u64);
    let amount_out_minimum: U256 = U256::from(0);
    let token_in = AlloyPrimitivesAddress::from_str("0x0000000000000000000000000000000000000000").map_err(|e| e.to_string())?;
    let token_out = AlloyPrimitivesAddress::from_str("0xf08a50178dfcde18524640ea6618a1f965821715").map_err(|e| e.to_string())?;

    swap(token_in, token_out, fee, amount_in, amount_out_minimum).await
}