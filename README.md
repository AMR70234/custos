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

## Honest scope (read this before a demo)

- **Real:** the site's design, copy, layout, and the dApp UI flow (policy
  creation forms, dashboard, wallet-connect attempt via the Freighter
  browser extension).
- **Simulated:** policy cards created in the UI are stored client-side only.
  The contract in `contracts/policy-wallet` is a genuine reference
  implementation of the auth logic, but it is **not deployed** — I have no
  network access to Stellar RPC from this environment, so I couldn't build,
  test, or push it to testnet for you.
- To make this fully live before judging: deploy the contract, put the
  resulting `contract_id` into the `contractNote` element / a config
  variable in `index.html`, and replace `createPolicy()`'s simulation with a
  real `InvokeHostFunction` call via `@stellar/stellar-sdk` (initialize /
  add_session_key).

Being upfront about this in your pitch is safer than judges discovering it —
most hackathon rubrics reward a clear "here's what's live, here's the
roadmap" more than a demo that quietly fudges the difference.

## Deploying the contract (testnet)

```bash
# from contracts/policy-wallet/
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/policy_wallet.wasm \
  --source <your-testnet-identity> \
  --network testnet
```

Then call `initialize` with your owner keys, threshold, and daily limit
using the Soroban CLI or the JS SDK. Check field/trait names against the
exact `soroban-sdk` version in your toolchain — the custom-auth API has
changed across recent protocol releases, so diff this against
https://stellar.github.io/js-stellar-sdk/guides/00-protocol-27-soroban-auth/
before you rely on it.

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

## What the app UI now does live

- **Live contract state**: on load, `index.html` calls Soroban testnet RPC
  directly (`server.getContractData`) and reads the real `SPEND` entry out
  of the deployed contract's instance storage — the same numbers
  `stellar contract invoke ... get_spend_policy` returns from the CLI. If
  the RPC call fails (offline, CORS, etc.) it says so explicitly and falls
  back to the last CLI-confirmed figures, labeled as such.
- **"Would this be blocked?" checker**: mirrors `__check_auth`'s logic
  (signer weight vs. threshold, rolling daily limit, session expiry/budget)
  against the live daily limit, and explains which `Error` variant would
  fire. This runs client-side rather than as a signed transaction, so it's
  labeled as a rules check, not an on-chain call.

## Ideas to extend for extra points

- Parse actual token `transfer` calls out of `auth_contexts` in
  `estimate_spend()` instead of the placeholder `0`.
- Add a policy **revocation** function + UI button (needs a new contract
  function + redeploy).
- Emit contract events on policy changes and show them as an activity feed.
