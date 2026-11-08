#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_templar_debt_ledger::pallet as debt_pallet;
    use pallet_templar_debt_ledger::pallet::DebtEvent;
    use sp_std::vec::Vec;

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
    pub trait Config: frame_system::Config + pallet_set_time::Config + debt_pallet::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type NextLoanId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type Loans<T: Config> = StorageMap<_, Blake2_128Concat, u64, LoanRecord<T::AccountId>>;

    #[pallet::storage]
    pub type LoansByLedger<T> =
        StorageMap<_, Blake2_128Concat, u64, BoundedVec<u64, frame_support::traits::ConstU32<100>>>;

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
        LedgerFrozen,
        LedgerBurned,
        LedgerMatured,
        InvalidLoanAmount,
        InsufficientLoanability,
        LoanRecordsFull,
        HistoryFull,
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
            let mut ledger =
                debt_pallet::Ledgers::<T>::get(ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            let now = Self::now();
            ensure!(
                ledger.schedules.loan.allows(now, true),
                Error::<T>::ScheduleNotAllowed
            );
            debt_pallet::Pallet::<T>::validate_loan_target(&ledger).map_err(|_| {
                if ledger.frozen {
                    Error::<T>::LedgerFrozen
                } else if ledger.burned {
                    Error::<T>::LedgerBurned
                } else {
                    Error::<T>::LedgerMatured
                }
            })?;

            let secondary = ledger.loan_count > 0 || ledger.debt_balance > 0;
            let interest = if secondary && !ledger.matured {
                ledger.calculate_interest(amount)
            } else {
                0u128
            };
            let disbursed = amount.saturating_sub(interest);
            ensure!(amount > 0 && disbursed > 0, Error::<T>::InvalidLoanAmount);
            ensure!(
                disbursed <= ledger.deposit_loanability,
                Error::<T>::InsufficientLoanability
            );
            let mint_currency = if secondary {
                ledger.loaned_currency_token
            } else {
                ledger.token
            };

            let loan_id = Self::next_loan_id();
            let interest_rate_bps = ledger.interest_rate_bps;
            let loan_record = debt_pallet::LoanRecord::<T::AccountId> {
                loan_id,
                ledger_id,
                borrower: who.clone(),
                amount: disbursed,
                interest_rate_bps,
                borrowed_at_tick: now,
                loan_count: ledger.loan_count.saturating_add(1),
            };

            ledger.debt_balance = ledger.debt_balance.saturating_add(disbursed);
            ledger.loan_count = ledger.loan_count.saturating_add(1);
            ledger.deposit_loanability = ledger.deposit_loanability.saturating_sub(disbursed);
            ledger.total_minted_from_ledger =
                ledger.total_minted_from_ledger.saturating_add(disbursed);
            ledger.schedules.loan.executed_count =
                ledger.schedules.loan.executed_count.saturating_add(1);
            ledger
                .loans
                .try_push(loan_record)
                .map_err(|_| Error::<T>::LoanRecordsFull)?;

            let matured_by_amount = amount >= ledger.deposit_amount;
            let matured_by_debt = ledger.debt_balance >= ledger.deposit_amount;
            if matured_by_amount || matured_by_debt {
                ledger.matured = true;
                let _ = ledger.debt_history.try_push(DebtEvent::LoanMatured {
                    loan_id,
                    tick: now,
                });
                debt_pallet::Pallet::<T>::deposit_event(debt_pallet::Event::<T>::LoanMatured {
                    ledger: ledger_id,
                    loan_id,
                });
            }

            let _ = ledger.debt_history.try_push(DebtEvent::LoanIssued {
                loan_id,
                borrower: who.clone(),
                requested_amount: amount,
                disbursed_amount: disbursed,
                interest_amount: interest,
                currency_token: mint_currency,
                tick: now,
            });

            debt_pallet::Ledgers::<T>::insert(ledger_id, ledger);
            debt_pallet::Pallet::<T>::deposit_event(debt_pallet::Event::<T>::LoanIssued {
                ledger: ledger_id,
                loan_id,
                borrower: who.clone(),
                requested_amount: amount,
                disbursed_amount: disbursed,
                currency_token: mint_currency,
            });

            Loans::<T>::insert(
                loan_id,
                LoanRecord {
                    id: loan_id,
                    ledger_id,
                    borrower: who,
                    principal: disbursed,
                    maturity_tick: now.saturating_add(100),
                    repaid: 0,
                },
            );
            LoansByLedger::<T>::mutate(ledger_id, |v| {
                let v = v.get_or_insert_with(BoundedVec::default);
                let _ = v.try_push(loan_id);
            });
            Self::deposit_event(Event::LoanCreated {
                id: loan_id,
                ledger: ledger_id,
                amount: disbursed,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn repay(origin: OriginFor<T>, loan_id: u64, amount: u128) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            let mut loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;
            let mut ledger =
                debt_pallet::Ledgers::<T>::get(loan.ledger_id).ok_or(Error::<T>::LedgerNotFound)?;
            let now = Self::now();
            ensure!(
                ledger.schedules.repayment.allows(now, true),
                Error::<T>::ScheduleNotAllowed
            );
            let lid = loan.ledger_id;
            loan.repaid = loan.repaid.saturating_add(amount);
            ledger.debt_balance = ledger.debt_balance.saturating_sub(amount.min(ledger.debt_balance));
            ledger.schedules.repayment.executed_count =
                ledger.schedules.repayment.executed_count.saturating_add(1);
            Loans::<T>::insert(loan_id, loan);
            debt_pallet::Ledgers::<T>::insert(lid, ledger);
            Self::deposit_event(Event::Repaid { loan: loan_id, amount });
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_noop, assert_ok, derive_impl};
    use pallet_templar_debt_ledger::pallet as debt_pallet;
    use sp_io::TestExternalities;
    use sp_runtime::BuildStorage;
    use templar_types::{LedgerSchedules, Schedule};

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            Time: pallet_set_time,
            Pricing: pallet_set_pricing,
            DebtLedger: pallet_templar_debt_ledger,
            Lending: pallet,
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
    impl pallet_templar_debt_ledger::Config for Test {
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

    fn create_ledger(id_owner: u64, token: u64, deposit_amount: u128, interest_bps: u32) -> u64 {
        let s = scheds(0, 0, 0);
        assert_ok!(pallet_templar_debt_ledger::Pallet::<Test>::create_ledger(
            frame_system::RawOrigin::Signed(id_owner).into(),
            token,
            s,
            deposit_amount,
            interest_bps
        ));
        pallet_templar_debt_ledger::NextLedgerId::<Test>::get().saturating_sub(1)
    }

    #[test]
    fn primary_loan_records_full_amount_no_mint() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let lid = create_ledger(1, 10, 100, 500);
            assert_ok!(pallet::Pallet::<Test>::loan(
                frame_system::RawOrigin::Signed(2).into(),
                lid,
                50
            ));
            let ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            assert_eq!(ledger.loan_count, 1);
            assert_eq!(ledger.debt_balance, 50);
            assert_eq!(ledger.deposit_loanability, 20);
            assert_eq!(ledger.loans.len(), 1);
            assert_eq!(ledger.loans[0].amount, 50);
            assert!(ledger.debt_history.iter().any(|e| matches!(
                e,
                pallet_templar_debt_ledger::pallet::DebtEvent::LoanIssued {
                    disbursed_amount: 50,
                    currency_token: 10,
                    ..
                }
            )));
            assert!(pallet_set_pricing::TokenBalances::<Test>::get(10, 2).is_none());
        });
    }

    #[test]
    fn primary_loan_rejected_exceeds_capacity() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let lid = create_ledger(1, 10, 100, 500);
            assert_noop!(
                pallet::Pallet::<Test>::loan(frame_system::RawOrigin::Signed(2).into(), lid, 71),
                Error::<Test>::InsufficientLoanability
            );
        });
    }

    #[test]
    fn secondary_loan_amount_minus_interest_uses_loaned_currency() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let lid = create_ledger(1, 10, 1000, 500);
            assert_ok!(pallet::Pallet::<Test>::loan(
                frame_system::RawOrigin::Signed(2).into(),
                lid,
                100
            ));
            let mut ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            ledger.loaned_currency_token = 99;
            ledger.deposit_loanability = 500;
            pallet_templar_debt_ledger::Ledgers::<Test>::insert(lid, ledger);
            assert_ok!(pallet::Pallet::<Test>::loan(
                frame_system::RawOrigin::Signed(2).into(),
                lid,
                100
            ));
            let ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            assert_eq!(ledger.loan_count, 2);
            assert_eq!(ledger.debt_balance, 195);
            assert!(ledger.loans.last().unwrap().amount == 95);
            assert!(ledger.debt_history.iter().any(|e| matches!(
                e,
                pallet_templar_debt_ledger::pallet::DebtEvent::LoanIssued {
                    disbursed_amount: 95,
                    currency_token: 99,
                    ..
                }
            )));
            assert!(pallet_set_pricing::TokenBalances::<Test>::get(99, 2).is_none());
            assert!(pallet_set_pricing::TokenBalances::<Test>::get(10, 2).is_none());
        });
    }

    #[test]
    fn secondary_loan_rejected_when_disbursed_exceeds_capacity() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let lid = create_ledger(1, 10, 1000, 500);
            let mut ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            ledger.loan_count = 1;
            ledger.debt_balance = 100;
            ledger.deposit_loanability = 50;
            pallet_templar_debt_ledger::Ledgers::<Test>::insert(lid, ledger);
            assert_noop!(
                pallet::Pallet::<Test>::loan(frame_system::RawOrigin::Signed(2).into(), lid, 100),
                Error::<Test>::InsufficientLoanability
            );
        });
    }

    #[test]
    fn loan_entire_deposit_matures_and_blocks_further_loans() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let lid = create_ledger(1, 10, 100, 500);
            let mut ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            ledger.deposit_loanability = 100;
            pallet_templar_debt_ledger::Ledgers::<Test>::insert(lid, ledger);
            assert_ok!(pallet::Pallet::<Test>::loan(
                frame_system::RawOrigin::Signed(2).into(),
                lid,
                100
            ));
            let ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            assert!(ledger.matured);
            assert!(ledger.debt_history.iter().any(|e| matches!(e, pallet_templar_debt_ledger::pallet::DebtEvent::LoanMatured { .. })));
            let mut ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            ledger.deposit_loanability = 10;
            pallet_templar_debt_ledger::Ledgers::<Test>::insert(lid, ledger);
            assert_noop!(
                pallet::Pallet::<Test>::loan(frame_system::RawOrigin::Signed(2).into(), lid, 1),
                Error::<Test>::LedgerMatured
            );
        });
    }

    #[test]
    fn secondary_loan_rejected_when_interest_exceeds_amount() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let lid = create_ledger(1, 10, 1000, 20_000);
            let mut ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            ledger.loan_count = 1;
            ledger.debt_balance = 100;
            ledger.deposit_loanability = 500;
            pallet_templar_debt_ledger::Ledgers::<Test>::insert(lid, ledger);
            assert_noop!(
                pallet::Pallet::<Test>::loan(frame_system::RawOrigin::Signed(2).into(), lid, 100),
                Error::<Test>::InvalidLoanAmount
            );
        });
    }

    #[test]
    fn loan_rejected_missing_frozen_burned() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            assert_noop!(
                pallet::Pallet::<Test>::loan(frame_system::RawOrigin::Signed(2).into(), 999, 10),
                Error::<Test>::LedgerNotFound
            );
            let lid = create_ledger(1, 10, 100, 500);
            pallet_templar_debt_ledger::Ledgers::<Test>::mutate(lid, |l| {
                if let Some(l) = l {
                    l.frozen = true;
                }
            });
            assert_noop!(
                pallet::Pallet::<Test>::loan(frame_system::RawOrigin::Signed(2).into(), lid, 10),
                Error::<Test>::LedgerFrozen
            );
            pallet_templar_debt_ledger::Ledgers::<Test>::mutate(lid, |l| {
                if let Some(l) = l {
                    l.frozen = false;
                    l.burned = true;
                }
            });
            assert_noop!(
                pallet::Pallet::<Test>::loan(frame_system::RawOrigin::Signed(2).into(), lid, 10),
                Error::<Test>::LedgerBurned
            );
        });
    }

    #[test]
    fn transfer_resets_loanability_and_clears_maturity() {
        new_ext().execute_with(|| {
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(
                frame_system::RawOrigin::Root.into(),
                5
            ));
            let lid = create_ledger(1, 10, 100, 500);
            let mut ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            ledger.deposit_loanability = 100;
            pallet_templar_debt_ledger::Ledgers::<Test>::insert(lid, ledger);
            assert_ok!(pallet::Pallet::<Test>::loan(
                frame_system::RawOrigin::Signed(2).into(),
                lid,
                100
            ));
            assert!(pallet_templar_debt_ledger::Ledgers::<Test>::get(lid)
                .unwrap()
                .matured);
            assert_ok!(pallet_templar_debt_ledger::Pallet::<Test>::transfer_ledger(
                frame_system::RawOrigin::Signed(1).into(),
                lid,
                3
            ));
            let ledger = pallet_templar_debt_ledger::Ledgers::<Test>::get(lid).unwrap();
            assert!(!ledger.matured);
            assert_eq!(ledger.owner, 3);
            assert_eq!(ledger.loan_count, 0);
            assert!(ledger.loans.is_empty());
            assert_eq!(ledger.deposit_loanability, 70);
            assert!(ledger
                .debt_history
                .iter()
                .any(|e| matches!(e, pallet_templar_debt_ledger::pallet::DebtEvent::LoanabilityReset { .. })));
        });
    }
}
