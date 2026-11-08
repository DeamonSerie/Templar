#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::pallet_prelude::MaxEncodedLen;
use scale_info::TypeInfo;

pub type Amount = u128;

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
pub struct LedgerId(pub u64);

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
pub struct AssetId(pub u64);

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
pub struct TxId(pub u64);

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
pub struct Schedule {
    pub start_tick: u128,
    pub interval_ticks: u128,
    pub max_occurrences: u32,
    pub executed_count: u32,
    pub tan_query_id: Option<u64>,
    pub enabled: bool,
}

impl Schedule {
    pub fn new(
        start_tick: u128,
        interval_ticks: u128,
        max_occurrences: u32,
        tan_query_id: Option<u64>,
    ) -> Self {
        Self {
            start_tick,
            interval_ticks,
            max_occurrences,
            executed_count: 0,
            tan_query_id,
            enabled: true,
        }
    }

    pub fn allows(&self, now: u128, tan_result: bool) -> bool {
        if !self.enabled {
            return false;
        }
        if now < self.start_tick {
            return false;
        }
        if self.interval_ticks > 0 {
            let elapsed = now - self.start_tick;
            if elapsed % self.interval_ticks != 0 {
                return false;
            }
        }
        if self.max_occurrences != 0 && self.executed_count >= self.max_occurrences {
            return false;
        }
        if self.tan_query_id.is_some() && !tan_result {
            return false;
        }
        true
    }
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
pub struct LedgerSchedules {
    pub deposit: Schedule,
    pub withdrawal: Schedule,
    pub loan: Schedule,
    pub repayment: Schedule,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
pub enum TemplarError {
    LedgerNotFound,
    InsufficientBalance,
    ScheduleNotAllowed,
    InvalidSchedule,
    NotOwner,
    NotDepositor,
    DebtNotPaid,
    Frozen,
    LiquidationFailed,
    Matured,
    InvalidLoanAmount,
    Burned,
}

pub fn f64_to_bps(v: f64) -> u32 {
    (v * 10000.0) as u32
}

pub fn calculate_simple_interest(principal: u128, interest_rate_bps: u32) -> u128 {
    principal.saturating_mul(interest_rate_bps as u128) / 10_000u128
}
