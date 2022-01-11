#![allow(warnings)]
use crate as pallet_chainbridge;
use frame_support::{
    parameter_types,
    traits::SortedMembers,
    PalletId,
};
use frame_system as system;
use frame_system::EnsureSignedBy;
use pallet_chainbridge::types::ChainId;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{
        BlakeTwo256,
        IdentityLookup,
    },
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
        Bridge: pallet_chainbridge::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type AccountData = ();
    type AccountId = u64;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockHashCount = BlockHashCount;
    type BlockLength = ();
    type BlockNumber = u64;
    type BlockWeights = ();
    type Call = Call;
    type DbWeight = ();
    type Event = Event;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Index = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type Origin = Origin;
    type PalletInfo = PalletInfo;
    type SS58Prefix = SS58Prefix;
    type SystemWeightInfo = ();
    type Version = ();
}

// Parameterize default test user identifier (with id 1)
parameter_types! {
    pub const TestUserId: u64 = 1;
    pub const TestChainId: ChainId = 5;
    pub const ProposalLifetime: u64 = 10;
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
}

impl SortedMembers<u64> for TestUserId {
    fn sorted_members() -> Vec<u64> {
        vec![1]
    }
}

impl pallet_chainbridge::Config for Test {
    type AdminOrigin = EnsureSignedBy<TestUserId, u64>;
    type ChainId = TestChainId;
    type Event = Event;
    type PalletId = ChainBridgePalletId;
    type Proposal = Call;
    type ProposalLifetime = ProposalLifetime;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<Event>) {
    let mut actual: Vec<Event> = system::Module::<Test>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();

    expected.reverse();

    for evt in expected {
        let next = actual.pop().expect("event expected");
        assert_eq!(next, evt.into(), "Events don't match (actual,expected)");
    }
}
