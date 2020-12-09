# chaincode_invoker

A tool for building and sending `executor_chaincode`'s transactions.

## Usage

### 1. Examples

```rust
cargo run --example asset-transfer-basic
```
```rust
cargo run --example asset-transfer-secured-agreement
```

### 2. Use as a library

```rust
use chaincode_invoker::Invoker;

let kms_addr = "localhost:50005";
let controller_addr = "localhost:50004";

let channel_id = "cita-cloud";
let org1_mspid = "Org1MSP";

let org1_cert = /* cert file ommitted */;

let org1 = Invoker::new(
    kms_addr,
    controller_addr,
    channel_id,
    org1_mspid,
    org1_cert.as_bytes().to_vec(),
).await;

org1.call(
    // method name
    "CreateAsset",
    // args
    &["asset1", "A new asset for Org1MSP"],
    // transient_map(a key-value map representing the private data)
    &[("asset_properties", "asset1's property")],
).await;
```

