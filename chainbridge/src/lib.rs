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

#[frame_support::pallet]
pub mod pallet {
    use crate::types::{ChainId, DepositNonce, ResourceId};
    use frame_support::inherent::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::U256;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Origin used to administer the pallet
        //type AdminOrigin: EnsureOrigin<Self::Origin>;
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

    #[pallet::storage]
    #[pallet::getter(fn resources)]
    pub type Resources<T: Config> = StorageMap<_, Blake2_256, ResourceId, Vec<u8>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn chains)]
    pub type ChainNonces<T: Config> =
        StorageMap<_, Blake2_256, ChainId, Option<DepositNonce>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn relayers)]
    pub type Relayers<T: Config> = StorageMap<_, Blake2_256, T::AccountId, bool, ValueQuery>;

    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/v3/runtime/events-and-errors
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

    // Errors inform users that something went wrong.
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

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
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

        #[pallet::weight(10_000)]
        pub fn whitelist_chain(origin: OriginFor<T>, id: ChainId) -> DispatchResult {
            log::info!("whitelisting chain_id {:?}", id);
            Self::ensure_admin(origin)?;
            Self::whitelist(id)?;
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        // ** Utility methods ***

        pub fn ensure_admin(origin: OriginFor<T>) -> DispatchResult {
            /*
            T::AdminOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;
            */
            Ok(())
        }
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
            ensure!(id != T::ChainId::get(), Error::<T>::InvalidChainId);
            ensure!(
                !Self::chain_whitelisted(id),
                Error::<T>::ChainAlreadyWhitelisted
            );
            <ChainNonces<T>>::insert(&id, Some(0));
            Self::deposit_event(Event::ChainWhitelisted(id));
            Ok(())
        }

        /// Checks if a chain exists as a whitelisted destination
        pub fn chain_whitelisted(id: ChainId) -> bool {
            return Self::chains(id) != None;
        }

        pub fn is_relayer(who: &T::AccountId) -> bool {
            Self::relayers(who)
        }

        /// Adds a new relayer to the set
        pub fn register_relayer(relayer: T::AccountId) -> DispatchResult {
            ensure!(
                !Self::is_relayer(&relayer),
                Error::<T>::RelayerAlreadyExists
            );
            <Relayers<T>>::insert(&relayer, true);
            Ok(())
        }
    }
}
