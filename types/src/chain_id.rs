// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0
use anyhow::{format_err, Error, Result};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use std::{
    convert::TryFrom,
    fmt::{Display, Formatter},
    str::FromStr,
};

/// A registry of named chain IDs
/// Its main purpose is to improve human readability of reserved chain IDs in config files and CLI
/// When signing transactions for such chains, the numerical chain ID should still be used
/// (e.g. MAINNET has numeric chain ID 0, PREMAINNET has chain ID 1, etc)
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum NamedChain {
    /// MAINNET is the Libra mainnet production chain and is reserved for 0
    MAINNET,
    // The Libra chain below are non-reserved, non-production, and may change over time.  They are listed for convenience here.
    PREMAINNET,
    TESTNET,
    DEVNET,
    TESTING,
}

impl NamedChain {
    fn str_to_chain_id(s: &str) -> Result<ChainId> {
        // TODO implement custom macro that derives FromStr impl for enum (similar to libra/common/num-variants)
        let reserved_chain = match s {
            "MAINNET" => NamedChain::MAINNET,
            "PREMAINNET" => NamedChain::PREMAINNET,
            "TESTNET" => NamedChain::TESTNET,
            "DEVNET" => NamedChain::DEVNET,
            "TESTING" => NamedChain::TESTING,
            _ => {
                return Err(format_err!("Not a reserved chain: {:?}", s));
            }
        };
        Ok(ChainId::new(reserved_chain.id()))
    }

    fn id(&self) -> u8 {
        *self as u8
    }
}

/// Note: u7 in a u8 is uleb-compatible, and any usage of this should be aware
/// that this field maybe updated to be uleb64 in the future
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ChainId(u8);

pub fn deserialize_config_chain_id<'de, D>(
    deserializer: D,
) -> std::result::Result<ChainId, D::Error>
where
    D: Deserializer<'de>,
{
    struct ChainIdVisitor;

    impl<'de> Visitor<'de> for ChainIdVisitor {
        type Value = ChainId;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("ChainId as string or u8")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            ChainId::from_str(value).map_err(serde::de::Error::custom)
        }

        fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(ChainId::new(
                u8::try_from(value).map_err(serde::de::Error::custom)?,
            ))
        }
    }

    deserializer.deserialize_any(ChainIdVisitor)
}

impl Display for ChainId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        // TODO add pretty printing for NamedChain
        write!(f, "ChainId {:?}", self.0)
    }
}

impl Default for ChainId {
    fn default() -> Self {
        Self::test()
    }
}

impl FromStr for ChainId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        assert!(!s.is_empty());
        NamedChain::str_to_chain_id(s).or_else(|_err| Ok(ChainId::new(s.parse::<u8>()?)))
    }
}

impl ChainId {
    pub fn new(id: u8) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u8 {
        self.0
    }

    pub fn test() -> Self {
        ChainId::new(NamedChain::TESTING.id())
    }
}
