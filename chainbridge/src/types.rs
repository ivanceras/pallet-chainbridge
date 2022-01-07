use codec::{Decode, Encode, EncodeLike};
use frame_support::inherent::*;
use frame_support::pallet_prelude::*;

pub type ChainId = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
pub struct ProposalVotes<AccountId, BlockNumber> {
    pub votes_for: Vec<AccountId>,
    pub expiry: BlockNumber,
}
