//! KubChain header extra-data encoding and decoding.
//!
//! The extra-data field in KubChain block headers contains:
//!
//! ## Standard blocks
//! ```text
//! [32B vanity] [65B ECDSA signature]
//! ```
//!
//! ## Span boundary blocks (when validator set is updated)
//! ```text
//! [32B vanity] [N×20B validator addresses] [60B system contracts] [65B signature]
//! ```
//!
//! System contracts section: StakeManager(20B) + SlashManager(20B) + SuperNode(20B)

use alloy_primitives::{Address, Bytes};
use kubchain_primitives::{
    SystemContracts, EXTRA_CONTRACTS_LEN, EXTRA_CONTRACT_LEN, EXTRA_SEAL, EXTRA_VANITY,
};

/// Parsed extra-data from a KubChain block header.
#[derive(Debug, Clone)]
pub struct ExtraData {
    /// 32-byte vanity data.
    pub vanity: [u8; EXTRA_VANITY],
    /// Validator addresses (empty for non-span-boundary blocks).
    pub validators: Vec<Address>,
    /// System contract addresses (only present at span boundaries).
    pub system_contracts: Option<SystemContracts>,
    /// 65-byte ECDSA signature (V, R, S).
    pub seal: [u8; EXTRA_SEAL],
}

impl ExtraData {
    /// Decodes extra-data from raw bytes.
    ///
    /// Returns `None` if the data is too short to contain vanity + seal.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let min_len = EXTRA_VANITY + EXTRA_SEAL;
        if data.len() < min_len {
            return None;
        }

        let mut vanity = [0u8; EXTRA_VANITY];
        vanity.copy_from_slice(&data[..EXTRA_VANITY]);

        let mut seal = [0u8; EXTRA_SEAL];
        seal.copy_from_slice(&data[data.len() - EXTRA_SEAL..]);

        let payload = &data[EXTRA_VANITY..data.len() - EXTRA_SEAL];

        if payload.is_empty() {
            // Standard block - no validator data
            return Some(Self {
                vanity,
                validators: Vec::new(),
                system_contracts: None,
                seal,
            });
        }

        // Span boundary block: payload = validator addresses + system contracts
        if payload.len() < EXTRA_CONTRACTS_LEN {
            return None; // Too short for system contracts
        }

        let validator_bytes_len = payload.len() - EXTRA_CONTRACTS_LEN;
        if validator_bytes_len % EXTRA_CONTRACT_LEN != 0 {
            return None; // Validator addresses must be 20-byte aligned
        }

        let num_validators = validator_bytes_len / EXTRA_CONTRACT_LEN;
        let mut validators = Vec::with_capacity(num_validators);

        for i in 0..num_validators {
            let offset = i * EXTRA_CONTRACT_LEN;
            let addr = Address::from_slice(&payload[offset..offset + EXTRA_CONTRACT_LEN]);
            validators.push(addr);
        }

        // Parse system contracts (last 60 bytes of payload)
        let sc_offset = validator_bytes_len;
        let stake_manager =
            Address::from_slice(&payload[sc_offset..sc_offset + EXTRA_CONTRACT_LEN]);
        let slash_manager = Address::from_slice(
            &payload[sc_offset + EXTRA_CONTRACT_LEN..sc_offset + 2 * EXTRA_CONTRACT_LEN],
        );
        let super_node = Address::from_slice(
            &payload[sc_offset + 2 * EXTRA_CONTRACT_LEN..sc_offset + 3 * EXTRA_CONTRACT_LEN],
        );

        Some(Self {
            vanity,
            validators,
            system_contracts: Some(SystemContracts {
                stake_manager,
                slash_manager,
                super_node,
            }),
            seal,
        })
    }

    /// Encodes this extra-data back to raw bytes.
    pub fn encode(&self) -> Bytes {
        let mut data =
            Vec::with_capacity(EXTRA_VANITY + self.validators_bytes_len() + EXTRA_SEAL);

        // Vanity
        data.extend_from_slice(&self.vanity);

        // Validator addresses
        for addr in &self.validators {
            data.extend_from_slice(addr.as_slice());
        }

        // System contracts
        if let Some(sc) = &self.system_contracts {
            data.extend_from_slice(sc.stake_manager.as_slice());
            data.extend_from_slice(sc.slash_manager.as_slice());
            data.extend_from_slice(sc.super_node.as_slice());
        }

        // Seal (signature)
        data.extend_from_slice(&self.seal);

        Bytes::from(data)
    }

    /// Returns the length of the validator + system contracts section in bytes.
    fn validators_bytes_len(&self) -> usize {
        let validator_len = self.validators.len() * EXTRA_CONTRACT_LEN;
        let contracts_len = if self.system_contracts.is_some() {
            EXTRA_CONTRACTS_LEN
        } else {
            0
        };
        validator_len + contracts_len
    }

    /// Returns true if this extra-data contains validator information (span boundary).
    pub fn has_validators(&self) -> bool {
        !self.validators.is_empty()
    }

    /// Returns the seal hash (header hash without the seal bytes).
    ///
    /// This is the hash that the block producer signs. It's computed by
    /// replacing the extra-data field with only the vanity portion.
    pub fn seal_data(extra_data: &[u8]) -> Vec<u8> {
        if extra_data.len() < EXTRA_VANITY + EXTRA_SEAL {
            return extra_data.to_vec();
        }
        extra_data[..extra_data.len() - EXTRA_SEAL].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_standard_block() {
        let mut data = vec![0u8; EXTRA_VANITY + EXTRA_SEAL];
        data[0] = 0x42; // vanity marker
        data[EXTRA_VANITY] = 0x01; // signature marker

        let extra = ExtraData::decode(&data).unwrap();
        assert_eq!(extra.vanity[0], 0x42);
        assert!(extra.validators.is_empty());
        assert!(extra.system_contracts.is_none());
    }

    #[test]
    fn test_decode_span_boundary() {
        // 2 validators + 3 system contracts
        let num_validators = 2;
        let payload_len = num_validators * 20 + EXTRA_CONTRACTS_LEN;
        let total_len = EXTRA_VANITY + payload_len + EXTRA_SEAL;
        let mut data = vec![0u8; total_len];

        // Fill validator 1
        let addr1_offset = EXTRA_VANITY;
        data[addr1_offset] = 0xAA;

        // Fill validator 2
        let addr2_offset = EXTRA_VANITY + 20;
        data[addr2_offset] = 0xBB;

        // Fill stake_manager
        let sc_offset = EXTRA_VANITY + num_validators * 20;
        data[sc_offset] = 0x11;

        // Fill slash_manager
        data[sc_offset + 20] = 0x22;

        // Fill super_node
        data[sc_offset + 40] = 0x33;

        let extra = ExtraData::decode(&data).unwrap();
        assert_eq!(extra.validators.len(), 2);
        assert_eq!(extra.validators[0].as_slice()[0], 0xAA);
        assert_eq!(extra.validators[1].as_slice()[0], 0xBB);

        let sc = extra.system_contracts.unwrap();
        assert_eq!(sc.stake_manager.as_slice()[0], 0x11);
        assert_eq!(sc.slash_manager.as_slice()[0], 0x22);
        assert_eq!(sc.super_node.as_slice()[0], 0x33);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let extra = ExtraData {
            vanity: [0x42; EXTRA_VANITY],
            validators: vec![Address::repeat_byte(0xAA), Address::repeat_byte(0xBB)],
            system_contracts: Some(SystemContracts {
                stake_manager: Address::repeat_byte(0x11),
                slash_manager: Address::repeat_byte(0x22),
                super_node: Address::repeat_byte(0x33),
            }),
            seal: [0xFF; EXTRA_SEAL],
        };

        let encoded = extra.encode();
        let decoded = ExtraData::decode(&encoded).unwrap();

        assert_eq!(decoded.vanity, extra.vanity);
        assert_eq!(decoded.validators, extra.validators);
        assert_eq!(decoded.system_contracts, extra.system_contracts);
        assert_eq!(decoded.seal, extra.seal);
    }

    #[test]
    fn test_too_short_returns_none() {
        let data = vec![0u8; EXTRA_VANITY + EXTRA_SEAL - 1];
        assert!(ExtraData::decode(&data).is_none());
    }
}
