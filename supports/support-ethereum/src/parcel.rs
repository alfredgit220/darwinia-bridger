//! Ethereum EthereumRelayHeaderParcel
use std::fmt::Formatter;

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::bytes::to_hex;

use bridge_primitives::{bytes, hex};

use crate::block::{EthereumHeader, EthereumHeaderJson};

/// Ethereum EthereumRelayHeaderParcel
#[derive(Encode, Decode, Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct EthereumRelayHeaderParcel {
    /// Ethereum header
    pub header: EthereumHeader,
    /// MMR root
    pub mmr_root: [u8; 32],
}

impl EthereumRelayHeaderParcel {
    /// Is same as another parcel
    pub fn is_same_as(&self, another: &EthereumRelayHeaderParcel) -> bool {
        self.header.hash == another.header.hash && self.mmr_root == another.mmr_root
    }
}

impl std::fmt::Display for EthereumRelayHeaderParcel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let header = &self.header.to_string();
        let msg = format!(
            "{}\n{:>19}{}",
            header,
            "mmr_root: ",
            to_hex(&self.mmr_root, false)
        );
        write!(f, "{}", msg)
    }
}

/// Ethereum EthereumRelayHeaderParcel JSON
#[derive(Default, Deserialize, Serialize)]
pub struct EthereumRelayHeaderParcelJson {
    /// Ethereum header
    pub header: EthereumHeaderJson,
    /// MMR root
    pub mmr_root: String,
}

impl From<EthereumRelayHeaderParcelJson> for EthereumRelayHeaderParcel {
    fn from(that: EthereumRelayHeaderParcelJson) -> Self {
        EthereumRelayHeaderParcel {
            header: that.header.into(),
            mmr_root: bytes!(that.mmr_root.as_str(), 32),
        }
    }
}

impl From<EthereumRelayHeaderParcel> for EthereumRelayHeaderParcelJson {
    fn from(that: EthereumRelayHeaderParcel) -> Self {
        EthereumRelayHeaderParcelJson {
            header: that.header.into(),
            mmr_root: hex!(&that.mmr_root),
        }
    }
}
