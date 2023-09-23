// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_api_test_context::{assert_json, current_function_name, pretty, TestContext};
use super::new_test_context;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_call_trace_by_hash() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2&limit=1").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    let resp = context
        .get(&format!(
            "/transactions/by_hash/{}",
            txns[0]["hash"].as_str().unwrap()
        ))
        .await;
    assert_json(resp, txns[0].clone());

    let call_trace_resp = context.
        get(&format!(
            "/call_trace/by_hash/{}",
            txns[0]["hash"].as_str().unwrap()
        ))
        .await;
    context.check_golden_output(call_trace_resp);
}
