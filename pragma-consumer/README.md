# Pragma Consumer SDK

The Pragma Consumer SDK is used to fetch options and their associated Merkle proofs
so you can use them in our Pragma Oracle contract to interact with the Merkle Feed published on-chain.

We have [examples](./examples/src/) to help you get started.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
pragma-consumer = "0.1.0"
```

## Quick Start

```rust
use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::{ApiConfig, PragmaBaseUrl};
use pragma_consumer::macros::instrument;
use pragma_consumer::types::{BlockId, Instrument};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let api_config = ApiConfig {
        base_url: PragmaBaseUrl::Prod,
        api_key: "your_api_key".into(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_mainnet()
        .with_http(api_config)
        .await
        .unwrap();

    let instrument = instrument!("BTC-16AUG24-52000-P");

    let result = consumer
        .get_merkle_feed_calldata(&instrument, None) // None = Pending block by default
        .await
        .unwrap();

    // Use the calldata with the pragma-oracle contract...
    println!("Hex calldata: {}", result.as_hex_calldata());

    // result.calldata() returns the calldata wrapped with Felt
    // from starknet-rs 0.9.0
}
```

## Usage

### Configure the API connection

Create an instance of an `ApiConfig` object:

```rust
let api_config = ApiConfig {
    // This will use our dev API
    base_url: PragmaBaseUrl::Dev, // or PragmaBaseUrl::Prod
    api_key: "your_api_key".into(),
};

// If you need a custom url, you can do:
let api_config = ApiConfig {
    base_url: PragmaBaseUrl::Custom("http://localhost:3000".into()),
    api_key: "your_api_key".into(),
};
```

### Initializing the Consumer

Create a `PragmaConsumer` instance using the builder pattern:

```rust
let consumer = PragmaConsumerBuilder::new()
    .on_sepolia() // or .on_mainnet()
    .with_http(api_config)
    .await?;
```

**NOTE**: By default, the network will be `Sepolia` if you don't specify it:

```rust
let consumer = PragmaConsumerBuilder::new()
    .with_http(api_config)
    .await?;
```

You can also add a `check_api_health` call to the builder to make sure the connection with the PragmAPI is healthy:

```rust
let consumer = PragmaConsumerBuilder::new()
    .check_api_health()
    .with_http(api_config)
    .await?;
```

### Fetching Merkle Feed Calldata

Use the `get_merkle_feed_calldata` method to fetch the necessary data for interacting with the Pragma Oracle:

```rust
let calldata = consumer
    .get_merkle_feed_calldata(&instrument, block_number)
    .await?;
```

### Creating Instruments

You can create an Instrument in two ways:

#### 1. Using the `instrument!` macro:

```rust
let instrument = instrument!("BTC-16AUG24-52000-P");
```

#### 2. Manually constructing the `Instrument` struct:

```rust
use pragma_consumer::{Instrument, OptionCurrency, OptionType};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;

let instrument = Instrument {
    base_currency: OptionCurrency::BTC,
    expiration_date: NaiveDate::from_ymd_opt(2024, 8, 16).unwrap(),
    strike_price: BigDecimal::from(52000).unwrap(),
    option_type: OptionType::Put
};
```

You can retrieve the name of an instrument with the `name()` method:

```rust
println!("{}", instrument.name());

// BTC-16AUG24-52000-P
```

### Specifying Block ID

You can specify the block in different ways:

```rust
use pragma_consumer::types::{BlockId, BlockTag};

// Using a specific block number
let block = BlockId::Number(85925);

// Using the latest block
let block = BlockId::Tag(BlockTag::Latest);

// Using the pending block
let block = BlockId::Tag(BlockTag::Pending);
```

### Error Handling

The SDK uses the `thiserror` crate for error handling. The two main errors types are:

- `builder::BuilderError` for errors during the `PragmaConsumer` building,
- `conssumer::ConsumerError` for errors during the fetching of the option and the merkle proof.
