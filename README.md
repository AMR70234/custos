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
- **Session keys are sealed on-chain, for real**: choosing "Session key"
  and hitting "Seal policy" now builds a genuine `InvokeHostFunction` call
  to `add_session_key` on the deployed contract, has Freighter sign it,
  submits it via Soroban RPC, and waits for confirmation. A fresh testnet
  keypair is generated client-side to act as the session signer; its
  secret is shown once so you could actually sign with it. Daily-limit and
  multisig policy types stay UI-only in this demo, since they map to
  `initialize`, which the contract only allows to run once — see
  "Ideas to extend" below for what a real update path would need.
  **Known limitation, disclosed here on purpose:** `add_session_key`'s
  `owner: Address` parameter is only checked via `owner.require_auth()` —
  it proves *someone* signed as that address, but the contract doesn't
  cross-check that address against the `OWNERS` list from `initialize`.
  In practice this means any connected wallet can seal a session key for
  itself right now, not just the registered owner. That's a real gap in
  `lib.rs`, not a UI limitation — worth fixing before this goes anywhere
  near mainnet.
- **Send / Receive (testnet)**: Receive shows your connected address for
  copying. Send builds a real classic Stellar `Payment` operation, has
  Freighter sign it, and submits it to testnet Horizon — a genuine
  transaction you can view on Stellar Expert. Note the honest boundary
  here: this moves funds from your regular Freighter account, not from
  the policy-wallet contract account, so the daily-limit / multisig rules
  aren't enforced by the protocol on this particular send — the "Check
  against policy first" button runs the same rule client-side as a
  preview of what *would* happen if the sending account were the
  contract. Making the contract itself custodial (funds live in the
  contract, and it authorizes its own payments) is the natural next step.
- **Balance display**: shows the connected account's live XLM balance,
  fetched from Horizon testnet.
- **QR code**: the Receive tab renders a scannable QR of your address
  (via the `qrcodejs` library), generated client-side.
- **Recent activity feed**: shows the contract owner account's real
  Horizon operations before a wallet connects (proof the contract is live
  even to a visitor who hasn't connected anything); once a wallet connects,
  it switches to that wallet's own transaction history instead.
- **Light/dark theme toggle** and mobile-responsive layout tweaks (nav
  collapses on small screens, stat grids stack to one column, etc).
- **Comparison table** on the landing page contrasting a regular Stellar
  wallet with a policy-governed one, row by row.
- **Export policy as JSON**: each policy card created in the UI can be
  exported as a `.json` file, showing the structured data behind it.
- **Live session-key countdown**: session-key policy cards show a
  ticking "expires in Xh Ym Zs" that updates every second.
- **Network status dot** and **copy contract ID** button next to the
  live contract state panel.

## Ideas to extend for extra points

- Parse actual token `transfer` calls out of `auth_contexts` in
  `estimate_spend()` instead of the placeholder `0`.
- Add a policy **revocation** function + UI button (needs a new contract
  function + redeploy).
- Emit contract events on policy changes and show them as an activity feed.
- **Fix `add_session_key`'s owner check** — validate that the `owner`
  parameter is actually a member of the `OWNERS` list before trusting its
  `require_auth()`, instead of accepting any address that signs for itself.
