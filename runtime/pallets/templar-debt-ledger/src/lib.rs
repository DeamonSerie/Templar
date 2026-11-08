#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use templar_types::{calculate_simple_interest, LedgerSchedules, Schedule};

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, Default)]
    pub struct LoanRecord<AccountId> {
        pub loan_id: u64,
        pub ledger_id: u64,
        pub borrower: AccountId,
        pub amount: u128,
        pub interest_rate_bps: u32,
        pub borrowed_at_tick: u128,
        pub loan_count: u32,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum DebtEvent<AccountId> {
        Deposit {
            who: AccountId,
            amount: u128,
            tick: u128,
        },
        LoanIssued {
            loan_id: u64,
            borrower: AccountId,
            requested_amount: u128,
            disbursed_amount: u128,
            interest_amount: u128,
            currency_token: u64,
            tick: u128,
        },
        LoanMatured {
            loan_id: u64,
            tick: u128,
        },
        LoanabilityReset {
            from: AccountId,
            to: AccountId,
            previous_loanability: u128,
            new_loanability: u128,
            tick: u128,
        },
    }

    pub type MaxDebtEvents = frame_support::traits::ConstU32<256>;
    pub type MaxLedgerLoans = frame_support::traits::ConstU32<128>;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Ledger<T: Config> {
        pub id: u64,
        pub owner: T::AccountId,
        pub depositor: T::AccountId,
        pub token: u64,
        pub schedules: LedgerSchedules,
        pub balance: u128,
        pub frozen: bool,
        pub burned: bool,
        pub matured: bool,
        pub deposit_amount: u128,
        pub debt_balance: u128,
        pub loan_count: u32,
        pub interest_rate_bps: u32,
        pub loaned_currency_token: u64,
        pub deposit_loanability: u128,
        pub total_minted_from_ledger: u128,
        pub debt_history: BoundedVec<DebtEvent<T::AccountId>, MaxDebtEvents>,
        pub loans: BoundedVec<LoanRecord<T::AccountId>, MaxLedgerLoans>,
    }

    impl<T: Config> Ledger<T> {
        pub fn calculate_interest(&self, amount: u128) -> u128 {
            calculate_simple_interest(amount, self.interest_rate_bps)
        }

        pub fn push_event(&mut self, event: DebtEvent<T::AccountId>) {
            let _ = self.debt_history.try_push(event);
        }

        pub fn push_loan(&mut self, loan: LoanRecord<T::AccountId>) {
            let _ = self.loans.try_push(loan);
        }
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
    pub type Ledgers<T: Config> = StorageMap<_, Blake2_128Concat, u64, Ledger<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        LedgerCreated { id: u64, owner: T::AccountId },
        Deposited { ledger: u64, who: T::AccountId, amount: u128 },
        Withdrawn { ledger: u64, who: T::AccountId, amount: u128 },
        Frozen { ledger: u64 },
        LoanIssued {
            ledger: u64,
            loan_id: u64,
            borrower: T::AccountId,
            requested_amount: u128,
            disbursed_amount: u128,
            currency_token: u64,
        },
        LoanMatured { ledger: u64, loan_id: u64 },
        LoanabilityReset { ledger: u64, from: T::AccountId, to: T::AccountId },
    }

    #[pallet::error]
    pub enum Error<T> {
        LedgerNotFound,
        InsufficientBalance,
        ScheduleNotAllowed,
        NotOwner,
        NotDepositor,
        Frozen,
        Burned,
        Matured,
        InvalidLoanAmount,
        InsufficientLoanability,
        LoanRecordsFull,
        HistoryFull,
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

        fn tan_result(_qid: Option<u64>) -> bool {
            true
        }

        pub fn schedule_allows(sched: &Schedule, now: u128) -> bool {
            sched.allows(now, true)
        }

        pub fn schedule_allows_with_tan(sched: &Schedule, now: u128, tan_result: bool) -> bool {
            sched.allows(now, tan_result)
        }

        pub fn get_ledger(id: u64) -> Option<Ledger<T>> {
            Ledgers::<T>::get(id)
        }

        pub fn put_ledger(ledger: Ledger<T>) {
            Ledgers::<T>::insert(ledger.id, ledger);
        }

        pub fn validate_loan_target(ledger: &Ledger<T>) -> DispatchResult {
            ensure!(!ledger.frozen, Error::<T>::Frozen);
            ensure!(!ledger.burned, Error::<T>::Burned);
            ensure!(!ledger.matured, Error::<T>::Matured);
            Ok(())
        }

        pub fn reset_loanability(ledger_id: u64, to: T::AccountId) -> DispatchResult {
            let mut ledger = Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            let previous = ledger.deposit_loanability;
            let from = ledger.owner.clone();
            let new_loanability = ledger.deposit_amount.saturating_mul(70) / 100;
            ledger.owner = to.clone();
            ledger.depositor = to.clone();
            ledger.deposit_loanability = new_loanability;
            ledger.total_minted_from_ledger = new_loanability;
            ledger.loan_count = 0;
            ledger.loans = BoundedVec::default();
            ledger.matured = false;
            let tick = Self::now();
            ledger.push_event(DebtEvent::LoanabilityReset {
                from: from.clone(),
                to: to.clone(),
                previous_loanability: previous,
                new_loanability,
                tick,
            });
            Self::put_ledger(ledger);
            Self::deposit_event(Event::LoanabilityReset { ledger: ledger_id, from, to });
            Ok(())
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn create_ledger(
            origin: OriginFor<T>,
            token: u64,
            schedules: LedgerSchedules,
            deposit_amount: u128,
            interest_rate_bps: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let id = Self::next_ledger_id();
            let deposit_loanability = deposit_amount.saturating_mul(70) / 100;
            let ledger = Ledger::<T> {
                id,
                owner: who.clone(),
                depositor: who.clone(),
                token,
                schedules,
                balance: 0,
                frozen: false,
                burned: false,
                matured: false,
                deposit_amount,
                debt_balance: 0,
                loan_count: 0,
                interest_rate_bps,
                loaned_currency_token: token,
                deposit_loanability,
                total_minted_from_ledger: deposit_loanability,
                debt_history: BoundedVec::default(),
                loans: BoundedVec::default(),
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
            if !ledger
                .schedules
                .deposit
                .allows(now, Self::tan_result(ledger.schedules.deposit.tan_query_id))
            {
                return Err(Error::<T>::ScheduleNotAllowed.into());
            }
            let bal = pallet_set_pricing::TokenBalances::<T>::get(ledger.token, &who).unwrap_or(0);
            ensure!(bal >= amount, Error::<T>::InsufficientBalance);
            pallet_set_pricing::TokenBalances::<T>::mutate(ledger.token, &who, |b| {
                *b = Some(b.unwrap_or(0).saturating_sub(amount));
            });
            ledger.balance = ledger.balance.saturating_add(amount);
            ledger.deposit_amount = ledger.deposit_amount.saturating_add(amount);
            ledger.deposit_loanability = ledger
                .deposit_loanability
                .saturating_add(amount.saturating_mul(70) / 100);
            ledger.schedules.deposit.executed_count =
                ledger.schedules.deposit.executed_count.saturating_add(1);
            ledger.push_event(DebtEvent::Deposit {
                who: who.clone(),
                amount,
                tick: now,
            });
            Ledgers::<T>::insert(ledger_id, ledger);
            Self::deposit_event(Event::Deposited {
                ledger: ledger_id,
                who,
                amount,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn depositor_withdraw(origin: OriginFor<T>, ledger_id: u64, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut ledger = Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            ensure!(ledger.depositor == who, Error::<T>::NotDepositor);
            ensure!(!ledger.frozen, Error::<T>::Frozen);
            let now = Self::now();
            if !ledger
                .schedules
                .withdrawal
                .allows(now, Self::tan_result(ledger.schedules.withdrawal.tan_query_id))
            {
                return Err(Error::<T>::ScheduleNotAllowed.into());
            }
            ensure!(ledger.balance >= amount, Error::<T>::InsufficientBalance);
            ledger.balance = ledger.balance.saturating_sub(amount);
            pallet_set_pricing::TokenBalances::<T>::mutate(ledger.token, &who, |b| {
                *b = Some(b.unwrap_or(0).saturating_add(amount));
            });
            ledger.schedules.withdrawal.executed_count =
                ledger.schedules.withdrawal.executed_count.saturating_add(1);
            Ledgers::<T>::insert(ledger_id, ledger);
            Self::deposit_event(Event::Withdrawn {
                ledger: ledger_id,
                who,
                amount,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn freeze_for_yield_split(origin: OriginFor<T>, ledger_id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut ledger = Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            ensure!(ledger.owner == who, Error::<T>::NotOwner);
            ledger.frozen = true;
            Ledgers::<T>::insert(ledger_id, ledger);
            Self::deposit_event(Event::Frozen { ledger: ledger_id });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn transfer_ledger(origin: OriginFor<T>, ledger_id: u64, to: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let ledger = Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            ensure!(ledger.owner == who, Error::<T>::NotOwner);
            ensure!(!ledger.frozen, Error::<T>::Frozen);
            ensure!(!ledger.burned, Error::<T>::Burned);
            Self::reset_loanability(ledger_id, to)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as pallet_templar_debt_ledger;
    use frame_support::{assert_noop, assert_ok, derive_impl};
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
        frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .into()
    }

    fn scheds(start: u128, interval: u128, max: u32) -> LedgerSchedules {
        let s = Schedule::new(start, interval, max, None);
        LedgerSchedules {
            deposit: s.clone(),
            withdrawal: s.clone(),
            loan: s.clone(),
            repayment: s,
        }
    }

    #[test]
    fn create_and_schedule_gate() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let s = scheds(10, 0, 0);
            assert_ok!(pallet::Pallet::<Test>::create_ledger(
                frame_system::RawOrigin::Signed(1).into(),
                1,
                s,
                0,
                500
            ));
            assert_noop!(
                pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5),
                Error::<Test>::ScheduleNotAllowed
            );
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            pallet_set_pricing::TokenBalances::<Test>::insert(1, 1, 100u128);
            assert_ok!(pallet::Pallet::<Test>::deposit(
                frame_system::RawOrigin::Signed(1).into(),
                1,
                5
            ));
        });
    }

    #[test]
    fn off_boundary() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                10
            ));
            let s = scheds(0, 5, 0);
            assert_ok!(pallet::Pallet::<Test>::create_ledger(
                frame_system::RawOrigin::Signed(1).into(),
                1,
                s,
                0,
                500
            ));
            pallet_set_pricing::TokenBalances::<Test>::insert(1, 1, 100u128);
            assert_ok!(pallet::Pallet::<Test>::deposit(
                frame_system::RawOrigin::Signed(1).into(),
                1,
                5
            ));
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                1
            ));
            assert_noop!(
                pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5),
                Error::<Test>::ScheduleNotAllowed
            );
        });
    }

    #[test]
    fn max_occurrences() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                10
            ));
            let s = scheds(0, 0, 1);
            assert_ok!(pallet::Pallet::<Test>::create_ledger(
                frame_system::RawOrigin::Signed(1).into(),
                1,
                s,
                0,
                500
            ));
            pallet_set_pricing::TokenBalances::<Test>::insert(1, 1, 100u128);
            assert_ok!(pallet::Pallet::<Test>::deposit(
                frame_system::RawOrigin::Signed(1).into(),
                1,
                5
            ));
            assert_noop!(
                pallet::Pallet::<Test>::deposit(frame_system::RawOrigin::Signed(1).into(), 1, 5),
                Error::<Test>::ScheduleNotAllowed
            );
        });
    }
}
