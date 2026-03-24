# Sentio Tracer Rebase

Rebase the sentio tracer customizations onto the latest upstream/main.

## Background

This fork (sentioxyz/aptos-core) adds `aptos-tracer` on top of upstream aptos-labs/aptos-core. Upstream updates frequently with new features that get enabled on new blocks. The fork must be periodically rebased to stay compatible.

## Sentio-Modified Files

New packages (no conflicts, always clean):
- `aptos-move/aptos-tracer/` — the tracer package
- `docker/builder/aptos-tracer.Dockerfile`, `build-tracer.sh`
- `third_party/move/move-binary-format/src/call_trace.rs`

Modified upstream files (conflict-prone):
- `third_party/move/move-vm/runtime/src/interpreter.rs` — **most complex**, call trace collection in VM interpreter (`call_trace_internal`)
- `aptos-move/aptos-vm/src/aptos_vm.rs` — `get_call_trace()`, `CallTraceResult`, related functions
- `aptos-move/aptos-vm/src/verifier/transaction_arg_validation.rs` — `_call_trace` variant functions
- `aptos-move/aptos-vm/src/move_vm_ext/session/mod.rs`
- `aptos-move/aptos-validator-interface/src/` — REST and storage interfaces
- `crates/aptos-rest-client/src/lib.rs` — sync REST methods
- `third_party/move/move-vm/types/src/values/values_impl.rs` — ungated `as_move_value`
- `third_party/move/move-vm/types/src/values/function_values_impl.rs` — ungated `mock` module
- `third_party/move/move-binary-format/src/file_format.rs`, `errors.rs`, `lib.rs`
- `Cargo.toml`, `Cargo.lock`

## Input

$ARGUMENTS should contain one or more tx hashes (space or newline separated). These will be added to the test suite and used to verify the rebase. If empty, ask:

> Please provide at least one recent tx hash to add to the test suite.

## Workflow

Execute these steps in order. Do NOT skip steps.

### Step 1: Validate input

Parse tx hashes from $ARGUMENTS. Each must be a 0x-prefixed 66-char hex string.

### Step 2: Check current status of each tx

For each tx, run one-shot mode to check if it works on the current code:
```
cargo run --package aptos-tracer --bin aptos-tracer -- rest https://rpc.sentio.xyz/aptos/v1 <tx_hash> 1
```
Report which txs work and which fail. If a tx fails (panic, error, or trace has errors for a successful on-chain tx), note the failure mode — you'll need to fix it after the rebase.

### Step 3: Fetch upstream and check gap

```bash
git fetch upstream
git log --oneline HEAD..upstream/main --no-merges | wc -l
```
Report how many commits behind. If already up to date, stop.

### Step 4: Create backup and squash

```bash
git branch backup-$(git branch --show-current)-$(date +%Y%m%d) HEAD
git reset --soft $(git merge-base HEAD upstream/main)
git commit -m "aptos-tracer: all sentio customizations squashed"
```

### Step 5: Rebase onto upstream/main

```bash
git rebase upstream/main
```

Resolve conflicts:
- **Cargo.lock**: accept upstream (`git checkout --theirs Cargo.lock`)
- **Cargo.toml**: check for removed workspace deps. Remove from aptos-tracer's Cargo.toml if unused.
- **transaction_arg_validation.rs**: keep BOTH upstream's new functions AND sentio's `_call_trace` variants. Watch for function renames (e.g. `is_valid_txn_arg` → `legacy_is_valid_txn_arg`).
- **interpreter.rs**: usually auto-merges. Check for new opcodes/ExitCodes that need handling in the trace path (e.g. `CallClosure` needs trace frame push).
- **aptos_vm.rs**: add match arms for new enum variants (e.g. `TransactionExecutableRef::Encrypted`).
- **values_impl.rs**: `as_move_value()` MUST be available at runtime. If upstream moves it behind `#[cfg(test)]`, remove the gate and add imports (`use move_core_types::value::{MoveValue, MoveStruct}`).
- **function_values_impl.rs**: `mod mock` MUST NOT be behind `#[cfg(test)]`.

### Step 6: Fix compilation errors

Run `cargo check -p aptos-tracer` and fix iteratively. Common issues:
- Removed workspace deps → remove from Cargo.toml
- Changed function signatures (e.g. `TransactionMetadata::new` added `TimedFeatures`) → adapt callers
- New struct fields (e.g. `encryption_key` in `State`) → add with default/None
- Removed APIs (e.g. `get_account_ordered_transaction` from `DbReader`) → stub with `unimplemented!()` if only used in DB mode
- New enum variants → add match arms

### Step 7: Add new tx to test suite

Edit `aptos-move/aptos-tracer/tests/trace_consistency_test.rs`, add each new tx hash to `TEST_TXS`:
```rust
const TEST_TXS: &[(&str, &str)] = &[
    ("deposit_and_stake", "0xb7a6..."),
    ("dex_bulk_orders",   "0x1d3b..."),
    ("new_failing_case",  "0x<new_hash>"),  // add here
];
```
Use a descriptive name based on the entry function. Check what the tx calls via:
```
curl -s "https://rpc.sentio.xyz/aptos/v1/transactions/by_hash/<tx_hash>" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('payload',{}).get('function','unknown'))"
```

### Step 8: Build and run tests

```bash
cargo build --release -p aptos-tracer
cargo test -p aptos-tracer --test trace_consistency_test -- --nocapture
```

The test verifies against on-chain data: events content must match exactly (excluding FeeStatement), write set keys must be a subset, trace must have 0 errors.

If tests fail, debug and fix. Common issues:
- **Native function failures**: check `NativeTransactionContext` initialization — needs `UserTransactionContext` (via `SessionId::txn_meta` + `txn_metadata.as_user_transaction_context()`) and correct `AuxiliaryInfo` (via `PersistedAuxiliaryInfo::V1 { transaction_index }` computed from block info).
- **Stack underflow in trace**: new opcodes (e.g. `CallClosure`) may need trace frame push in `call_trace_internal`. Every `set_new_call_frame` in the trace path must have a matching `call_traces.push(InternalCallTrace{...})`.
- **Panic in call_trace.rs**: all `self.0[length-1]` accesses must use safe `if let Some(last)` pattern.
- **`param_count()` vs `param_tys().len()`**: with closure support these can differ. Always use `param_tys().len()` for stack reads in trace code.

### Step 9: Verify one-shot mode

Test each new tx to confirm no panics:
```bash
./target/release/aptos-tracer rest https://rpc.sentio.xyz/aptos/v1 <tx_hash> 1
```

### Step 10: Amend commit

```bash
git add -A
git commit --amend -m "aptos-tracer: rebase onto upstream/main ($(git rev-parse --short upstream/main))"
```

### Step 11: Report

Print summary:
- Upstream commit rebased to
- Number of conflicts resolved
- Compilation errors fixed
- New txs added to test suite
- Test results per tx (events, write set, trace frames, errors)

## Important Notes

- **Always use `--release` for testing.** Debug builds are 10x+ slower for complex txs.
- **Disk space**: target/ can grow to 35GB+. Run `cargo clean` if disk is tight.
- **`reqwest::blocking` in async context**: any new blocking HTTP calls must be wrapped in `std::thread::spawn` to avoid tokio panics.
