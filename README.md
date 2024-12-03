# Cross-Chain Arbitrage Scanner with REVM

This project demonstrates how to use revm (Rust EVM) to efficiently detect cross-chain arbitrage opportunities between Ethereum and Polygon, specifically focusing on Uniswap V3 pools.

## Overview

The scanner simulates trades using local EVM execution to:
- Query WETH -> INST price on Ethereum
- Query the resulting INST -> WETH price on Polygon
- Compare prices to identify potential arbitrage opportunities

## Setup

1. Clone the repository:   ```bash
   git clone <repository-url>   ```

2. Create a `.env` file with the following variables:   ```env
   ETHEREUM_RPC=your_ethereum_rpc_url
   POLYGON_RPC_URL=your_polygon_rpc_url   ```

3. Run the scanner:   ```bash
   cargo run   ```
