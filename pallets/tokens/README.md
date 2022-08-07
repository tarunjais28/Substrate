# Tokens Pallet

The tokens pallet provides functionality for creating and managing tokens on chain. The tokens are created by an account by reserving `TreasuryReserve` of the base currency. This account is then provided with the entire supply of the created tokens. Each token has a name and unique identifier. This pallet is a modification of [orml_tokens](https://github.com/open-web3-stack/open-runtime-module-library/tree/0.3.2/tokens)

## Overview

The tokens pallet provides functions for:

- transfer token balance.
- issue a new token on chain
- withdraw reserved amount from token creator account

### Terminology

- **Token:** Token here refers to a special asset created by an account (like a central bank creating its own currency token).


## Interface

### Dispatchable Functions

- `transfer` - Perform balance transfer of selected token between DIDs.
- `transfer_all` - Sweep transfer token balance to destination DID
- `issue_token` - Create a new token on chain by reserving the TreasuryReserve amount of the base asset (MetaMUI)
- `withdraw_reserved` - Withdraw from the treasury reserve amount from the token creator account.
- `slash_token` - Slash the balance of selected token from the vc owner account.
- `mint_token` - Add balance of selected token to the vc owner account.
- `transfer_token` - Perform balance transfer of selected token from VC Owner to given account.