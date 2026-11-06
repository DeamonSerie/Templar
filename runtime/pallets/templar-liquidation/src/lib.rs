#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_set_time::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Liquidated { ledger: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        PermissionDenied,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn liquidate(origin: OriginFor<T>, ledger: u64) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::deposit_event(Event::Liquidated { ledger });
            Ok(())
        }
    }
}
