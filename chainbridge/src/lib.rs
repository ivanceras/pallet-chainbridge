#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use crate::types::ProposalVotes;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::types::{ChainId, DepositNonce, ResourceId};
    use crate::ProposalVotes;
    use codec::{Decode, Encode, EncodeLike};
    use frame_support::dispatch::Dispatchable;
    use frame_support::inherent::*;
    use frame_support::pallet_prelude::*;
    use frame_support::weights::GetDispatchInfo;
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::boxed::Box;
    use sp_core::U256;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Origin used to administer the pallet
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        /// Proposed dispatchable call
        type Proposal: Parameter
            + Dispatchable<Origin = Self::Origin>
            + EncodeLike
            + GetDispatchInfo;
        /// The identifier for this chain.
        /// This must be unique and must not collide with existing IDs within a set of bridged
        /// chains.
        #[pallet::constant]
        type ChainId: Get<ChainId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn relayer_threshold)]
    /// Number of votes required for a proposal to execute
    pub type RelayerThreshold<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Utilized by the bridge software to map resource IDs to actual methods
    #[pallet::storage]
    #[pallet::getter(fn resources)]
    pub type Resources<T: Config> = StorageMap<_, Blake2_256, ResourceId, Vec<u8>, ValueQuery>;

    /// All whitelisted chains and their respective transaction counts
    #[pallet::storage]
    #[pallet::getter(fn chains)]
    pub type ChainNonces<T: Config> =
        StorageMap<_, Blake2_256, ChainId, Option<DepositNonce>, ValueQuery>;

    /// Tracks current relayer set
    #[pallet::storage]
    #[pallet::getter(fn relayers)]
    pub type Relayers<T: Config> = StorageMap<_, Blake2_256, T::AccountId, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn relayer_count)]
    pub type RelayerCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Vote threshold has changed (new_threshold)
        RelayerThresholdChanged(u32),
        /// Chain now available for transfers (chain_id)
        ChainWhitelisted(ChainId),
        /// Relayer added to set
        RelayerAdded(T::AccountId),
        /// Relayer removed from set
        RelayerRemoved(T::AccountId),
        /// FunglibleTransfer is for relaying fungibles (dest_id, nonce, resource_id, amount, recipient, metadata)
        FungibleTransfer(ChainId, DepositNonce, ResourceId, U256, Vec<u8>),
        /// NonFungibleTransfer is for relaying NFTS (dest_id, nonce, resource_id, token_id, recipient, metadata)
        NonFungibleTransfer(ChainId, DepositNonce, ResourceId, Vec<u8>, Vec<u8>, Vec<u8>),
        /// GenericTransfer is for a generic data payload (dest_id, nonce, resource_id, metadata)
        GenericTransfer(ChainId, DepositNonce, ResourceId, Vec<u8>),
        /// Vote submitted in favour of proposal
        VoteFor(ChainId, DepositNonce, T::AccountId),
        /// Vot submitted against proposal
        VoteAgainst(ChainId, DepositNonce, T::AccountId),
        /// Voting successful for a proposal
        ProposalApproved(ChainId, DepositNonce),
        /// Voting rejected a proposal
        ProposalRejected(ChainId, DepositNonce),
        /// Execution of call succeeded
        ProposalSucceeded(ChainId, DepositNonce),
        /// Execution of call failed
        ProposalFailed(ChainId, DepositNonce),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Relayer threshold not set
        ThresholdNotSet,
        /// Provided chain Id is not valid
        InvalidChainId,
        /// Relayer threshold cannot be 0
        InvalidThreshold,
        /// Interactions with this chain is not permitted
        ChainNotWhitelisted,
        /// Chain has already been enabled
        ChainAlreadyWhitelisted,
        /// Resource ID provided isn't mapped to anything
        ResourceDoesNotExist,
        /// Relayer already in set
        RelayerAlreadyExists,
        /// Provided accountId is not a relayer
        RelayerInvalid,
        /// Protected operation, must be performed by relayer
        MustBeRelayer,
        /// Relayer has already submitted some vote for this proposal
        RelayerAlreadyVoted,
        /// A proposal with these parameters has already been submitted
        ProposalAlreadyExists,
        /// No proposal with the ID was found
        ProposalDoesNotExist,
        /// Cannot complete proposal, needs more votes
        ProposalNotComplete,
        /// Proposal has either failed or succeeded
        ProposalAlreadyComplete,
        /// Lifetime of proposal has been exceeded
        ProposalExpired,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the vote threshold for proposals.
        ///
        /// This threshold is used to determine how many votes are required
        /// before a proposal is executed.
        ///
        /// # <weight>
        /// - O(1) lookup and insert
        /// # </weight>
        #[pallet::weight(10_000)]
        pub fn set_threshold(origin: OriginFor<T>, threshold: u32) -> DispatchResult {
            log::info!("--->>>> Threshold is now set to: {}", threshold);
            Self::ensure_admin(origin)?;
            Self::set_relayer_threshold(threshold)?;
            Ok(())
        }

        /// Stores a method name on chain under an associated resource ID.
        ///
        /// # <weight>
        /// - O(1) write
        /// # </weight>
        #[pallet::weight(10_000)]
        pub fn set_resource(
            origin: OriginFor<T>,
            id: ResourceId,
            method: Vec<u8>,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::register_resource(id, method)?;
            Ok(())
        }

        /// Removes a resource ID from the resource mapping.
        ///
        /// After this call, bridge transfers with the associated resource ID will
        /// be rejected.
        ///
        /// # <weight>
        /// - O(1) removeal
        /// # </weight>
        #[pallet::weight(10_000)]
        pub fn remove_resource(origin: OriginFor<T>, id: ResourceId) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::unregister_resource(id)?;
            Ok(())
        }

        /// Adds a new relayer to the relayer set.
        ///
        /// # <weight>
        /// - O(1) lookup and insert
        /// # </weight>
        #[pallet::weight(10_000)]
        pub fn whitelist_chain(origin: OriginFor<T>, id: ChainId) -> DispatchResult {
            log::info!("whitelisting chain_id {:?}", id);
            Self::ensure_admin(origin)?;
            Self::whitelist(id)?;
            Ok(())
        }

        /// Adds a new relayer to the relayer set.
        ///
        /// # <weight>
        /// - O(1) lookup and removal
        /// # </weight>
        #[pallet::weight(10_000)]
        pub fn add_relayer(origin: OriginFor<T>, v: T::AccountId) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::register_relayer(v)?;
            Ok(())
        }

        /// Removes an existing relaye to the set.
        ///
        /// # <weight>
        /// - O(1) lookup and removal
        /// # </weight>
        #[pallet::weight(10_0000)]
        pub fn remove_relayer(origin: OriginFor<T>, v: T::AccountId) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::unregister_relayer(v)?;
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        // ** Utility methods ***

        pub fn ensure_admin(origin: OriginFor<T>) -> DispatchResult {
            T::AdminOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;
            Ok(())
        }

        /// Checks if who is a relayer
        pub fn is_relayer(who: &T::AccountId) -> bool {
            Self::relayers(who)
        }

        /// Checks if a chain exists as a whitelisted destination
        pub fn chain_whitelisted(id: ChainId) -> bool {
            return Self::chains(id) != None;
        }

        /// Increments the deposit nonce for the specified chain ID
        fn bump_nonce(id: ChainId) -> DepositNonce {
            //TODO: use saturating_add here
            let nonce = Self::chains(id).unwrap_or_default() + 1;
            <ChainNonces<T>>::insert(id, Some(nonce));
            nonce
        }

        // *** Admin methods ****

        /// Set a new voting threshold
        pub fn set_relayer_threshold(threshold: u32) -> DispatchResult {
            ensure!(threshold > 0, Error::<T>::InvalidThreshold);
            <RelayerThreshold<T>>::put(threshold);
            Self::deposit_event(Event::RelayerThresholdChanged(threshold));
            Ok(())
        }

        /// Register a method for a resource Id, enabling associated transfer
        pub fn register_resource(id: ResourceId, method: Vec<u8>) -> DispatchResult {
            <Resources<T>>::insert(id, method);
            Ok(())
        }

        /// Removes a resource ID, disabling associated transfer
        pub fn unregister_resource(id: ResourceId) -> DispatchResult {
            <Resources<T>>::remove(id);
            Ok(())
        }

        /// Whitelist a chain ID for transfer
        pub fn whitelist(id: ChainId) -> DispatchResult {
            // Cannot whitelist this chain
            ensure!(id != T::ChainId::get(), Error::<T>::InvalidChainId);
            // Cannot whitelist with an existing entry
            ensure!(
                !Self::chain_whitelisted(id),
                Error::<T>::ChainAlreadyWhitelisted
            );
            <ChainNonces<T>>::insert(&id, Some(0));
            Self::deposit_event(Event::ChainWhitelisted(id));
            Ok(())
        }

        /// Adds a new relayer to the set
        pub fn register_relayer(relayer: T::AccountId) -> DispatchResult {
            ensure!(
                !Self::is_relayer(&relayer),
                Error::<T>::RelayerAlreadyExists
            );
            <Relayers<T>>::insert(&relayer, true);
            //TODO: use saturating_add
            <RelayerCount<T>>::mutate(|i| *i += 1);
            Self::deposit_event(Event::RelayerAdded(relayer));
            Ok(())
        }

        /// Removes a relayer from the set
        pub fn unregister_relayer(relayer: T::AccountId) -> DispatchResult {
            ensure!(Self::is_relayer(&relayer), Error::<T>::RelayerInvalid);
            <Relayers<T>>::remove(&relayer);
            //TODO: use saturating_sub
            <RelayerCount<T>>::mutate(|i| *i -= 1);
            Self::deposit_event(Event::RelayerRemoved(relayer));
            Ok(())
        }

        /// Initiates a transfer of a fungible asset out of the chain. This should be called by
        /// another pallet
        pub fn transfer_fungible(
            dest_id: ChainId,
            resource_id: ResourceId,
            to: Vec<u8>,
            amount: U256,
        ) -> DispatchResult {
            ensure!(
                Self::chain_whitelisted(dest_id),
                Error::<T>::ChainNotWhitelisted
            );
            let nonce = Self::bump_nonce(dest_id);
            Self::deposit_event(Event::FungibleTransfer(
                dest_id,
                nonce,
                resource_id,
                amount,
                to,
            ));
            Ok(())
        }

        /// Initiates a transfer of a nunfungible asset out of the chain. This should be called by
        /// another pallet
        pub fn transfer_nonfungible(
            dest_id: ChainId,
            resource_id: ResourceId,
            token_id: Vec<u8>,
            to: Vec<u8>,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            ensure!(
                Self::chain_whitelisted(dest_id),
                Error::<T>::ChainNotWhitelisted
            );
            let nonce = Self::bump_nonce(dest_id);
            Self::deposit_event(Event::NonFungibleTransfer(
                dest_id,
                nonce,
                resource_id,
                token_id,
                to,
                metadata,
            ));
            Ok(())
        }

        /// Initiates a transfer of generic data out of the chain. This should be called by
        /// another pallet.
        pub fn transfer_generic(
            dest_id: ChainId,
            resource_id: ResourceId,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            ensure!(
                Self::chain_whitelisted(dest_id),
                Error::<T>::ChainNotWhitelisted
            );
            let nonce = Self::bump_nonce(dest_id);
            Self::deposit_event(Event::GenericTransfer(
                dest_id,
                nonce,
                resource_id,
                metadata,
            ));
            Ok(())
        }
    }
}
