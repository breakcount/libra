// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::{Account, AccountData},
    assert_prologue_disparity, assert_prologue_parity, assert_status_eq,
    compile::compile_module_with_address,
    executor::FakeExecutor,
    transaction_status_eq,
};
use compiled_stdlib::transaction_scripts::StdlibScript;
use compiler::Compiler;
use libra_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use libra_types::{
    account_config::{self, lbr_type_tag, LBR_NAME},
    on_chain_config::VMPublishingOption,
    test_helpers::transaction_test_helpers,
    transaction::{
        Script, TransactionArgument, TransactionPayload, TransactionStatus,
        MAX_TRANSACTION_SIZE_IN_BYTES,
    },
    vm_status::{StatusCode, StatusType, VMStatus},
};
use move_core_types::gas_schedule::{GasAlgebra, GasConstants};
use transaction_builder::encode_peer_to_peer_with_metadata_script;

#[test]
fn verify_signature() {
    let mut executor = FakeExecutor::from_genesis_file();
    let sender = AccountData::new(900_000, 10);
    executor.add_account_data(&sender);
    // Generate a new key pair to try and sign things with.
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let program = encode_peer_to_peer_with_metadata_script(
        lbr_type_tag(),
        *sender.address(),
        100,
        vec![],
        vec![],
    );
    let signed_txn = transaction_test_helpers::get_test_unchecked_txn(
        *sender.address(),
        0,
        &private_key,
        sender.account().pubkey.clone(),
        Some(program),
    );

    assert_prologue_parity!(
        executor.verify_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        VMStatus::Error(StatusCode::INVALID_SIGNATURE)
    );
}

#[test]
fn verify_reserved_sender() {
    let mut executor = FakeExecutor::from_genesis_file();
    let sender = AccountData::new(900_000, 10);
    executor.add_account_data(&sender);
    // Generate a new key pair to try and sign things with.
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let program = encode_peer_to_peer_with_metadata_script(
        lbr_type_tag(),
        *sender.address(),
        100,
        vec![],
        vec![],
    );
    let signed_txn = transaction_test_helpers::get_test_signed_txn(
        account_config::reserved_vm_address(),
        0,
        &private_key,
        private_key.public_key(),
        Some(program),
    );

    assert_prologue_parity!(
        executor.verify_transaction(signed_txn.clone()).status(),
        executor.execute_transaction(signed_txn).status(),
        VMStatus::Error(StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST)
    );
}

#[test]
fn verify_simple_payment() {
    // create a FakeExecutor with a genesis from file
    let mut executor = FakeExecutor::from_genesis_file();
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender = AccountData::new(900_000, 10);
    let receiver = AccountData::new(100_000, 10);
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    // define the arguments to the peer to peer transaction
    let transfer_amount = 1_000;
    let mut args: Vec<TransactionArgument> = Vec::new();
    args.push(TransactionArgument::Address(*receiver.address()));
    args.push(TransactionArgument::U64(transfer_amount));
    args.push(TransactionArgument::U8Vector(vec![]));
    args.push(TransactionArgument::U8Vector(vec![]));

    let p2p_script = StdlibScript::PeerToPeerWithMetadata
        .compiled_bytes()
        .into_vec();

    // Create a new transaction that has the exact right sequence number.
    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        10, // this should be programmable but for now is 1 more than the setup
        100_000,
        1,
        LBR_NAME.to_owned(),
    );
    assert_eq!(executor.verify_transaction(txn).status(), None);

    // Create a new transaction that has the bad auth key.
    let txn = sender.account().create_signed_txn_with_args_and_sender(
        *receiver.address(),
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        10, // this should be programmable but for now is 1 more than the setup
        100_000,
        1,
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::INVALID_AUTH_KEY)
    );

    // Create a new transaction that has a old sequence number.
    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        1,
        100_000,
        1,
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::SEQUENCE_NUMBER_TOO_OLD)
    );

    // Create a new transaction that has a too new sequence number.
    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        11,
        100_000,
        1,
        LBR_NAME.to_owned(),
    );
    assert_prologue_disparity!(
        executor.verify_transaction(txn.clone()).status() => None,
        executor.execute_transaction(txn).status() =>
        TransactionStatus::Discard(VMStatus::Error(
                StatusCode::SEQUENCE_NUMBER_TOO_NEW
        ))
    );

    // Create a new transaction that doesn't have enough balance to pay for gas.
    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        10,
        1_000_000,
        1,
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE,)
    );

    // XXX TZ: TransactionExpired

    // RejectedWriteSet is tested in `verify_rejected_write_set`
    // InvalidWriteSet is tested in genesis.rs

    // Create a new transaction from a bogus account that doesn't exist
    let bogus_account = AccountData::new(100_000, 10);
    let txn = bogus_account.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        10,
        10_000,
        1,
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST)
    );

    // RejectedWriteSet is tested in `verify_rejected_write_set`
    // InvalidWriteSet is tested in genesis.rs

    // The next couple tests test transaction size, and bounds on gas price and the number of
    // gas units that can be submitted with a transaction.
    //
    // We test these in the reverse order that they appear in verify_transaction, and build up
    // the errors one-by-one to make sure that we are both catching all of them, and
    // that we are doing so in the specified order.
    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        10,
        1_000_000,
        GasConstants::default().max_price_per_gas_unit.get() + 1,
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND)
    );

    // Note: We can't test this at the moment since MIN_PRICE_PER_GAS_UNIT is set to 0 for
    // testnet. Uncomment this test once we have a non-zero MIN_PRICE_PER_GAS_UNIT.
    // let txn = sender.account().create_signed_txn_with_args(
    //     p2p_script.clone(),
    //     args.clone(),
    //     10,
    //     1_000_000,
    //     gas_schedule::MIN_PRICE_PER_GAS_UNIT - 1,
    // );
    // assert_eq!(
    //     executor.verify_transaction(txn).status(),
    //     Some(VMStatus::Error(
    //         StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND
    //     ))
    // );

    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        10,
        1,
        GasConstants::default().max_price_per_gas_unit.get(),
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,)
    );

    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args.clone(),
        10,
        GasConstants::default().min_transaction_gas_units.get() - 1,
        GasConstants::default().max_price_per_gas_unit.get(),
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,)
    );

    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args,
        10,
        GasConstants::default().maximum_number_of_gas_units.get() + 1,
        GasConstants::default().max_price_per_gas_unit.get(),
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,)
    );

    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        vec![TransactionArgument::U64(42); MAX_TRANSACTION_SIZE_IN_BYTES],
        10,
        GasConstants::default().maximum_number_of_gas_units.get() + 1,
        GasConstants::default().max_price_per_gas_unit.get(),
        LBR_NAME.to_owned(),
    );
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE)
    );

    // Create a new transaction that swaps the two arguments.
    let mut args: Vec<TransactionArgument> = Vec::new();
    args.push(TransactionArgument::U64(transfer_amount));
    args.push(TransactionArgument::Address(*receiver.address()));

    let txn = sender.account().create_signed_txn_with_args(
        p2p_script.clone(),
        vec![lbr_type_tag()],
        args,
        10,
        100_000,
        1,
        LBR_NAME.to_owned(),
    );
    assert_eq!(
        executor.execute_transaction(txn).status(),
        &TransactionStatus::Keep(VMStatus::Error(StatusCode::TYPE_MISMATCH,))
    );

    // Create a new transaction that has no argument.
    let txn = sender.account().create_signed_txn_with_args(
        p2p_script,
        vec![lbr_type_tag()],
        vec![],
        10,
        100_000,
        1,
        LBR_NAME.to_owned(),
    );

    assert_eq!(
        executor.execute_transaction(txn).status(),
        &TransactionStatus::Keep(VMStatus::Error(StatusCode::TYPE_MISMATCH,))
    );
}

#[test]
pub fn test_whitelist() {
    // create a FakeExecutor with a genesis from file
    let mut executor = FakeExecutor::whitelist_genesis();
    // create an empty transaction
    let sender = AccountData::new(1_000_000, 10);
    executor.add_account_data(&sender);

    // When CustomScripts is off, a garbage script should be rejected with Keep(UnknownScript)
    let random_script = vec![];
    let txn = sender.account().create_signed_txn_with_args(
        random_script,
        vec![],
        vec![],
        10,
        100_000,
        1,
        LBR_NAME.to_owned(),
    );

    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::UNKNOWN_SCRIPT)
    );
}

#[test]
pub fn test_arbitrary_script_execution() {
    // create a FakeExecutor with a genesis from file
    let mut executor =
        FakeExecutor::from_genesis_with_options(VMPublishingOption::custom_scripts());

    // create an empty transaction
    let sender = AccountData::new(1_000_000, 10);
    executor.add_account_data(&sender);

    // If CustomScripts is on, result should be Keep(DeserializationError). If it's off, the
    // result should be Keep(UnknownScript)
    let random_script = vec![];
    let txn = sender.account().create_signed_txn_with_args(
        random_script,
        vec![],
        vec![],
        10,
        100_000,
        1,
        LBR_NAME.to_owned(),
    );

    assert_eq!(executor.verify_transaction(txn.clone()).status(), None);
    let status = executor.execute_transaction(txn).status().clone();
    assert!(!status.is_discarded());
    assert_eq!(
        status.vm_status().status_code(),
        StatusCode::CODE_DESERIALIZATION_ERROR,
    );
}

#[test]
pub fn test_publish_from_libra_root() {
    // create a FakeExecutor with a genesis from file
    let mut executor =
        FakeExecutor::from_genesis_with_options(VMPublishingOption::custom_scripts());

    // create a transaction trying to publish a new module.
    let sender = AccountData::new(1_000_000, 10);
    executor.add_account_data(&sender);

    let module = String::from(
        "
        module M {
            public max(a: u64, b: u64): u64 {
                if (copy(a) > copy(b)) {
                    return copy(a);
                } else {
                    return copy(b);
                }
                return 0;
            }

            public sum(a: u64, b: u64): u64 {
                let c: u64;
                c = copy(a) + copy(b);
                return copy(c);
            }
        }
        ",
    );

    let random_module = compile_module_with_address(sender.address(), "file_name", &module);
    let txn = sender
        .account()
        .create_user_txn(random_module, 10, 100_000, 1, LBR_NAME.to_owned());
    assert_prologue_parity!(
        executor.verify_transaction(txn.clone()).status(),
        executor.execute_transaction(txn).status(),
        VMStatus::Error(StatusCode::INVALID_MODULE_PUBLISHER)
    );
}

#[test]
pub fn test_no_publishing_libra_root_sender() {
    // create a FakeExecutor with a genesis from file
    let executor = FakeExecutor::from_genesis_with_options(VMPublishingOption::custom_scripts());

    // create a transaction trying to publish a new module.
    let sender = Account::new_libra_root();

    let module = String::from(
        "
        module M {
            public max(a: u64, b: u64): u64 {
                if (copy(a) > copy(b)) {
                    return copy(a);
                } else {
                    return copy(b);
                }
                return 0;
            }

            public sum(a: u64, b: u64): u64 {
                let c: u64;
                c = copy(a) + copy(b);
                return copy(c);
            }
        }
        ",
    );

    let random_module =
        compile_module_with_address(&account_config::CORE_CODE_ADDRESS, "file_name", &module);
    let txn = sender.create_user_txn(random_module, 1, 100_000, 0, LBR_NAME.to_owned());
    assert_eq!(executor.verify_transaction(txn.clone()).status(), None);
    assert_eq!(
        executor.execute_transaction(txn).status(),
        &TransactionStatus::Keep(VMStatus::Executed)
    );
}

#[test]
pub fn test_open_publishing_invalid_address() {
    // create a FakeExecutor with a genesis from file
    let mut executor = FakeExecutor::from_genesis_with_options(VMPublishingOption::open());

    // create a transaction trying to publish a new module.
    let sender = AccountData::new(1_000_000, 10);
    let receiver = AccountData::new(1_000_000, 10);
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let module = String::from(
        "
        module M {
            public max(a: u64, b: u64): u64 {
                if (copy(a) > copy(b)) {
                    return copy(a);
                } else {
                    return copy(b);
                }
                return 0;
            }

            public sum(a: u64, b: u64): u64 {
                let c: u64;
                c = copy(a) + copy(b);
                return copy(c);
            }
        }
        ",
    );

    let random_module = compile_module_with_address(receiver.address(), "file_name", &module);
    let txn = sender
        .account()
        .create_user_txn(random_module, 10, 100_000, 1, LBR_NAME.to_owned());

    // TODO: This is not verified for now.
    // verify and fail because the addresses don't match
    // let vm_status = executor.verify_transaction(txn.clone()).status().unwrap();

    // assert!(vm_status.is(StatusType::Verification));
    // assert!(vm_status.major_status == StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER);

    // execute and fail for the same reason
    let output = executor.execute_transaction(txn);
    if let TransactionStatus::Keep(status) = output.status() {
        assert!(status.status_code() == StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER)
    } else {
        panic!("Unexpected execution status: {:?}", output)
    };
}

#[test]
pub fn test_open_publishing() {
    // create a FakeExecutor with a genesis from file
    let mut executor = FakeExecutor::from_genesis_with_options(VMPublishingOption::open());

    // create a transaction trying to publish a new module.
    let sender = AccountData::new(1_000_000, 10);
    executor.add_account_data(&sender);

    let program = String::from(
        "
        module M {
            public max(a: u64, b: u64): u64 {
                if (copy(a) > copy(b)) {
                    return copy(a);
                } else {
                    return copy(b);
                }
                return 0;
            }

            public sum(a: u64, b: u64): u64 {
                let c: u64;
                c = copy(a) + copy(b);
                return copy(c);
            }
        }
        ",
    );

    let random_module = compile_module_with_address(sender.address(), "file_name", &program);
    let txn = sender
        .account()
        .create_user_txn(random_module, 10, 100_000, 1, LBR_NAME.to_owned());
    assert_eq!(executor.verify_transaction(txn.clone()).status(), None);
    assert_eq!(
        executor.execute_transaction(txn).status(),
        &TransactionStatus::Keep(VMStatus::Executed)
    );
}

#[test]
fn test_dependency_fails_verification() {
    let mut executor = FakeExecutor::from_genesis_with_options(VMPublishingOption::open());

    // Get a module that fails verification into the store.
    let bad_module_code = "
    module Test {
        resource R1 { b: bool }
        struct S1 { r1: Self.R1 }

        public new_S1(): Self.S1 {
            let s: Self.S1;
            let r: Self.R1;
            r = R1 { b: true };
            s = S1 { r1: move(r) };
            return move(s);
        }
    }
    ";
    let compiler = Compiler {
        ..Compiler::default()
    };
    let module = compiler
        .into_compiled_module("file_name", bad_module_code)
        .expect("Failed to compile");
    executor.add_module(&module.self_id(), &module);

    // Create a transaction that tries to use that module.
    let sender = AccountData::new(1_000_000, 10);
    executor.add_account_data(&sender);

    let code = "
    import 0x1.Test;

    main() {
        let x: Test.S1;
        x = Test.new_S1();
        return;
    }
    ";

    let compiler = Compiler {
        address: *sender.address(),
        // This is OK because we *know* the module is unverified.
        extra_deps: vec![module],
        ..Compiler::default()
    };
    let script = compiler
        .into_script_blob("file_name", code)
        .expect("Failed to compile");
    let txn = sender.account().create_user_txn(
        TransactionPayload::Script(Script::new(script, vec![], vec![])),
        10,
        100_000,
        1,
        LBR_NAME.to_owned(),
    );
    // As of now, we don't verify dependencies in verify_transaction.
    assert_eq!(executor.verify_transaction(txn.clone()).status(), None);
    match executor.execute_transaction(txn).status() {
        TransactionStatus::Keep(status) => {
            assert!(status.status_type() == StatusType::Verification);
            assert!(status.status_code() == StatusCode::INVALID_RESOURCE_FIELD);
        }
        _ => panic!("Failed to find missing dependency in bytecode verifier"),
    }
}
