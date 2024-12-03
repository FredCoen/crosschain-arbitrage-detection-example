use alloy::primitives::address;
use alloy::{
    primitives::{aliases::U24, Address, Bytes, U160, U256},
    providers::ProviderBuilder,
    sol,
    sol_types::{SolCall, SolValue},
    uint,
};
use revm::{
    db::{AlloyDB, CacheDB},
    primitives::{ExecutionResult, Output},
    Evm,
};
use std::sync::Arc;

pub static WETH_ADDRESS_ETHEREUM: Address = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
pub static WETH_ADDRESS_POLYGON: Address = address!("7ceB23fD6bC0adD59E62ac25578270cFf1b9f619");
pub static UNISWAP_QUOTER_ADDRESS_ETHEREUM: Address =
    address!("61fFE014bA17989E743c5F6cB21bF9697530B21e");
pub static UNISWAP_QUOTER_ADDRESS_POLYGON: Address =
    address!("61fFE014bA17989E743c5F6cB21bF9697530B21e");
pub static ZERO_ADDRESS: Address = address!("0000000000000000000000000000000000000000");
pub static INST_ADDRESS_ETHEREUM: Address = address!("6f40d4a6237c257fff2db00fa0510deeecd303eb");
pub static INST_ADDRESS_POLYGON: Address = address!("f50d05a1402d0adafa880d36050736f9f6ee7dee");

use dotenv::dotenv;

#[tokio::main]
async fn main() -> () {
    dotenv().ok();

    let ethereum_provider =
        ProviderBuilder::new().on_http(std::env::var("ETHEREUM_RPC").unwrap().parse().unwrap());

    let cache_db_ethereum =
        CacheDB::new(AlloyDB::new(Arc::new(ethereum_provider), Default::default()).unwrap());

    let wad: U256 = uint!(1_000_000_000_000_000_000_U256);
    let quoter_calldata =
        construct_calldata(WETH_ADDRESS_ETHEREUM, INST_ADDRESS_ETHEREUM, wad, 10000);

    let mut evm = Evm::builder()
        .with_db(cache_db_ethereum)
        .modify_tx_env(|tx| {
            tx.caller = ZERO_ADDRESS;
            tx.transact_to = revm::primitives::TransactTo::Call(UNISWAP_QUOTER_ADDRESS_ETHEREUM);
            tx.data = quoter_calldata;
            tx.value = U256::ZERO;
        })
        .build();

    let result = match evm.transact().unwrap().result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            panic!("quoter call failed: {result:?}");
        }
    };

    let (inst_token_received_ethereum, _, _, _) =
        <(u128, u128, u32, u128)>::abi_decode(&result, false).unwrap();

    let polygon_provider =
        ProviderBuilder::new().on_http(std::env::var("POLYGON_RPC_URL").unwrap().parse().unwrap());
    let cache_db_polygon =
        CacheDB::new(AlloyDB::new(Arc::new(polygon_provider), Default::default()).unwrap());

    let quoter_calldata = construct_calldata(
        INST_ADDRESS_POLYGON,
        WETH_ADDRESS_POLYGON,
        U256::from(inst_token_received_ethereum),
        10000,
    );

    let mut evm = Evm::builder()
        .with_db(cache_db_polygon)
        .modify_tx_env(|tx| {
            tx.caller = ZERO_ADDRESS;
            tx.transact_to = revm::primitives::TransactTo::Call(UNISWAP_QUOTER_ADDRESS_POLYGON);
            tx.data = quoter_calldata;
            tx.value = U256::ZERO;
        })
        .build();

    let result = evm.transact().unwrap().result;

    let quoter_response = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            panic!("quoter call failed: {result:?}");
        }
    };

    let (weth_token_received_polygon, _, _, _) =
        <(u128, u128, u32, u128)>::abi_decode(&quoter_response, false).unwrap();

    println!(
        "On ethereum {:.10} weth gets you {:.10} $inst, \
            if you want to sell that amount of $inst on polygon you get {:.10} weth",
        wad.to_string().parse::<f64>().unwrap() / 1e18,
        parse_to_decimal(inst_token_received_ethereum),
        parse_to_decimal(weth_token_received_polygon)
    );
}

fn parse_to_decimal(value: u128) -> f64 {
    value.to_string().parse::<f64>().unwrap() / 1e18
}

sol! {
    struct QuoteExactInputSingleParams {
        address tokenIn;
        address tokenOut;
        uint256 amountIn;
        uint24 fee;
        uint160 sqrtPriceLimitX96;
    }

    function quoteExactInputSingle(QuoteExactInputSingleParams memory params)
    public
    override
    returns (
        uint256 amountOut,
        uint160 sqrtPriceX96After,
        uint32 initializedTicksCrossed,
        uint256 gasEstimate
    );

}

sol! {
    function getAmountOut(
        address pool,
        bool zeroForOne,
        uint256 amountIn
    ) external;
}

pub fn construct_calldata(
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee: u32,
) -> Bytes {
    // Determine swap direction based on token addresses
    // - If token_in < token_out (zero_for_one = true): swapping token0 for token1
    // - If token_in > token_out (zero_for_one = false): swapping token1 for token0
    let zero_for_one = token_in < token_out;

    // Set price limits for the swap using Uniswap V3's sqrt-price format (Q64.96)
    // When zero_for_one is true: Set minimum acceptable price (~0)
    // When zero_for_one is false: Set maximum acceptable price (very large number)
    // This ensures the swap executes at any price, while maintaining Uniswap's price boundary requirements
    let sqrt_price_limit_x96: U160 = if zero_for_one {
        // Minimum price limit
        "4295128749".parse().unwrap()
    } else {
        // Maximum price limit
        "1461446703485210103287273052203988822378723970341"
            .parse()
            .unwrap()
    };

    let params = QuoteExactInputSingleParams {
        tokenIn: WETH_ADDRESS_ETHEREUM,
        tokenOut: INST_ADDRESS_ETHEREUM,
        amountIn: amount_in,
        fee: U24::from(fee),
        sqrtPriceLimitX96: sqrt_price_limit_x96,
    };

    Bytes::from(quoteExactInputSingleCall { params }.abi_encode())
}
