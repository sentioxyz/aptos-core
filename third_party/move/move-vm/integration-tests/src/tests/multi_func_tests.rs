// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use move_binary_format::errors::VMResult;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::{MoveTypeLayout, MoveValue},
};
use move_vm_runtime::{move_vm::MoveVM, session::SerializedReturnValues};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::call_trace::CallTraces;
use move_vm_types::gas::UnmeteredGasMeter;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

fn run(
    ty_args: Vec<TypeTag>,
    args: Vec<MoveValue>,
) -> VMResult<CallTraces> {
    let code = r#"
        module {{ADDR}}::M {
            public fun foo(v1: u64): u64 {
                bar(v1) + 1
            }

            public fun bar(v1: u64): u64 {
                1 + v1
            }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));

    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();

    let mut storage = InMemoryStorage::new();
    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
    storage.publish_or_overwrite_module(module_id.clone(), blob);

    let vm = MoveVM::new(vec![]).unwrap();
    let mut sess = vm.new_session(&storage);

    let fun_name = Identifier::new("foo").unwrap();

    let args: Vec<_> = args
        .into_iter()
        .map(|val| val.simple_serialize().unwrap())
        .collect();

    // let SerializedReturnValues {
    //     return_values,
    //     mutable_reference_outputs: _,
    // } = sess.execute_function_bypass_visibility(
    //     &module_id,
    //     &fun_name,
    //     ty_args,
    //     args,
    //     &mut UnmeteredGasMeter,
    // )?;

    sess.call_trace(
        &module_id,
        &fun_name,
        ty_args,
        args,
        &mut UnmeteredGasMeter,
    )

    // Ok(return_values
    //     .into_iter()
    //     .map(|(bytes, _layout)| bytes)
    //     .collect())
}

fn expect_success(
    ty_args: Vec<TypeTag>,
    args: Vec<MoveValue>,
    expected_layouts: &[MoveTypeLayout],
) {
    let return_vals = run(ty_args, args).unwrap();
    assert_eq!(return_vals.len(), 1);
}

#[test]
fn multi_func() {
    expect_success(vec![], vec![MoveValue::U64(6)], &[
        MoveTypeLayout::U64,
    ])
}
