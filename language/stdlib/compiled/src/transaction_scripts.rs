// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! Rust representation of a Move transaction script that can be executed on the Libra blockchain.
//! Libra does not allow arbitrary transaction scripts; only scripts whose hashes are present in
//! the on-chain script whitelist. The genesis whitelist is derived from this file, and the
//! `Stdlib` script enum will be modified to reflect changes in the on-chain whitelist as time goes
//! on.

use anyhow::{anyhow, Error, Result};
use include_dir::{include_dir, Dir};
use libra_crypto::HashValue;
use libra_types::transaction::{ScriptABI, SCRIPT_HASH_LENGTH};
use std::{convert::TryFrom, fmt, path::PathBuf};

// This includes the script ABIs as binaries. We must use this hack to work around
// a problem with Docker, which does not copy over the Move source files that would be be used to
// produce these binaries at runtime.
const TXN_SCRIPTS_ABI_DIR: Dir = include_dir!("transaction_scripts/abi");

/// All of the Move transaction scripts that can be executed on the Libra blockchain
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum StdlibScript {
    AddCurrencyToAccount,
    AddRecoveryRotationCapability,
    AddValidator,
    Burn,
    BurnTxnFees,
    CancelBurn,
    CreateChildVaspAccount,
    CreateDesignatedDealer,
    CreateParentVaspAccount,
    CreateRecoveryAddress,
    CreateTestingAccount,
    CreateValidatorAccount,
    CreateValidatorOperatorAccount,
    FreezeAccount,
    MintLbr,
    ModifyPublishingOption,
    PeerToPeerWithMetadata,
    Preburn,
    PublishAccountLimitDefinition,
    PublishSharedEd2551PublicKey,
    Reconfigure,
    RemoveValidator,
    RotateAuthenticationKey,
    RotateAuthenticationKeyWithNonce,
    RotateAuthenticationKeyWithRecoveryAddress,
    RotateDualAttestationInfo,
    RotateSharedEd2551PublicKey,
    UpdateAccountLimitWindowInfo,
    SetValidatorConfig,
    SetValidatorOperator,
    TestnetMint,
    TieredMint,
    UnfreezeAccount,
    UnmintLbr,
    UpdateAccountLimitDefinition,
    UpdateExchangeRate,
    UpdateLibraVersion,
    UpdateMintingAbility,
    UpdateDualAttestationLimit,
    // ...add new scripts here
}

impl StdlibScript {
    /// Return a vector containing all of the standard library scripts (i.e., all inhabitants of the
    /// StdlibScript enum)
    pub fn all() -> Vec<Self> {
        use StdlibScript::*;
        vec![
            AddCurrencyToAccount,
            AddRecoveryRotationCapability,
            AddValidator,
            Burn,
            BurnTxnFees,
            CancelBurn,
            CreateChildVaspAccount,
            CreateDesignatedDealer,
            CreateParentVaspAccount,
            CreateRecoveryAddress,
            CreateTestingAccount,
            CreateValidatorAccount,
            CreateValidatorOperatorAccount,
            FreezeAccount,
            MintLbr,
            ModifyPublishingOption,
            PeerToPeerWithMetadata,
            Preburn,
            PublishAccountLimitDefinition,
            PublishSharedEd2551PublicKey,
            Reconfigure,
            RemoveValidator,
            RotateAuthenticationKey,
            RotateAuthenticationKeyWithNonce,
            RotateAuthenticationKeyWithRecoveryAddress,
            RotateDualAttestationInfo,
            RotateSharedEd2551PublicKey,
            UpdateAccountLimitWindowInfo,
            SetValidatorConfig,
            SetValidatorOperator,
            TestnetMint,
            TieredMint,
            UnfreezeAccount,
            UnmintLbr,
            UpdateAccountLimitDefinition,
            UpdateExchangeRate,
            UpdateLibraVersion,
            UpdateMintingAbility,
            UpdateDualAttestationLimit,
            // ...add new scripts here
        ]
    }

    /// Construct the whitelist of script hashes used to determine whether a transaction script can
    /// be executed on the Libra blockchain
    pub fn whitelist() -> Vec<[u8; SCRIPT_HASH_LENGTH]> {
        StdlibScript::all()
            .iter()
            .map(|script| *script.compiled_bytes().hash().as_ref())
            .collect()
    }

    /// Return a lowercase-underscore style name for this script
    pub fn name(self) -> String {
        self.to_string()
    }

    /// Return true if `code_bytes` is the bytecode of one of the standard library scripts
    pub fn is(code_bytes: &[u8]) -> bool {
        Self::try_from(code_bytes).is_ok()
    }

    /// Return the Move bytecode that was produced by compiling this script.
    pub fn compiled_bytes(self) -> CompiledBytes {
        CompiledBytes(self.abi().code().to_vec())
    }

    /// Return the ABI of the script (including the bytecode).
    pub fn abi(self) -> ScriptABI {
        let mut path = PathBuf::from(self.name());
        path.set_extension("abi");
        let content = TXN_SCRIPTS_ABI_DIR
            .get_file(path.clone())
            .unwrap_or_else(|| panic!("File {:?} does not exist", path))
            .contents();
        lcs::from_bytes(content)
            .unwrap_or_else(|err| panic!("Failed to deserialize ABI file {:?}: {}", path, err))
    }

    /// Return the sha3-256 hash of the compiled script bytes.
    pub fn hash(self) -> HashValue {
        self.compiled_bytes().hash()
    }
}

/// Bytes produced by compiling a Move source language script into Move bytecode
#[derive(Clone)]
pub struct CompiledBytes(Vec<u8>);

impl CompiledBytes {
    /// Return the sha3-256 hash of the script bytes
    pub fn hash(&self) -> HashValue {
        Self::hash_bytes(&self.0)
    }

    /// Return the sha3-256 hash of the script bytes
    fn hash_bytes(bytes: &[u8]) -> HashValue {
        HashValue::sha3_256_of(bytes)
    }

    /// Convert this newtype wrapper into a vector of bytes
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl TryFrom<&[u8]> for StdlibScript {
    type Error = Error;

    /// Return `Some(<script_name>)` if  `code_bytes` is the bytecode of one of the standard library
    /// scripts, None otherwise.
    fn try_from(code_bytes: &[u8]) -> Result<Self> {
        let hash = CompiledBytes::hash_bytes(code_bytes);
        Self::all()
            .iter()
            .find(|script| script.hash() == hash)
            .cloned()
            .ok_or_else(|| anyhow!("Could not create standard library script from bytes"))
    }
}

impl fmt::Display for StdlibScript {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use StdlibScript::*;
        write!(
            f,
            "{}",
            match self {
                AddValidator => "add_validator",
                AddCurrencyToAccount => "add_currency_to_account",
                AddRecoveryRotationCapability => "add_recovery_rotation_capability",
                Burn => "burn",
                BurnTxnFees => "burn_txn_fees",
                CancelBurn => "cancel_burn",
                CreateChildVaspAccount => "create_child_vasp_account",
                CreateDesignatedDealer => "create_designated_dealer",
                CreateParentVaspAccount => "create_parent_vasp_account",
                CreateRecoveryAddress => "create_recovery_address",
                CreateTestingAccount => "create_testing_account",
                CreateValidatorAccount => "create_validator_account",
                CreateValidatorOperatorAccount => "create_validator_operator_account",
                FreezeAccount => "freeze_account",
                TestnetMint => "testnet_mint",
                MintLbr => "mint_lbr",
                ModifyPublishingOption => "modify_publishing_option",
                PeerToPeerWithMetadata => "peer_to_peer_with_metadata",
                Preburn => "preburn",
                PublishAccountLimitDefinition => "publish_account_limit_definition",
                PublishSharedEd2551PublicKey => "publish_shared_ed25519_public_key",
                Reconfigure => "reconfigure",
                RemoveValidator => "remove_validator",
                RotateAuthenticationKey => "rotate_authentication_key",
                RotateAuthenticationKeyWithNonce => "rotate_authentication_key_with_nonce",
                RotateAuthenticationKeyWithRecoveryAddress =>
                    "rotate_authentication_key_with_recovery_address",
                RotateDualAttestationInfo => "rotate_dual_attestation_info",
                RotateSharedEd2551PublicKey => "rotate_shared_ed25519_public_key",
                UpdateAccountLimitWindowInfo => "update_account_limit_window_info",
                SetValidatorConfig => "set_validator_config",
                SetValidatorOperator => "set_validator_operator",
                TieredMint => "tiered_mint",
                UpdateDualAttestationLimit => "update_dual_attestation_limit",
                UnfreezeAccount => "unfreeze_account",
                UnmintLbr => "unmint_lbr",
                UpdateAccountLimitDefinition => "update_account_limit_definition",
                UpdateLibraVersion => "update_libra_version",
                UpdateExchangeRate => "update_exchange_rate",
                UpdateMintingAbility => "update_minting_ability",
            }
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // This includes the compiled script binaries.
    const COMPILED_TXN_SCRIPTS_DIR: Dir = include_dir!("transaction_scripts");

    #[test]
    fn test_file_correspondence() {
        // Make sure that every compiled file under transaction_scripts is represented in
        // StdlibScript::all() (and vice versa).
        let files = COMPILED_TXN_SCRIPTS_DIR.files();
        let scripts = StdlibScript::all();
        for file in files {
            assert!(
                StdlibScript::is(file.contents()),
                "File {} missing from StdlibScript enum",
                file.path().display()
            )
        }
        assert_eq!(
            files.len(),
            scripts.len(),
            "Mismatch between stdlib script files and StdlibScript enum. {}",
            if files.len() > scripts.len() {
                "Did you forget to extend the StdlibScript enum?"
            } else {
                "Did you forget to rebuild the standard library?"
            }
        );
    }

    #[test]
    fn test_names() {
        // Make sure that the names listed here matches the function names in the code.
        for script in StdlibScript::all() {
            assert_eq!(
                script.name(),
                script.abi().name(),
                "The main function in language/stdlib/transaction_scripts/{}.move is named `{}` instead of `{}`. Please fix the issue and re-run (cd language/stdlib && cargo run --release)",
                script.name(),
                script.abi().name(),
                script.name(),
            );
        }
    }

    #[test]
    fn test_docs() {
        // Make sure that scripts have non-empty documentation.
        for script in StdlibScript::all() {
            assert!(
                !script.abi().doc().is_empty(),
                "The main function in language/stdlib/transaction_scripts/{}.move does not have a `///` inline doc comment. Please fix the issue and re-run (cd language/stdlib && cargo run --release)",
                script.name(),
            );
        }
    }
}
