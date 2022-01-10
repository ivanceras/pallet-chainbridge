use crate as pallet_chainbridge;
use frame_support::pallet_prelude::*;
use frame_support::parameter_types;
use frame_support::traits::SortedMembers;
use frame_support::PalletId;
use frame_system as system;
use frame_system::EnsureSignedBy;
use pallet_chainbridge::types::ChainId;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        TemplateModule: pallet_chainbridge::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

// Parameterize default test user identifier (with id 1)
parameter_types! {
    pub const TestUserId: u64 = 1;
    pub const MockChainId: ChainId = 5;
    pub const ProposalLifetime: u64 = 10;
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
}

impl SortedMembers<u64> for TestUserId {
    fn sorted_members() -> Vec<u64> {
        vec![1]
    }
}

impl pallet_chainbridge::Config for Test {
    type Event = Event;
    type AdminOrigin = EnsureSignedBy<TestUserId, u64>;
    type ChainId = MockChainId;
    type Proposal = Call;
    type ProposalLifetime = ProposalLifetime;
    type PalletId = ChainBridgePalletId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}
