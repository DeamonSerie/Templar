#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    pub type MaxValidators = frame_support::traits::ConstU32<100>;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct Token<AccountId> {
        pub id: u64,
        pub creator: AccountId,
        pub grid_w: u32,
        pub grid_h: u32,
        pub supply: u128,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum Tx<AccountId> {
        Transfer {
            from: AccountId,
            to: AccountId,
            token: u64,
            amount: u128,
        },
        CreateToken {
            creator: AccountId,
            grid_w: u32,
            grid_h: u32,
            supply: u128,
        },
        Pump {
            wallet: AccountId,
            token: u64,
            amount: u128,
        },
        Dump {
            wallet: AccountId,
            token: u64,
            amount: u128,
        },
        Burn {
            wallet: AccountId,
            token: u64,
            amount: u128,
        },
        ContractCall {
            caller: AccountId,
            contract: BoundedVec<u8, frame_support::traits::ConstU32<256>>,
            calldata: BoundedVec<u8, frame_support::traits::ConstU32<256>>,
        },
        PriceCorrection {
            token: u64,
            index: u64,
            correct_price: u64,
            original_index: u64,
        },
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct TxEntry<AccountId> {
        pub id: u64,
        pub tx: Tx<AccountId>,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct PriceEntry {
        pub token: u64,
        pub location: u64,
        pub price: u64,
        pub tx_backing: u64,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type Tokens<T: Config> = StorageMap<_, Blake2_128Concat, u64, Token<T::AccountId>>;

    #[pallet::storage]
    pub type TokenBalances<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u64, Blake2_128Concat, T::AccountId, u128>;

    #[pallet::storage]
    pub type TxEntries<T: Config> = StorageMap<_, Blake2_128Concat, u64, TxEntry<T::AccountId>>;

    #[pallet::storage]
    pub type NextTxId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type PriceEntries<T: Config> = StorageMap<_, Blake2_128Concat, u64, PriceEntry>;

    #[pallet::storage]
    pub type NextPriceId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type NextTokenId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type Cooldowns<T: Config> = StorageMap<_, Blake2_128Concat, (T::AccountId, u64), u64>;

    #[pallet::storage]
    pub type Validators<T: Config> = StorageValue<_, BoundedVec<T::AccountId, MaxValidators>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TokenCreated {
            token: u64,
            creator: T::AccountId,
        },
        Transferred {
            from: T::AccountId,
            to: T::AccountId,
            token: u64,
            amount: u128,
        },
        Pumped {
            wallet: T::AccountId,
            token: u64,
            amount: u128,
        },
        Dumped {
            wallet: T::AccountId,
            token: u64,
            amount: u128,
        },
        Burned {
            wallet: T::AccountId,
            token: u64,
            amount: u128,
        },
        TxAppended {
            id: u64,
        },
        PriceAppended {
            id: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientBalance,
        CooldownActive,
        PermissionDenied,
        TokenNotFound,
        InvalidGrid,
    }

    const COOLDOWN_TXS: u64 = 20;
    const INTEGRAL_TOKEN_ID: u64 = u64::MAX;
    const LARGE_MOVEMENT_THRESHOLD_PCT: u64 = 10;

    impl<T: Config> Pallet<T> {
        pub fn is_validator(who: &T::AccountId) -> bool {
            Validators::<T>::get().iter().any(|v| v == who)
        }

        fn next_tx_id() -> u64 {
            let id = NextTxId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextTxId::<T>::put(id.saturating_add(1));
            id
        }

        fn next_price_id() -> u64 {
            let id = NextPriceId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextPriceId::<T>::put(id.saturating_add(1));
            id
        }

        fn next_token_id() -> u64 {
            let id = NextTokenId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextTokenId::<T>::put(id.saturating_add(1));
            id
        }

        pub fn append_tx(tx: Tx<T::AccountId>) -> u64 {
            let id = Self::next_tx_id();
            let entry = TxEntry { id, tx };
            TxEntries::<T>::insert(id, entry);
            Self::deposit_event(Event::TxAppended { id });
            id
        }

        pub fn append_price(token: u64, tx_backing: u64) -> u64 {
            let id = Self::next_price_id();
            let price = Self::recompute_price(token, tx_backing);
            let entry = PriceEntry {
                token,
                location: price,
                price,
                tx_backing,
            };
            PriceEntries::<T>::insert(id, entry);
            Self::deposit_event(Event::PriceAppended { id });
            id
        }

        fn cells_for(token: u64) -> u64 {
            Tokens::<T>::get(token)
                .map(|t| t.grid_w as u64 * t.grid_h as u64)
                .unwrap_or(0)
        }

        pub fn recompute_price(token: u64, up_to: u64) -> u64 {
            let cells = Self::cells_for(token);
            if cells == 0 {
                return 0;
            }
            let mut p: i128 = 0;
            for (id, entry) in TxEntries::<T>::iter() {
                if id > up_to {
                    continue;
                }
                match &entry.tx {
                    Tx::Pump { token: t, amount, .. } if *t == token => {
                        if token == INTEGRAL_TOKEN_ID {
                            p -= *amount as i128;
                        } else {
                            p += *amount as i128;
                        }
                    }
                    Tx::Dump { token: t, amount, .. } if *t == token => {
                        if token == INTEGRAL_TOKEN_ID {
                            p += *amount as i128;
                        } else {
                            p -= *amount as i128;
                        }
                    }
                    Tx::Transfer { token: t, amount, .. } if *t == token => {
                        p += *amount as i128;
                    }
                    Tx::Burn { token: t, amount, .. } if *t == token => {
                        p += *amount as i128;
                    }
                    _ => {}
                }
            }
            let cells_i = cells as i128;
            let v = ((p % cells_i) + cells_i) % cells_i;
            v as u64
        }

        pub fn apply_fee_burn(tx: &Tx<T::AccountId>, _tx_id: u64) {
            let (payer, fee) = match tx {
                Tx::Transfer { from, amount, .. } => (from.clone(), *amount / 100),
                Tx::Pump { wallet, amount, .. } => (wallet.clone(), *amount / 100),
                Tx::Dump { wallet, amount, .. } => (wallet.clone(), *amount / 100),
                _ => return,
            };
            if fee == 0 {
                return;
            }
            TokenBalances::<T>::mutate(INTEGRAL_TOKEN_ID, &payer, |b| {
                let cur = b.unwrap_or(0);
                *b = Some(cur.saturating_sub(fee));
            });
            let burn_tx = Tx::Burn {
                wallet: payer.clone(),
                token: INTEGRAL_TOKEN_ID,
                amount: fee,
            };
            let bid = Self::append_tx(burn_tx);
            let _ = bid;
            Self::deposit_event(Event::Burned {
                wallet: payer,
                token: INTEGRAL_TOKEN_ID,
                amount: fee,
            });
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn create_token(
            origin: OriginFor<T>,
            grid_w: u32,
            grid_h: u32,
            supply: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            if grid_w == 0 || grid_h == 0 {
                return Err(Error::<T>::InvalidGrid.into());
            }
            let id = Self::next_token_id();
            let token = Token {
                id,
                creator: who.clone(),
                grid_w,
                grid_h,
                supply,
            };
            Tokens::<T>::insert(id, token);
            TokenBalances::<T>::insert(id, &who, supply);
            let tx = Tx::CreateToken {
                creator: who.clone(),
                grid_w,
                grid_h,
                supply,
            };
            Self::append_tx(tx);
            Self::deposit_event(Event::TokenCreated {
                token: id,
                creator: who,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            token: u64,
            amount: u128,
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;
            let from_bal = TokenBalances::<T>::get(token, &from).unwrap_or(0);
            if from_bal < amount {
                return Err(Error::<T>::InsufficientBalance.into());
            }
            TokenBalances::<T>::mutate(token, &from, |b| {
                *b = Some(b.unwrap_or(0).saturating_sub(amount));
            });
            TokenBalances::<T>::mutate(token, &to, |b| {
                *b = Some(b.unwrap_or(0).saturating_add(amount));
            });
            let tx = Tx::Transfer {
                from: from.clone(),
                to: to.clone(),
                token,
                amount,
            };
            let tx_id = Self::append_tx(tx.clone());
            Self::append_price(token, tx_id);
            Self::apply_fee_burn(&tx, tx_id);
            Self::deposit_event(Event::Transferred {
                from,
                to,
                token,
                amount,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn pump(origin: OriginFor<T>, token: u64, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            if token == INTEGRAL_TOKEN_ID && !Self::is_validator(&who) {
                return Err(Error::<T>::PermissionDenied.into());
            }
            let now_id = NextTxId::<T>::get();
            let now_id_cur = if now_id == 0 { 1 } else { now_id };
            let price_before = Self::recompute_price(token, if now_id_cur == 0 { 0 } else { now_id_cur.saturating_sub(1) });
            // simulate
            // compute hypothetical price by applying effect mentally - easier: do it by temp logic via recompute on hypothetical? just add to balance temporarily conceptually not needed
            // compute move by: we need to estimate; alternatively do balance add, append tx to get price_after, check, if bad revert effects not trivial - better reorder
            // simpler: check cooldown only if will be large - estimate: add amount effect
            let cells = Self::cells_for(token);
            let mut p_est: i128 = 0;
            for (id, entry) in TxEntries::<T>::iter() {
                if id >= now_id_cur { continue; }
                match &entry.tx {
                    Tx::Pump { token: t, amount: a, .. } if *t == token => {
                        if token == INTEGRAL_TOKEN_ID { p_est -= *a as i128; } else { p_est += *a as i128; }
                    }
                    Tx::Dump { token: t, amount: a, .. } if *t == token => {
                        if token == INTEGRAL_TOKEN_ID { p_est += *a as i128; } else { p_est -= *a as i128; }
                    }
                    Tx::Transfer { token: t, amount: a, .. } if *t == token => { p_est += *a as i128; }
                    Tx::Burn { token: t, amount: a, .. } if *t == token => { p_est += *a as i128; }
                    _ => {}
                }
            }
            if token != INTEGRAL_TOKEN_ID { p_est += amount as i128; } else { p_est -= amount as i128; }
            let cells_i = if cells == 0 { 1i128 } else { cells as i128 };
            let price_after_est = (((p_est % cells_i) + cells_i) % cells_i) as u64;
            let move_pct_est = if price_before == 0 { 0u64 } else { price_before.abs_diff(price_after_est) * 100 / price_before };
            if move_pct_est >= LARGE_MOVEMENT_THRESHOLD_PCT {
                if let Some(prev) = Cooldowns::<T>::get((&who, token)) {
                    if now_id_cur.saturating_sub(prev) < COOLDOWN_TXS {
                        return Err(Error::<T>::CooldownActive.into());
                    }
                }
            }
            TokenBalances::<T>::mutate(token, &who, |b| {
                *b = Some(b.unwrap_or(0).saturating_add(amount));
            });
            let tx = Tx::Pump {
                wallet: who.clone(),
                token,
                amount,
            };
            let tx_id = Self::append_tx(tx.clone());
            if move_pct_est >= LARGE_MOVEMENT_THRESHOLD_PCT {
                Cooldowns::<T>::insert((&who, token), tx_id);
            }
            Self::append_price(token, tx_id);
            Self::apply_fee_burn(&tx, tx_id);
            Self::deposit_event(Event::Pumped {
                wallet: who,
                token,
                amount,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn dump(origin: OriginFor<T>, token: u64, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let now_id = NextTxId::<T>::get();
            let now_id = if now_id == 0 { 1 } else { now_id };
            let bal = TokenBalances::<T>::get(token, &who).unwrap_or(0);
            if bal < amount {
                return Err(Error::<T>::InsufficientBalance.into());
            }
            let price_before = Self::recompute_price(token, if now_id == 0 { 0 } else { now_id.saturating_sub(1) });
            // estimate after
            let cells = Self::cells_for(token);
            let mut p_est: i128 = 0;
            for (id, entry) in TxEntries::<T>::iter() {
                if id >= now_id { continue; }
                match &entry.tx {
                    Tx::Pump { token: t, amount: a, .. } if *t == token => {
                        if token == INTEGRAL_TOKEN_ID { p_est -= *a as i128; } else { p_est += *a as i128; }
                    }
                    Tx::Dump { token: t, amount: a, .. } if *t == token => {
                        if token == INTEGRAL_TOKEN_ID { p_est += *a as i128; } else { p_est -= *a as i128; }
                    }
                    Tx::Transfer { token: t, amount: a, .. } if *t == token => { p_est += *a as i128; }
                    Tx::Burn { token: t, amount: a, .. } if *t == token => { p_est += *a as i128; }
                    _ => {}
                }
            }
            if token != INTEGRAL_TOKEN_ID { p_est -= amount as i128; } else { p_est += amount as i128; }
            let cells_i = if cells == 0 { 1i128 } else { cells as i128 };
            let price_after_est = (((p_est % cells_i) + cells_i) % cells_i) as u64;
            let move_pct_est = if price_before == 0 { 0u64 } else { price_before.abs_diff(price_after_est) * 100 / price_before };
            if move_pct_est >= LARGE_MOVEMENT_THRESHOLD_PCT {
                if let Some(prev) = Cooldowns::<T>::get((&who, token)) {
                    if now_id.saturating_sub(prev) < COOLDOWN_TXS {
                        return Err(Error::<T>::CooldownActive.into());
                    }
                }
            }
            TokenBalances::<T>::mutate(token, &who, |b| {
                *b = Some(b.unwrap_or(0).saturating_sub(amount));
            });
            let tx = Tx::Dump {
                wallet: who.clone(),
                token,
                amount,
            };
            let tx_id = Self::append_tx(tx.clone());
            let price_after = Self::recompute_price(token, tx_id);
            let move_pct = if price_before == 0 {
                0
            } else {
                price_before.abs_diff(price_after) * 100 / price_before
            };
            if move_pct >= 10 {
                let remaining = TokenBalances::<T>::get(token, &who).unwrap_or(0);
                let half = remaining / 2;
                if half > 0 {
                    TokenBalances::<T>::mutate(token, &who, |b| {
                        *b = Some(b.unwrap_or(0).saturating_sub(half));
                    });
                    let proceeds = half.saturating_mul(price_after as u128);
                    TokenBalances::<T>::mutate(0u64, &who, |b| {
                        *b = Some(b.unwrap_or(0).saturating_add(proceeds));
                    });
                }
            }
            if move_pct_est >= LARGE_MOVEMENT_THRESHOLD_PCT {
                Cooldowns::<T>::insert((&who, token), tx_id);
            }
            Self::append_price(token, tx_id);
            Self::apply_fee_burn(&tx, tx_id);
            Self::deposit_event(Event::Dumped {
                wallet: who,
                token,
                amount,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn burn(origin: OriginFor<T>, token: u64, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let bal = TokenBalances::<T>::get(token, &who).unwrap_or(0);
            if bal < amount {
                return Err(Error::<T>::InsufficientBalance.into());
            }
            TokenBalances::<T>::mutate(token, &who, |b| {
                *b = Some(b.unwrap_or(0).saturating_sub(amount));
            });
            let tx = Tx::Burn {
                wallet: who.clone(),
                token,
                amount,
            };
            let tx_id = Self::append_tx(tx);
            Self::append_price(token, tx_id);
            Self::deposit_event(Event::Burned {
                wallet: who,
                token,
                amount,
            });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn price_correction(
            origin: OriginFor<T>,
            token: u64,
            index: u64,
            correct_price: u64,
            original_index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            if token == INTEGRAL_TOKEN_ID && !Self::is_validator(&who) {
                return Err(Error::<T>::PermissionDenied.into());
            }
            let tx = Tx::PriceCorrection {
                token,
                index: index,
                correct_price,
                original_index,
            };
            let tx_id = Self::append_tx(tx);
            let pid = Self::next_price_id();
            let entry = PriceEntry {
                token,
                location: correct_price,
                price: correct_price,
                tx_backing: tx_id,
            };
            PriceEntries::<T>::insert(pid, entry);
            Self::deposit_event(Event::PriceAppended { id: pid });
            Ok(())
        }
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub validators: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            Validators::<T>::put(BoundedVec::<T::AccountId, MaxValidators>::try_from(self.validators.clone()).unwrap_or_default());
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::pallet as pallet_pricing;
    use crate as pallet_set_pricing;

    use frame_support::{assert_ok, derive_impl, parameter_types, construct_runtime};
    use sp_io::TestExternalities;
    use sp_runtime::BuildStorage;

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            Pricing: pallet,
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
        frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
    }

    #[test]
    fn create_and_transfer() {
        new_test_ext().execute_with(|| {
            assert_ok!(pallet::Pallet::<Test>::create_token(frame_system::RawOrigin::Signed(1).into(), 2, 2, 100));
            assert_ok!(pallet::Pallet::<Test>::transfer(frame_system::RawOrigin::Signed(1).into(), 2, 1, 10));
            assert!(TxEntries::<Test>::contains_key(1));
            assert!(TxEntries::<Test>::contains_key(2));
        });
    }

    #[test]
    fn append_only_price() {
        new_test_ext().execute_with(|| {
            assert_ok!(pallet::Pallet::<Test>::create_token(frame_system::RawOrigin::Signed(1).into(), 2, 2, 100));
            let before = PriceEntries::<Test>::iter().count();
            assert_ok!(pallet::Pallet::<Test>::pump(frame_system::RawOrigin::Signed(1).into(), 1, 5));
            let after = PriceEntries::<Test>::iter().count();
            assert!(after > before);
        });
    }
}
