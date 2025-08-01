#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, LockableCurrency, LockIdentifier},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Zero, StaticLookup};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId> + LockableCurrency<Self::AccountId>;
        type WeightInfo: WeightInfo;
        type MinProposalDeposit: Get<BalanceOf<Self>>;
        type MaxProposalDescription: Get<u32>;
        type VotingPeriod: Get<BlockNumberFor<Self>>;
        type ExecutionDelay: Get<BlockNumberFor<Self>>;
        type MinVotes: Get<u32>;
        type MaxActiveProposals: Get<u32>;
        type TreasuryAccount: Get<Self::AccountId>;
        type GovernanceLockId: Get<LockIdentifier>;
    }

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    pub type LockIdFor = LockIdentifier;

    #[derive(Clone, Encode, Decode, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        pub id: u32,
        pub proposer: T::AccountId,
        pub description: BoundedVec<u8, ConstU32<1024>>,
        pub amount: BalanceOf<T>,
        pub recipient: T::AccountId,
        pub yes_votes: u32,
        pub no_votes: u32,
        pub start_block: BlockNumberFor<T>,
        pub end_block: BlockNumberFor<T>,
        pub executed: bool,
        pub cancelled: bool,
    }

    pub type Vote = u8;
    pub const VOTE_YES: Vote = 1;
    pub const VOTE_NO: Vote = 0;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn next_proposal_id)]
    pub type NextProposalId<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, u32, Proposal<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_proposals)]
    pub type ActiveProposals<T> = StorageValue<_, BoundedVec<u32, ConstU32<100>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn votes)]
    pub type Votes<T: Config> = StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, Vote, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn treasury_balance)]
    pub type TreasuryBalance<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn member_voting_power)]
    pub type MemberVotingPower<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_voting_power)]
    pub type TotalVotingPower<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ProposalCreated { proposal_id: u32, proposer: T::AccountId, amount: BalanceOf<T> },
        Voted { proposal_id: u32, voter: T::AccountId, vote: Vote },
        ProposalExecuted { proposal_id: u32, recipient: T::AccountId, amount: BalanceOf<T> },
        ProposalCancelled { proposal_id: u32 },
        TreasuryFunded { amount: BalanceOf<T> },
        MemberAdded { account: T::AccountId, voting_power: u32 },
        MemberRemoved { account: T::AccountId },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientBalance,
        ProposalNotFound,
        ProposalAlreadyExecuted,
        ProposalAlreadyCancelled,
        VotingPeriodNotEnded,
        AlreadyVoted,
        InsufficientVotes,
        TooManyActiveProposals,
        InvalidProposalAmount,
        NotEnoughVotingPower,
        TreasuryInsufficientFunds,
        InvalidRecipient,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::create_proposal())]
        pub fn create_proposal(
            origin: OriginFor<T>,
            description: BoundedVec<u8, ConstU32<1024>>,
            amount: BalanceOf<T>,
            recipient: <<T as frame_system::Config>::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            let recipient = T::Lookup::lookup(recipient)?;

            ensure!(amount > Zero::zero(), Error::<T>::InvalidProposalAmount);


            let active_proposals = ActiveProposals::<T>::get();
            ensure!(
                active_proposals.len() < T::MaxActiveProposals::get() as usize,
                Error::<T>::TooManyActiveProposals
            );

            let voting_power = MemberVotingPower::<T>::get(&proposer);
            ensure!(voting_power > 0, Error::<T>::NotEnoughVotingPower);

            // Lock proposal deposit
            T::Currency::withdraw(
                &proposer,
                T::MinProposalDeposit::get(),
                frame_support::traits::WithdrawReasons::RESERVE,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            let proposal_id = NextProposalId::<T>::mutate(|id| {
                *id += 1;
                *id
            });

            let now = frame_system::Pallet::<T>::block_number();
            let end_block = now + T::VotingPeriod::get();



            let proposal = Proposal {
                id: proposal_id,
                proposer: proposer.clone(),
                description,
                amount,
                recipient,
                yes_votes: 0,
                no_votes: 0,
                start_block: now,
                end_block,
                executed: false,
                cancelled: false,
            };

            Proposals::<T>::insert(proposal_id, proposal);
            ActiveProposals::<T>::mutate(|active| {
                active.try_push(proposal_id).map_err(|_| Error::<T>::TooManyActiveProposals)?;
                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::ProposalCreated {
                proposal_id,
                proposer,
                amount,
            });

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::vote())]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: u32,
            vote: Vote,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;

            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(!proposal.executed, Error::<T>::ProposalAlreadyExecuted);
            ensure!(!proposal.cancelled, Error::<T>::ProposalAlreadyCancelled);
            ensure!(
                frame_system::Pallet::<T>::block_number() < proposal.end_block,
                Error::<T>::VotingPeriodNotEnded
            );

            ensure!(
                Votes::<T>::get(proposal_id, &voter).is_none(),
                Error::<T>::AlreadyVoted
            );

            let voting_power = MemberVotingPower::<T>::get(&voter);
            ensure!(voting_power > 0, Error::<T>::NotEnoughVotingPower);

            Votes::<T>::insert(proposal_id, &voter, vote.clone());

            match vote {
                VOTE_YES => proposal.yes_votes += voting_power,
                VOTE_NO => proposal.no_votes += voting_power,
                _ => return Err(Error::<T>::InvalidRecipient.into()),
            }

            Proposals::<T>::insert(proposal_id, proposal);

            Self::deposit_event(Event::Voted {
                proposal_id,
                voter,
                vote,
            });

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::execute_proposal())]
        pub fn execute_proposal(
            origin: OriginFor<T>,
            proposal_id: u32,
        ) -> DispatchResult {
            let _executor = ensure_signed(origin)?;

            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(!proposal.executed, Error::<T>::ProposalAlreadyExecuted);
            ensure!(!proposal.cancelled, Error::<T>::ProposalAlreadyCancelled);
            ensure!(
                frame_system::Pallet::<T>::block_number() >= proposal.end_block,
                Error::<T>::VotingPeriodNotEnded
            );

            let total_votes = proposal.yes_votes + proposal.no_votes;
            ensure!(total_votes >= T::MinVotes::get(), Error::<T>::InsufficientVotes);

            if proposal.yes_votes > proposal.no_votes {
                let treasury_balance = TreasuryBalance::<T>::get();
                ensure!(treasury_balance >= proposal.amount, Error::<T>::TreasuryInsufficientFunds);

                // Transfer funds from treasury to recipient
                let recipient = proposal.recipient.clone();
                let amount = proposal.amount;
                TreasuryBalance::<T>::mutate(|balance| *balance -= amount);
                T::Currency::deposit_creating(&recipient, amount);

                proposal.executed = true;
                Proposals::<T>::insert(proposal_id, proposal);

                Self::deposit_event(Event::ProposalExecuted {
                    proposal_id,
                    recipient,
                    amount,
                });
            }

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::cancel_proposal())]
        pub fn cancel_proposal(
            origin: OriginFor<T>,
            proposal_id: u32,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;

            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(proposal.proposer == proposer, Error::<T>::InvalidRecipient);
            ensure!(!proposal.executed, Error::<T>::ProposalAlreadyExecuted);
            ensure!(!proposal.cancelled, Error::<T>::ProposalAlreadyCancelled);

            proposal.cancelled = true;
            Proposals::<T>::insert(proposal_id, proposal);

            // Return deposit to proposer
            T::Currency::deposit_creating(&proposer, T::MinProposalDeposit::get());

            Self::deposit_event(Event::ProposalCancelled { proposal_id });

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::fund_treasury())]
        pub fn fund_treasury(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let funder = ensure_signed(origin)?;

            T::Currency::withdraw(
                &funder,
                amount,
                frame_support::traits::WithdrawReasons::TRANSFER,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            TreasuryBalance::<T>::mutate(|balance| *balance += amount);

            Self::deposit_event(Event::TreasuryFunded { amount });

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::add_member())]
        pub fn add_member(
            origin: OriginFor<T>,
            account: <<T as frame_system::Config>::Lookup as StaticLookup>::Source,
            voting_power: u32,
        ) -> DispatchResult {
            let _admin = ensure_signed(origin)?;
            let account = T::Lookup::lookup(account)?;

            ensure!(voting_power > 0, Error::<T>::NotEnoughVotingPower);

            MemberVotingPower::<T>::insert(&account, voting_power);
            TotalVotingPower::<T>::mutate(|total| *total += voting_power);

            Self::deposit_event(Event::MemberAdded { account, voting_power });

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::remove_member())]
        pub fn remove_member(
            origin: OriginFor<T>,
            account: <<T as frame_system::Config>::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            let _admin = ensure_signed(origin)?;
            let account = T::Lookup::lookup(account)?;

            let voting_power = MemberVotingPower::<T>::get(&account);
            ensure!(voting_power > 0, Error::<T>::NotEnoughVotingPower);

            MemberVotingPower::<T>::remove(&account);
            TotalVotingPower::<T>::mutate(|total| *total -= voting_power);

            Self::deposit_event(Event::MemberRemoved { account });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_proposal(proposal_id: u32) -> Option<Proposal<T>> {
            Proposals::<T>::get(proposal_id)
        }

        pub fn get_active_proposals() -> BoundedVec<u32, ConstU32<100>> {
            ActiveProposals::<T>::get()
        }

        pub fn get_vote(proposal_id: u32, voter: &T::AccountId) -> Option<Vote> {
            Votes::<T>::get(proposal_id, voter)
        }

        pub fn is_member(account: &T::AccountId) -> bool {
            MemberVotingPower::<T>::get(account) > 0
        }

        pub fn get_member_voting_power(account: &T::AccountId) -> u32 {
            MemberVotingPower::<T>::get(account)
        }

        pub fn get_total_voting_power() -> u32 {
            TotalVotingPower::<T>::get()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pallet::*;
    use frame_support::{
        assert_ok, assert_noop,
        parameter_types,
        traits::{ConstU32, ConstU64, ConstU128},
    };
    use frame_system as system;
    use sp_core::H256;
    use sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, IdentityLookup},
    };

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system,
            Balances: pallet_balances,
            Dao: Pallet<Test>,
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
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<u128>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = SS58Prefix;
        type OnSetCode = ();
        type MaxConsumers = ConstU32<16>;
    }

    parameter_types! {
        pub const ExistentialDeposit: u128 = 1;
    }

    impl pallet_balances::Config for Test {
        type Balance = u128;
        type DustRemoval = ();
        type RuntimeEvent = RuntimeEvent;
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
        type HoldIdentifier = ();
        type FreezeIdentifier = ();
        type MaxHolds = ConstU32<0>;
        type MaxFreezes = ConstU32<0>;
    }

    parameter_types! {
        pub const MinProposalDeposit: u128 = 10;
        pub const MaxProposalDescription: u32 = 1024;
        pub const VotingPeriod: u64 = 100;
        pub const ExecutionDelay: u64 = 10;
        pub const MinVotes: u32 = 1;
        pub const MaxActiveProposals: u32 = 10;
        pub const TreasuryAccount: u64 = 999;
        pub const GovernanceLockId: LockIdentifier = *b"gov_lock";
    }

    impl Config for Test {
        type Currency = Balances;
        type WeightInfo = ();
        type MinProposalDeposit = MinProposalDeposit;
        type MaxProposalDescription = MaxProposalDescription;
        type VotingPeriod = VotingPeriod;
        type ExecutionDelay = ExecutionDelay;
        type MinVotes = MinVotes;
        type MaxActiveProposals = MaxActiveProposals;
        type TreasuryAccount = TreasuryAccount;
        type GovernanceLockId = GovernanceLockId;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let mut t = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 1000), (3, 1000)],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        t.into()
    }

    #[test]
    fn test_create_proposal() {
        new_test_ext().execute_with(|| {
            // Add member first
            assert_ok!(Dao::add_member(RuntimeOrigin::signed(1), 1, 10));

            // Create proposal
            assert_ok!(Dao::create_proposal(
                RuntimeOrigin::signed(1),
                b"Test proposal".to_vec(),
                100,
                2
            ));

            let proposal = Dao::get_proposal(1).unwrap();
            assert_eq!(proposal.proposer, 1);
            assert_eq!(proposal.amount, 100);
            assert_eq!(proposal.recipient, 2);
            assert_eq!(proposal.executed, false);
        });
    }

    #[test]
    fn test_vote() {
        new_test_ext().execute_with(|| {
            // Add members
            assert_ok!(Dao::add_member(RuntimeOrigin::signed(1), 1, 10));
            assert_ok!(Dao::add_member(RuntimeOrigin::signed(2), 2, 5));

            // Create proposal
            assert_ok!(Dao::create_proposal(
                RuntimeOrigin::signed(1),
                b"Test proposal".to_vec(),
                100,
                3
            ));

            // Vote
            assert_ok!(Dao::vote(RuntimeOrigin::signed(1), 1, VOTE_YES));
            assert_ok!(Dao::vote(RuntimeOrigin::signed(2), 1, VOTE_NO));

            let proposal = Dao::get_proposal(1).unwrap();
            assert_eq!(proposal.yes_votes, 10);
            assert_eq!(proposal.no_votes, 5);
        });
    }
}