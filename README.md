# Custos — Policy-Governed Wallets on Stellar

A smart wallet built on Soroban's custom authorization framework. Instead of
one key that can move everything, the wallet owner attaches on-chain
**policies**: a rolling daily spending limit, a weighted multisig threshold,
and expiring, budget-capped **session keys** for dApps or games.

## What's in this folder

```
index.html                         → landing/pitch page + working dApp UI (open directly in a browser)
contracts/policy-wallet/src/lib.rs → Soroban contract: CustomAccountInterface + policy logic
contracts/policy-wallet/Cargo.toml → build manifest
```

## Live deployment (testnet)

Deployed and initialized — this is a real, on-chain contract, not a mock.

| | |
|---|---|
| **Contract ID** | `CDWSVAK3GEC2T3YZT2NCGJNEKOAOEI2EIJVKZ3CIMHTUJRDJ75VCKBFV` |
| **Network** | Stellar Testnet |
| **Deployed** | Aug 3, 2026 |
| **Owner** | `GA6HHBRA73MMJ2M3RHFKGQKJVZSCWV52VL25G2CBNAVSJKDVORQNP6RH` |
| **Daily limit** | 500 XLM (rolling 24h) |
| **Explorer** | https://lab.stellar.org/r/testnet/contract/CDWSVAK3GEC2T3YZT2NCGJNEKOAOEI2EIJVKZ3CIMHTUJRDJ75VCKBFV |

Verify it yourself:

```bash
stellar contract invoke \
  --id CDWSVAK3GEC2T3YZT2NCGJNEKOAOEI2EIJVKZ3CIMHTUJRDJ75VCKBFV \
  --source my-wallet \
  --network testnet \
  -- \
  get_spend_policy
```

## Deploying your own copy

```bash
rustup target add wasm32-unknown-unknown
cd contracts/policy-wallet
cargo build --target wasm32-unknown-unknown --release
stellar keys generate my-wallet --network testnet --fund
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/policy_wallet.wasm \
  --source my-wallet \
  --network testnet
```

Then call `initialize` with your owner keys, threshold, and daily limit.

## Suggested pitch structure (3–5 min demo)

1. **Problem (30s):** one leaked key = total loss. No native concept of
   "this much, this app, this long."
2. **Live demo (2 min):** open `index.html` → Launch app → connect wallet →
   create a spending-limit policy and a session key → point at the sealed
   cards.
3. **How it's enforced (1 min):** walk through `__check_auth` in `lib.rs` —
   signature verification, weight tally, rolling-limit check, session
   expiry/budget check.
4. **What's next (30s):** real spend-amount parsing from `auth_contexts`,
   a revocation flow, and a mainnet audit pass.

## Ideas to extend for extra points

- Parse actual token `transfer` calls out of `auth_contexts` in
  `estimate_spend()` instead of the placeholder `0`.
- Add a policy **revocation** function + UI button.
- Emit contract events on policy changes and show them as an activity feed.
- Add a "why was this blocked" screen using the contract's `Error` variants.
