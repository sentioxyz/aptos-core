// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simple_call_trace() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    let resp = context
        .post(
            "/view",
            json!({
                "function":"0x1::coin::balance",
                "arguments": vec![owner.address().to_string()],
                "type_arguments": vec!["0x1::aptos_coin::AptosCoin"],
            }),
        )
        .await;

    context.check_golden_output_no_prune(resp);
}
