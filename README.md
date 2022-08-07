## Metablockchain Core 

Code for metablockchain node and runtime, built using Substrate (v3.0.0)

### Build from Source

1. Install Rust and RustUp as detailed [here](https://substrate.dev/docs/en/tutorials/create-your-first-substrate-chain/setup)
   `Note : For substrate 3.0 codebase, latest tested nightly version is nightly-2021-03-12, higher nightly version may through compiler panic error `
2. Clone the repo
3. `cd metablockchain-core`
4. Run `rustup target add wasm32-unknown-unknown --toolchain nightly-2021-03-12`
5. Run `cargo build`
6. Run `sh scripts/devstart.sh` will start the node in the default dev config

### Run Tests
````
$ cargo test
````