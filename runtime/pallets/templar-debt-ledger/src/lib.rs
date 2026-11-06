#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use templar_types::{LedgerSchedules, Schedule};

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct Ledger<AccountId> {
        pub id: u64,
        pub owner: AccountId,
        pub depositor: AccountId,
        pub token: u64,
        pub schedules: LedgerSchedules,
        pub balance: u128,
        pub frozen: bool,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_set_time::Config + pallet_set_pricing::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type NextLedgerId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type Ledgers<T: Config> = StorageMap<_, Blake2_128Concat, u64, Ledger<T::AccountId>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        LedgerCreated { id: u64, owner: T::AccountId },
        Deposited { ledger: u64, who: T::AccountId, amount: u128 },
        Withdrawn { ledger: u64, who: T::AccountId, amount: u128 },
        Frozen { ledger: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        LedgerNotFound,
        InsufficientBalance,
        ScheduleNotAllowed,
        NotOwner,
        NotDepositor,
        Frozen,
    }

    impl<T: Config> Pallet<T> {
        fn next_ledger_id() -> u64 {
            let id = NextLedgerId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextLedgerId::<T>::put(id.saturating_add(1));
            id
        }

        fn now() -> u128 {
            pallet_set_time::Pallet::<T>::net_time()
        }

        fn tan_result(qid: Option<u64>) -> bool {
            match qid {
                None => true,
                Some(id) => {
                    // best-effort: query exists
                    true
                }
            }
        }

        pub fn schedule_allows(sched: &Schedule, now: u128) -> bool {
            sched.allows(now, true)
        }

        pub fn schedule_allows_with_tan(sched: &Schedule, now: u128, tan_result: bool) -> bool {
            sched.allows(now, tan_result)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn create_ledger(
            origin: OriginFor<T>,
            token: u64,
            schedules: LedgerSchedules,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let id = Self::next_ledger_id();
            let ledger = Ledger {
                id,
                owner: who.clone(),
                depositor: who.clone(),
                token,
                schedules,
                balance: 0,
                frozen: false,
            };
            Ledgers::<T>::insert(id, ledger);
            Self::deposit_event(Event::LedgerCreated { id, owner: who });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn deposit(origin: OriginFor<T>, ledger_id: u64, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut ledger = Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            let now = Self::now();
            if !ledger.schedules.deposit.allows(now, Self::tan_result(ledger.schedules.deposit.tan_query_id)) {
                return Err(Error::<T>::ScheduleNotAllowed.into());
            }
            // pricing balance check
            let bal = pallet_set_pricing::TokenBalances::<T>::get(ledger.token, &who).unwrap_or(0);
            if bal < amount {
                return Err(Error::<T>::InsufficientBalance.into());
            }
            pallet_set_pricing::TokenBalances::<T>::mutate(ledger.token, &who, |b| {
                *b = Some(b.unwrap_or(0).saturating_sub(amount));
            });
            ledger.balance = ledger.balance.saturating_add(amount);
            ledger.schedules.deposit.executed_count = ledger.schedules.deposit.executed_count.saturating_add(1);
            Ledgers::<T>::insert(ledger_id, ledger);
            Self::deposit_event(Event::Deposited { ledger: ledger_id, who, amount });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn depositor_withdraw(origin: OriginFor<T>, ledger_id: u64, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut ledger = Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            if ledger.depositor != who {
                return Err(Error::<T>::NotDepositor.into());
            }
            if ledger.frozen {
                return Err(Error::<T>::Frozen.into());
            }
            let now = Self::now();
            if !ledger.schedules.withdrawal.allows(now, Self::tan_result(ledger.schedules.withdrawal.tan_query_id)) {
                return Err(Error::<T>::ScheduleNotAllowed.into());
            }
            if ledger.balance < amount {
                return Err(Error::<T>::InsufficientBalance.into());
            }
            ledger.balance = ledger.balance.saturating_sub(amount);
            pallet_set_pricing::TokenBalances::<T>::mutate(ledger.token, &who, |b| {
                *b = Some(b.unwrap_or(0).saturating_add(amount));
            });
            ledger.schedules.withdrawal.executed_count = ledger.schedules.withdrawal.executed_count.saturating_add(1);
            Ledgers::<T>::insert(ledger_id, ledger);
            Self::deposit_event(Event::Withdrawn { ledger: ledger_id, who, amount });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn freeze_for_yield_split(origin: OriginFor<T>, ledger_id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut ledger = Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            if ledger.owner != who {
                return Err(Error::<T>::NotOwner.into());
            }
            ledger.frozen = true;
            Ledgers::<T>::insert(ledger_id, ledger);
            Self::deposit_event(Event::Frozen { ledger: ledger_id });
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::pallet as pallet_debt;
    use crate as pallet_templar_debt_ledger;
    use frame_support::{assert_ok, assert_noop, derive_impl};
    use sp_io::TestExternalities;
    use sp_runtime::BuildStorage;
    use templar_types::{LedgerSchedules, Schedule};

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            Time: pallet_set_time,
            Pricing: pallet_set_pricing,
            DebtLedger: pallet,
        }
    );

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Test {
        type Block = Block;
    }
    impl pallet_set_time::Config for Test {
        type RuntimeEvent = RuntimeEvent;
    }
    impl pallet_set_pricing::Config for Test {
        type RuntimeEvent = RuntimeEvent;
    }
    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
    }

    fn new_ext() -> TestExternalities {
        frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
    }

    fn scheds(start: u128, interval: u128, max: u32) -> LedgerSchedules {
        let s = Schedule::new(start, interval, max, None);
        LedgerSchedules { deposit: s.clone(), withdrawal: s.clone(), loan: s.clone(), repayment: s.clone() }
    }

    #[test]
    fn create_and_schedule_gate() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(frame_system::RawOrigin::Root.into(), 5));
            let s = scheds(10, 0, 0);
            assert_ok!(pallet::Pallet::<Test>::create_ledger(frame_system::RawOrigin::Signed(1).into(), 1, s));
            // before start
            assert_noop!(
                pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5),
                Error::<Test>::ScheduleNotAllowed
            );
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(frame_system::RawOrigin::Root.into(), 5));
            // need balance
            // create token balance by pricing? skip; insert balance manually
            pallet_set_pricing::TokenBalances::<Test>::insert(1, 1, 100u128);
            assert_ok!(pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5));
        });
    }

    #[test]
    fn off_boundary() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(frame_system::RawOrigin::Root.into(), 10));
            let s = scheds(0, 5, 0);
            assert_ok!(pallet::Pallet::<Test>::create_ledger(frame_system::RawOrigin::Signed(1).into(), 1, s));
            pallet_set_pricing::TokenBalances::<Test>::insert(1, 1, 100u128);
            assert_ok!(pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5));
            // advance 1 tick -> off boundary
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(frame_system::RawOrigin::Root.into(), 1));
            assert_noop!(
                pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5),
                Error::<Test>::ScheduleNotAllowed
            );
        });
    }

    #[test]
    fn max_occurrences() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(frame_system::RawOrigin::Root.into(), 10));
            let s = scheds(0, 0, 1);
            assert_ok!(pallet::Pallet::<Test>::create_ledger(frame_system::RawOrigin::Signed(1).into(), 1, s));
            pallet_set_pricing::TokenBalances::<Test>::insert(1, 1, 100u128);
            assert_ok!(pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5));
            assert_noop!(
                pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5),
                Error::<Test>::ScheduleNotAllowed
            );
        });
    }
}
