#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    pub type MaxValidators = frame_support::traits::ConstU32<100>;
    pub type MaxCheckers = frame_support::traits::ConstU32<100>;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct PriceFixProposal<AccountId> {
        pub id: u64,
        pub proposer: AccountId,
        pub token: u64,
        pub index: u64,
        pub correct_price: u64,
        pub original_index: u64,
        pub finalized: bool,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct Vote<AccountId> {
        pub voter: AccountId,
        pub proposal_id: u64,
        pub approve: bool,
        pub is_checker: bool,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type Validators<T: Config> = StorageValue<_, BoundedVec<T::AccountId, MaxValidators>, ValueQuery>;

    #[pallet::storage]
    pub type Checkers<T: Config> = StorageValue<_, BoundedVec<T::AccountId, MaxCheckers>, ValueQuery>;

    #[pallet::storage]
    pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, u64, PriceFixProposal<T::AccountId>>;

    #[pallet::storage]
    pub type Votes<T: Config> = StorageDoubleMap<_, Blake2_128Concat, u64, Blake2_128Concat, T::AccountId, Vote<T::AccountId>>;

    #[pallet::storage]
    pub type NextProposalId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ProposalSubmitted { id: u64, proposer: T::AccountId },
        Voted { proposal: u64, voter: T::AccountId, approve: bool },
        Finalized { id: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotValidator,
        NotParticipant,
        DuplicateVote,
        ProposalNotFound,
        AlreadyFinalized,
        ThresholdNotMet,
    }

    impl<T: Config> Pallet<T> {
        pub fn is_validator(who: &T::AccountId) -> bool {
            Validators::<T>::get().iter().any(|v| v == who)
        }
        pub fn is_checker(who: &T::AccountId) -> bool {
            Checkers::<T>::get().iter().any(|v| v == who)
        }

        fn next_proposal_id() -> u64 {
            let id = NextProposalId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextProposalId::<T>::put(id.saturating_add(1));
            id
        }

        fn approved(proposal_id: u64) -> bool {
            let validators_total = Validators::<T>::get().len();
            let checkers_total = Checkers::<T>::get().len();
            let mut v_approve = 0usize;
            let mut c_approve = 0usize;
            for (_voter, vote) in Votes::<T>::iter_prefix(proposal_id) {
                if vote.is_checker {
                    if vote.approve {
                        c_approve += 1;
                    }
                } else if vote.approve {
                    v_approve += 1;
                }
            }
            let v_ok = v_approve * 3 >= validators_total * 2;
            let c_ok = c_approve * 3 >= checkers_total * 2;
            v_ok && c_ok
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn propose_price_fix(
            origin: OriginFor<T>,
            token: u64,
            index: u64,
            correct_price: u64,
            original_index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            if !Self::is_validator(&who) {
                return Err(Error::<T>::NotValidator.into());
            }
            let id = Self::next_proposal_id();
            let proposal = PriceFixProposal {
                id,
                proposer: who.clone(),
                token,
                index,
                correct_price,
                original_index,
                finalized: false,
            };
            Proposals::<T>::insert(id, proposal);
            Self::deposit_event(Event::ProposalSubmitted {
                id,
                proposer: who,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn vote(origin: OriginFor<T>, proposal_id: u64, approve: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let is_checker = if Self::is_checker(&who) {
                true
            } else if Self::is_validator(&who) {
                false
            } else {
                return Err(Error::<T>::NotParticipant.into());
            };
            if !Proposals::<T>::contains_key(proposal_id) {
                return Err(Error::<T>::ProposalNotFound.into());
            }
            if Votes::<T>::contains_key(proposal_id, &who) {
                return Err(Error::<T>::DuplicateVote.into());
            }
            let vote = Vote {
                voter: who.clone(),
                proposal_id,
                approve,
                is_checker,
            };
            Votes::<T>::insert(proposal_id, &who, vote);
            Self::deposit_event(Event::Voted {
                proposal: proposal_id,
                voter: who,
                approve,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn finalize_price_fix(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;
            if proposal.finalized {
                return Err(Error::<T>::AlreadyFinalized.into());
            }
            if !Self::approved(proposal_id) {
                return Err(Error::<T>::ThresholdNotMet.into());
            }
            proposal.finalized = true;
            Proposals::<T>::insert(proposal_id, proposal);
            Self::deposit_event(Event::Finalized { id: proposal_id });
            Ok(())
        }
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub validators: Vec<T::AccountId>,
        pub checkers: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            Validators::<T>::put(BoundedVec::<T::AccountId, MaxValidators>::try_from(self.validators.clone()).unwrap_or_default());
            Checkers::<T>::put(BoundedVec::<T::AccountId, MaxCheckers>::try_from(self.checkers.clone()).unwrap_or_default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as pallet_set_consensus;

    use frame_support::{assert_ok, assert_noop, derive_impl, construct_runtime};
    use sp_io::TestExternalities;
    use sp_runtime::BuildStorage;

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            Consensus: pallet,
        }
    );

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Test {
        type Block = Block;
    }

    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
    }

    fn new_test_ext() -> TestExternalities {
        let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
        pallet::GenesisConfig::<Test> {
            validators: vec![1, 2, 3],
            checkers: vec![4, 5, 6],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        TestExternalities::new(t)
    }

    #[test]
    fn proposal_vote_finalize() {
        new_test_ext().execute_with(|| {
            assert_ok!(pallet::Pallet::<Test>::propose_price_fix(frame_system::RawOrigin::Signed(1).into(), 1, 1, 5, 1));
            assert_ok!(pallet::Pallet::<Test>::vote(frame_system::RawOrigin::Signed(1).into(), 1, true));
            assert_ok!(pallet::Pallet::<Test>::vote(frame_system::RawOrigin::Signed(2).into(), 1, true));
            assert_ok!(pallet::Pallet::<Test>::vote(frame_system::RawOrigin::Signed(4).into(), 1, true));
            assert_ok!(pallet::Pallet::<Test>::vote(frame_system::RawOrigin::Signed(5).into(), 1, true));
            assert_ok!(pallet::Pallet::<Test>::finalize_price_fix(frame_system::RawOrigin::Signed(9).into(), 1));
            let p = Proposals::<Test>::get(1).unwrap();
            assert!(p.finalized);
        });
    }

    #[test]
    fn duplicate_vote_rejected() {
        new_test_ext().execute_with(|| {
            assert_ok!(pallet::Pallet::<Test>::propose_price_fix(frame_system::RawOrigin::Signed(1).into(), 1, 1, 5, 1));
            assert_ok!(pallet::Pallet::<Test>::vote(frame_system::RawOrigin::Signed(1).into(), 1, true));
            assert_noop!(
                pallet::Pallet::<Test>::vote(frame_system::RawOrigin::Signed(1).into(), 1, true),
                Error::<Test>::DuplicateVote
            );
        });
    }

    #[test]
    fn finalize_requires_threshold() {
        new_test_ext().execute_with(|| {
            assert_ok!(pallet::Pallet::<Test>::propose_price_fix(frame_system::RawOrigin::Signed(1).into(), 1, 1, 5, 1));
            assert_noop!(
                pallet::Pallet::<Test>::finalize_price_fix(frame_system::RawOrigin::Signed(9).into(), 1),
                Error::<Test>::ThresholdNotMet
            );
        });
    }
}
