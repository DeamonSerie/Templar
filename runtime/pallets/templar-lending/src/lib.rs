#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use templar_types::Schedule;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct LoanRecord<AccountId> {
        pub id: u64,
        pub ledger_id: u64,
        pub borrower: AccountId,
        pub principal: u128,
        pub maturity_tick: u128,
        pub repaid: u128,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_set_time::Config + pallet_templar_debt_ledger::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type NextLoanId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type Loans<T: Config> = StorageMap<_, Blake2_128Concat, u64, LoanRecord<T::AccountId>>;

    #[pallet::storage]
    pub type LoansByLedger<T> = StorageMap<_, Blake2_128Concat, u64, BoundedVec<u64, frame_support::traits::ConstU32<100>>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        LoanCreated { id: u64, ledger: u64, amount: u128 },
        Repaid { loan: u64, amount: u128 },
    }

    #[pallet::error]
    pub enum Error<T> {
        ScheduleNotAllowed,
        LoanNotFound,
        InsufficientBalance,
        LedgerNotFound,
    }

    impl<T: Config> Pallet<T> {
        fn next_loan_id() -> u64 {
            let id = NextLoanId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextLoanId::<T>::put(id.saturating_add(1));
            id
        }

        fn now() -> u128 {
            pallet_set_time::Pallet::<T>::net_time()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn loan(origin: OriginFor<T>, ledger_id: u64, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut ledger = pallet_templar_debt_ledger::pallet::Ledgers::<T>::get(ledger_id)
                .ok_or(Error::<T>::LedgerNotFound)?;
            let now = Self::now();
            if !ledger.schedules.loan.allows(now, true) {
                return Err(Error::<T>::ScheduleNotAllowed.into());
            }
            let id = Self::next_loan_id();
            let loan = LoanRecord {
                id,
                ledger_id,
                borrower: who.clone(),
                principal: amount,
                maturity_tick: now.saturating_add(100),
                repaid: 0,
            };
            Loans::<T>::insert(id, loan);
            LoansByLedger::<T>::mutate(ledger_id, |v| {
                let v = v.get_or_insert_with(BoundedVec::default);
                let _ = v.try_push(id);
            });
            ledger.schedules.loan.executed_count = ledger.schedules.loan.executed_count.saturating_add(1);
            pallet_templar_debt_ledger::pallet::Ledgers::<T>::insert(ledger_id, ledger);
            Self::deposit_event(Event::LoanCreated { id, ledger: ledger_id, amount });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn repay(origin: OriginFor<T>, loan_id: u64, amount: u128) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            let mut loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;
            let mut ledger = pallet_templar_debt_ledger::pallet::Ledgers::<T>::get(loan.ledger_id)
                .ok_or(Error::<T>::LedgerNotFound)?;
            let now = Self::now();
            if !ledger.schedules.repayment.allows(now, true) {
                return Err(Error::<T>::ScheduleNotAllowed.into());
            }
            let lid = loan.ledger_id;
            loan.repaid = loan.repaid.saturating_add(amount);
            ledger.schedules.repayment.executed_count = ledger.schedules.repayment.executed_count.saturating_add(1);
            Loans::<T>::insert(loan_id, loan);
            pallet_templar_debt_ledger::pallet::Ledgers::<T>::insert(lid, ledger);
            Self::deposit_event(Event::Repaid { loan: loan_id, amount });
            Ok(())
        }
    }
}
