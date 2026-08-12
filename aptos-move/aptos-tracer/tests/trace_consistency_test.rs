// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration tests that verify tracer execution matches on-chain results.
//!
//! `get_call_trace` executes only the user function body (no prologue/epilogue),
//! so we compare user-emitted events and resource changes, excluding
//! epilogue artifacts (FeeStatement event, gas payer CoinStore).

use aptos_rest_client::Client;
use aptos_types::contract_event::ContractEvent;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::{Transaction, Version};
use aptos_types::write_set::WriteOp;
use aptos_vm::AptosVM;
use move_binary_format::call_trace::CallTraces;
use std::collections::BTreeMap;
use url::Url;

mod common {
    use super::*;
    use aptos_rest_client::aptos_api_types::{HashValue, TransactionData, TransactionOnChainData};
    use aptos_types::state_store::{
        state_storage_usage::StateStorageUsage,
        state_value::StateValue, StateViewId, TStateView, errors::StateViewError,
    };
    use std::str::FromStr;

    pub struct RestStateView {
        client: Client,
        version: Version,
    }

    impl RestStateView {
        pub fn new(client: Client, version: Version) -> Self {
            Self { client, version }
        }
    }

    impl TStateView for RestStateView {
        type Key = StateKey;

        fn id(&self) -> StateViewId {
            StateViewId::Miscellaneous
        }

        fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>, StateViewError> {
            let client = self.client.clone();
            let state_key = state_key.clone();
            let version = self.version;

            match std::thread::spawn(move || {
                client.get_raw_state_value_sync(&state_key, version)
            })
            .join()
            .unwrap()
            {
                Ok(resp) => {
                    let bytes = resp.into_inner();
                    Ok(Some(bcs::from_bytes(&bytes).map_err(|e| {
                        StateViewError::Other(format!("BCS decode error: {}", e))
                    })?))
                },
                Err(_) => Ok(None),
            }
        }

        fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
            Ok(StateStorageUsage::Untracked)
        }
    }

    pub fn get_rest_client() -> Client {
        Client::builder(
            aptos_rest_client::AptosBaseUrl::Custom(
                Url::parse("https://rpc.sentio.xyz/aptos/v1").unwrap()
            )
        ).build()
    }

    pub fn get_on_chain_txn(client: &Client, hash: &str) -> TransactionOnChainData {
        let hash_value = HashValue::from_str(hash).unwrap();
        let resp = std::thread::spawn({
            let client = client.clone();
            move || client.get_transaction_by_hash_bcs_sync(hash_value.0)
        })
        .join()
        .unwrap()
        .expect("Failed to fetch transaction from REST API");

        match resp.into_inner() {
            TransactionData::OnChain(data) => data,
            _ => panic!("Expected on-chain transaction data"),
        }
    }

    fn get_block_first_version(client: &Client, version: u64) -> u64 {
        let base_url = client.path_prefix_string();
        let url = format!("{}blocks/by_version/{}", base_url, version);
        for attempt in 0..3 {
            match reqwest::blocking::get(&url).and_then(|r| r.json::<serde_json::Value>()) {
                Ok(resp) => {
                    return resp["first_version"].as_str().unwrap().parse::<u64>().unwrap();
                },
                Err(e) if attempt < 2 => {
                    eprintln!("Retrying get_block_first_version (attempt {}): {}", attempt + 1, e);
                    std::thread::sleep(std::time::Duration::from_secs(2));
                },
                Err(e) => panic!("Failed to fetch block after 3 attempts: {}", e),
            }
        }
        unreachable!()
    }

    fn count_trace_errors_in_tree(trace: &move_binary_format::call_trace::InternalCallTrace) -> usize {
        let mut count = if trace.error.is_some() { 1 } else { 0 };
        for sub in &trace.sub_traces.0 {
            count += count_trace_errors_in_tree(sub);
        }
        count
    }

    pub fn count_trace_errors(traces: &CallTraces) -> usize {
        traces.0.iter().map(count_trace_errors_in_tree).sum()
    }

    pub fn count_trace_frames(traces: &CallTraces) -> usize {
        fn count_tree(trace: &move_binary_format::call_trace::InternalCallTrace) -> usize {
            1 + trace.sub_traces.0.iter().map(count_tree).sum::<usize>()
        }
        traces.0.iter().map(count_tree).sum()
    }

    fn is_fee_event(e: &ContractEvent) -> bool {
        format!("{:?}", e).contains("transaction_fee")
    }

    pub struct TraceTestResult {
        pub on_chain_status: String,
        pub on_chain_events: Vec<ContractEvent>,
        pub on_chain_write_set: BTreeMap<StateKey, WriteOp>,
        pub trace_events: Vec<ContractEvent>,
        pub trace_write_set: BTreeMap<StateKey, WriteOp>,
        pub trace_error_count: usize,
        pub trace_frame_count: usize,
    }

    pub fn run_trace_test(tx_hash: &str) -> TraceTestResult {
        let client = get_rest_client();
        let on_chain = get_on_chain_txn(&client, tx_hash);

        let on_chain_status = format!("{:?}", on_chain.info.status());
        // Exclude FeeStatement (emitted by epilogue, not user execution)
        let on_chain_events: Vec<_> = on_chain.events.iter()
            .filter(|e| !is_fee_event(e))
            .cloned()
            .collect();
        let on_chain_write_set: BTreeMap<StateKey, WriteOp> = on_chain.changes.as_v0()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let user_txn = match &on_chain.transaction {
            Transaction::UserTransaction(txn) => txn.clone(),
            _ => panic!("Expected UserTransaction"),
        };

        let block_first_version = get_block_first_version(&client, on_chain.version);
        let transaction_index = (on_chain.version - block_first_version) as u32;
        let state_view = RestStateView::new(client.clone(), on_chain.version - 1);

        let result = AptosVM::get_call_trace(
            &state_view,
            &user_txn,
            user_txn.max_gas_amount(),
            Some(transaction_index),
        )
        .expect("get_call_trace should not fail");

        let trace_error_count = count_trace_errors(&result.call_traces);
        let trace_frame_count = count_trace_frames(&result.call_traces);
        let trace_events: Vec<_> = result.events.iter()
            .filter(|e| !is_fee_event(e))
            .cloned()
            .collect();
        let trace_write_set: BTreeMap<StateKey, WriteOp> = result.write_set.as_v0()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        TraceTestResult {
            on_chain_status,
            on_chain_events,
            on_chain_write_set,
            trace_events,
            trace_write_set,
            trace_error_count,
            trace_frame_count,
        }
    }
}

/// Test transactions. Add new tx hashes here to expand coverage.
const TEST_TXS: &[(&str, &str)] = &[
    ("deposit_and_stake", "0xb7a6100aab24c8db0bc53928e9a04bce0c7c4304aa5df62b362a0a408441f5bf"),
    ("dex_bulk_orders",   "0x1d3bb7dfdefc0928e8c0041bc150a9f918c4abdaa73c47ef1ffc60578a7333bc"),
];

fn assert_trace_consistency(r: &common::TraceTestResult, tx_name: &str) {
    // 1. VM status
    assert!(r.on_chain_status.contains("Success"),
        "[{}] On-chain tx should be successful, got: {}", tx_name, r.on_chain_status);

    // 2. User events: exact content match (excluding FeeStatement from epilogue)
    assert_eq!(r.on_chain_events.len(), r.trace_events.len(),
        "[{}] Event count mismatch: on-chain={}, trace={}",
        tx_name, r.on_chain_events.len(), r.trace_events.len());
    for (i, (a, b)) in r.on_chain_events.iter().zip(r.trace_events.iter()).enumerate() {
        assert_eq!(a, b, "[{}] Event {} differs", tx_name, i);
    }

    // 3. Write set: trace produces the pre-epilogue write set. On-chain includes epilogue
    //    writes (gas deduction on sender's CoinStore/FungibleStore).
    //    - All trace keys must exist in on-chain
    //    - Values match for non-gas-related keys; gas-related keys may differ
    let mut value_mismatches = vec![];
    for (key, trace_op) in &r.trace_write_set {
        let on_chain_op = r.on_chain_write_set.get(key)
            .unwrap_or_else(|| panic!("[{}] Trace key missing from on-chain: {:?}", tx_name, key));
        if on_chain_op != trace_op {
            value_mismatches.push(format!("{:?}", key));
        }
    }
    // At most the gas payer's coin store can differ (epilogue deducts gas)
    assert!(value_mismatches.len() <= 1,
        "[{}] Too many write set mismatches ({}): {}",
        tx_name, value_mismatches.len(), value_mismatches.join(", "));

    // 4. Call trace: no errors, at least 1 frame
    assert_eq!(r.trace_error_count, 0,
        "[{}] Call trace has {} errors, expected 0", tx_name, r.trace_error_count);
    assert!(r.trace_frame_count > 0,
        "[{}] Call trace should have at least 1 frame", tx_name);
}

#[test]
fn test_trace_consistency() {
    for (name, hash) in TEST_TXS {
        let r = common::run_trace_test(hash);
        println!("=== {} ({}) ===", name, &hash[..10]);
        println!("  on-chain:  status={}, events={}, changes={}",
            r.on_chain_status, r.on_chain_events.len(), r.on_chain_write_set.len());
        println!("  trace:     events={}, changes={}, frames={}, errors={}",
            r.trace_events.len(), r.trace_write_set.len(), r.trace_frame_count, r.trace_error_count);
        assert_trace_consistency(&r, name);
    }
}
