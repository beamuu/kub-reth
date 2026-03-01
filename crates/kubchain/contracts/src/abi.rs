//! ABI definitions for KubChain system contracts.
//!
//! Uses `alloy-sol-types` to define type-safe Solidity ABI bindings for
//! the system contracts. These are used to encode/decode function calls
//! for system transactions during block finalization.
//!
//! Source: `bkc/consensus/clique/contract/abi.go`

use alloy_sol_types::sol;

// StakeManager contract interface
sol! {
    /// StakeManager contract for reward distribution and staking management.
    ///
    /// Source: `bkc/consensus/clique/contract/abi.go` (stakeManageABI)
    #[allow(missing_docs)]
    interface IStakeManager {
        /// Distributes block reward to the validator.
        function distributeReward(uint256 amount, address validator) external;

        /// Returns the StakeManagerStorage contract address.
        function stakeManagerStorage() external view returns (address);

        /// Returns the StakeManagerVault contract address.
        function stakeManagerVault() external view returns (address);

        /// Returns the NFT contract address.
        function nftContract() external view returns (address);

        /// Returns the KKUB token contract address.
        function kkub() external view returns (address);

        /// Initializes super node during Basel hard fork.
        function initialSuperNode(address superNodeAddress) external;
    }
}

// ValidatorSet contract interface
sol! {
    /// ValidatorSet contract for validator management and span commitment.
    ///
    /// Source: `bkc/consensus/clique/contract/abi.go` (validatorSetABI)
    #[allow(missing_docs)]
    interface IValidatorSet {
        /// Commits a new validator set for the next span.
        function commitSpan(bytes validatorBytes) external;

        /// Returns the current span number.
        function currentSpanNumber() external view returns (uint256);

        /// Returns the current validator set and system contract addresses.
        function getValidators() external view returns (
            address[] memory validators,
            uint256[] memory powers,
            address stakeManager,
            address slashManager,
            address officialNode
        );

        /// Returns eligible validators for the next span.
        function getEligibleValidators() external view returns (
            address[] memory validators,
            uint256[] memory powers
        );

        /// Returns the StakeManager address.
        function getStakeManager() external view returns (address);

        /// Returns the SlashManager address.
        function getSlashManager() external view returns (address);
    }
}

// ValidatorSet V2 with SuperNode support (Basel onwards)
sol! {
    /// ValidatorSet V2 with super node support.
    ///
    /// Extends the base ValidatorSet with super node information.
    #[allow(missing_docs)]
    interface IValidatorSetV2 {
        /// Returns validators including super node information.
        function getValidatorsWithSuperNode() external view returns (
            address[] memory validators,
            uint256[] memory powers,
            address stakeManager,
            address slashManager,
            address superNode
        );
    }
}

// SlashManager contract interface
sol! {
    /// SlashManager contract for validator slashing.
    ///
    /// Source: `bkc/consensus/clique/contract/abi.go` (slashABI)
    #[allow(missing_docs)]
    interface ISlashManager {
        /// Slashes a validator for the given span.
        function slash(address signer, uint256 span) external;

        /// Checks if a signer has been slashed in the given span.
        function isSignerSlashed(address signer, uint256 span) external view returns (bool);
    }
}
