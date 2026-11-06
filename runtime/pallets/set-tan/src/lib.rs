#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    pub type MaxName = frame_support::traits::ConstU32<64>;
    pub type MaxQueryLen = frame_support::traits::ConstU32<256>;
    pub type MaxResult = frame_support::traits::ConstU32<256>;
    pub type MaxParams = frame_support::traits::ConstU32<16>;

    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum TargetKind {
        Token,
        Account,
        Price,
        Transaction,
        AsyncProcess,
        Generic,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct TargetMeta {
        pub kind: TargetKind,
        pub is_tran_delta: bool,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum QueryStatus {
        Pending,
        Running,
        Completed,
        Failed,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum AsyncTaskStatus {
        Pending,
        Running,
        Completed,
        Failed,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum ConditionOp {
        Eq,
        Ne,
        Lt,
        Lte,
        Gt,
        Gte,
    }

    #[derive(Clone, Copy, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum MetricKind {
        Price,
        Balance,
        Time,
    }
    pub type Metric = MetricKind;
    pub type MetricType = MetricKind;

    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum U64Ref {
        Lit(u64),
        Var(BoundedVec<u8, MaxName>),
    }

    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum U128Ref {
        Lit(u128),
        Var(BoundedVec<u8, MaxName>),
    }

    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum NameRef {
        Lit(BoundedVec<u8, MaxName>),
        Var(BoundedVec<u8, MaxName>),
    }

    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum VariableValue {
        U64(u64),
        U128(u128),
        Name(BoundedVec<u8, MaxName>),
        Bytes(BoundedVec<u8, MaxResult>),
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct Condition {
        pub metric_kind: MetricKind,
        pub token: U64Ref,
        pub op: ConditionOp,
        pub rhs: U64Ref,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub enum Action {
        None,
        Buy { token: U64Ref, amount: U128Ref },
        Sell { token: U64Ref, amount: U128Ref },
        Task { name: NameRef },
        Deposit { ledger: U64Ref, amount: U128Ref },
        Withdraw { ledger: U64Ref, amount: U128Ref },
        Loan { ledger: U64Ref, amount: U128Ref },
        Repay { loan: U64Ref, amount: U128Ref },
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ParsedQuery {
        pub tran_delta: Vec<u8>,
        pub dependent_task: Vec<u8>,
        pub async_task: Vec<u8>,
        pub condition: Option<Condition>,
        pub on_true: Action,
        pub on_false: Action,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct QueryRecord<AccountId> {
        pub submitter: AccountId,
        pub raw: BoundedVec<u8, MaxQueryLen>,
        pub tran_delta: BoundedVec<u8, MaxName>,
        pub dependent_task: BoundedVec<u8, MaxName>,
        pub async_task: BoundedVec<u8, MaxName>,
        pub status: QueryStatus,
        pub result: Option<BoundedVec<u8, MaxResult>>,
        pub condition: Option<Condition>,
        pub on_true: Action,
        pub on_false: Action,
        pub params: BoundedVec<(BoundedVec<u8, MaxName>, VariableValue), MaxParams>,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    pub struct AsyncTaskRecord<AccountId> {
        pub owner: AccountId,
        pub query_id: u64,
        pub name: BoundedVec<u8, MaxName>,
        pub payload: BoundedVec<u8, MaxResult>,
        pub status: AsyncTaskStatus,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_set_pricing::Config + pallet_set_time::Config + pallet_templar_debt_ledger::Config + pallet_templar_lending::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type Targets<T: Config> =
        StorageMap<_, Blake2_128Concat, BoundedVec<u8, MaxName>, TargetMeta>;

    #[pallet::storage]
    pub type TranDeltas<T: Config> =
        StorageMap<_, Blake2_128Concat, BoundedVec<u8, MaxName>, ()>;

    #[pallet::storage]
    pub type Queries<T: Config> = StorageMap<_, Blake2_128Concat, u64, QueryRecord<T::AccountId>>;

    #[pallet::storage]
    pub type AsyncTasks<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, AsyncTaskRecord<T::AccountId>>;

    #[pallet::storage]
    pub type Variables<T: Config> =
        StorageMap<_, Blake2_128Concat, BoundedVec<u8, MaxName>, VariableValue>;

    #[pallet::storage]
    pub type NextQueryId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type NextTaskId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TargetRegistered {
            name: BoundedVec<u8, MaxName>,
        },
        TranDeltaRegistered {
            name: BoundedVec<u8, MaxName>,
        },
        QuerySubmitted {
            id: u64,
            tran_delta: BoundedVec<u8, MaxName>,
        },
        QueryCompleted {
            id: u64,
        },
        ConditionEvaluated {
            id: u64,
            result: bool,
        },
        ActionExecuted {
            id: u64,
            action: BoundedVec<u8, MaxName>,
        },
        AsyncScheduled {
            query: u64,
            task: u64,
        },
        AsyncCompleted {
            task: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidName,
        InvalidQuery,
        ParseError,
        TranDeltaNotFound,
        QueryNotFound,
        TaskNotFound,
        PermissionDenied,
        InvalidCondition,
        InvalidAction,
        MetricNotFound,
        ExecutionFailed,
        VariableNotFound,
        TypeMismatch,
        InvalidVariable,
        TooManyParams,
    }

    impl<T: Config> Pallet<T> {
        fn next_query_id() -> u64 {
            let id = NextQueryId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextQueryId::<T>::put(id.saturating_add(1));
            id
        }
        fn next_task_id() -> u64 {
            let id = NextTaskId::<T>::get();
            let id = if id == 0 { 1 } else { id };
            NextTaskId::<T>::put(id.saturating_add(1));
            id
        }

        fn to_bounded_name(v: Vec<u8>) -> Result<BoundedVec<u8, MaxName>, Error<T>> {
            BoundedVec::<u8, MaxName>::try_from(v).map_err(|_| Error::<T>::InvalidName)
        }

        fn to_bounded_query(v: Vec<u8>) -> Result<BoundedVec<u8, MaxQueryLen>, Error<T>> {
            BoundedVec::<u8, MaxQueryLen>::try_from(v).map_err(|_| Error::<T>::InvalidQuery)
        }

        fn to_bounded_result(v: Vec<u8>) -> Result<BoundedVec<u8, MaxResult>, Error<T>> {
            BoundedVec::<u8, MaxResult>::try_from(v).map_err(|_| Error::<T>::InvalidQuery)
        }

        pub fn parse_raw(raw: &[u8]) -> Result<ParsedQuery, Error<T>> {
            let s = core::str::from_utf8(raw).map_err(|_| Error::<T>::ParseError)?;
            let s = s.trim();
            if !s.starts_with("@(") {
                return Err(Error::<T>::ParseError);
            }
            let open = s.find('(').ok_or(Error::<T>::ParseError)?;
            let close = find_matching(s, open).ok_or(Error::<T>::ParseError)?;
            let delta = s[open + 1..close].trim();
            let mut rest = s[close + 1..].trim();
            if !rest.starts_with("::") {
                return Err(Error::<T>::ParseError);
            }
            rest = rest[2..].trim();
            if !rest.starts_with("->") {
                return Err(Error::<T>::ParseError);
            }
            rest = rest[2..].trim();
            if !rest.starts_with('(') {
                return Err(Error::<T>::ParseError);
            }
            let close2 = find_matching(rest, 0).ok_or(Error::<T>::ParseError)?;
            let dep = rest[1..close2].trim();
            rest = rest[close2 + 1..].trim();
            if !rest.starts_with("<-->") {
                return Err(Error::<T>::ParseError);
            }
            rest = rest[4..].trim();
            if !rest.starts_with('(') {
                return Err(Error::<T>::ParseError);
            }
            let close3 = find_matching(rest, 0).ok_or(Error::<T>::ParseError)?;
            let async_task = rest[1..close3].trim();
            let mut trailing = rest[close3 + 1..].trim();
            // allow optional trailing interval brackets
            if trailing.starts_with('[') {
                if let Some(idx) = trailing.find(']') {
                    trailing = trailing[idx + 1..].trim();
                } else {
                    return Err(Error::<T>::ParseError);
                }
            }
            let mut condition: Option<Condition> = None;
            let mut on_true = Action::None;
            let mut on_false = Action::None;
            if !trailing.is_empty() {
                if !trailing.starts_with("if") {
                    return Err(Error::<T>::ParseError);
                }
                let mut t = trailing[2..].trim();
                if !t.starts_with('(') {
                    return Err(Error::<T>::ParseError);
                }
                let cclose = find_matching(t, 0).ok_or(Error::<T>::ParseError)?;
                let cond_str = t[1..cclose].trim();
                t = t[cclose + 1..].trim();
                condition = Some(Self::parse_condition(cond_str)?);
                if !t.starts_with('{') {
                    return Err(Error::<T>::ParseError);
                }
                let bclose = find_matching_brace(t, 0).ok_or(Error::<T>::ParseError)?;
                let action_str = t[1..bclose].trim();
                on_true = Self::parse_action(action_str)?;
                t = t[bclose + 1..].trim();
                if t.starts_with("else") {
                    t = t[4..].trim();
                    if !t.starts_with('{') {
                        return Err(Error::<T>::ParseError);
                    }
                    let bclose2 = find_matching_brace(t, 0).ok_or(Error::<T>::ParseError)?;
                    let action_str2 = t[1..bclose2].trim();
                    on_false = Self::parse_action(action_str2)?;
                    t = t[bclose2 + 1..].trim();
                }
                if !t.is_empty() {
                    return Err(Error::<T>::ParseError);
                }
            }
            Ok(ParsedQuery {
                tran_delta: delta.as_bytes().to_vec(),
                dependent_task: dep.as_bytes().to_vec(),
                async_task: async_task.as_bytes().to_vec(),
                condition,
                on_true,
                on_false,
            })
        }

        fn parse_metric(s: &str) -> Result<(MetricKind, U64Ref), Error<T>> {
            let s = s.trim();
            if let Some(rest) = s.strip_prefix("time") {
                let rest = rest.trim();
                if rest.is_empty() {
                    return Ok((MetricKind::Time, U64Ref::Lit(0)));
                }
                if !rest.starts_with('(') {
                    return Err(Error::<T>::InvalidCondition);
                }
                let close = find_matching(rest, 0).ok_or(Error::<T>::InvalidCondition)?;
                if !rest[close + 1..].trim().is_empty() {
                    return Err(Error::<T>::InvalidCondition);
                }
                let inner = rest[1..close].trim();
                if inner.is_empty() {
                    return Ok((MetricKind::Time, U64Ref::Lit(0)));
                }
                let token = Self::parse_u64_ref(inner, Error::<T>::InvalidCondition)?;
                return Ok((MetricKind::Time, token));
            }
            for (prefix, kind) in [("price", MetricKind::Price), ("balance", MetricKind::Balance)] {
                if let Some(rest) = s.strip_prefix(prefix) {
                    let rest = rest.trim();
                    if !rest.starts_with('(') {
                        return Err(Error::<T>::InvalidCondition);
                    }
                    let close = find_matching(rest, 0).ok_or(Error::<T>::InvalidCondition)?;
                    let token_str = rest[1..close].trim();
                    let token = Self::parse_u64_ref(token_str, Error::<T>::InvalidCondition)?;
                    if !rest[close + 1..].trim().is_empty() {
                        return Err(Error::<T>::InvalidCondition);
                    }
                    return Ok((kind, token));
                }
            }
            Err(Error::<T>::InvalidCondition)
        }

        fn parse_u64_ref(s: &str, err: Error<T>) -> Result<U64Ref, Error<T>> {
            let s = s.trim();
            if let Some(name) = s.strip_prefix('$') {
                let b = Self::to_bounded_name(name.trim().as_bytes().to_vec())
                    .map_err(|_| Error::<T>::InvalidVariable)?;
                Ok(U64Ref::Var(b))
            } else {
                let v: u64 = s.parse().map_err(|_| err)?;
                Ok(U64Ref::Lit(v))
            }
        }

        fn parse_u128_ref(s: &str, err: Error<T>) -> Result<U128Ref, Error<T>> {
            let s = s.trim();
            if let Some(name) = s.strip_prefix('$') {
                let b = Self::to_bounded_name(name.trim().as_bytes().to_vec())
                    .map_err(|_| Error::<T>::InvalidVariable)?;
                Ok(U128Ref::Var(b))
            } else {
                let v: u128 = s.parse().map_err(|_| err)?;
                Ok(U128Ref::Lit(v))
            }
        }

        fn parse_name_ref(s: &str, err: Error<T>) -> Result<NameRef, Error<T>> {
            let s = s.trim();
            if let Some(name) = s.strip_prefix('$') {
                let b = Self::to_bounded_name(name.trim().as_bytes().to_vec())
                    .map_err(|_| Error::<T>::InvalidVariable)?;
                Ok(NameRef::Var(b))
            } else {
                let b = Self::to_bounded_name(s.as_bytes().to_vec()).map_err(|_| err)?;
                Ok(NameRef::Lit(b))
            }
        }

        fn parse_condition(s: &str) -> Result<Condition, Error<T>> {
            let s = s.trim();
            let mut op_pos: Option<usize> = None;
            let mut op_str = "";
            let mut i = 0;
            while i < s.len() {
                if i + 2 <= s.len() {
                    let two = &s[i..i + 2];
                    if two == "==" || two == "!=" || two == "<=" || two == ">=" {
                        op_pos = Some(i);
                        op_str = two;
                        break;
                    }
                }
                if i + 1 <= s.len() {
                    let one = &s[i..i + 1];
                    if one == "<" || one == ">" {
                        op_pos = Some(i);
                        op_str = one;
                        break;
                    }
                }
                i += 1;
            }
            let pos = op_pos.ok_or(Error::<T>::InvalidCondition)?;
            let lhs = s[..pos].trim();
            let rhs_str = s[pos + op_str.len()..].trim();
            if lhs.is_empty() || rhs_str.is_empty() {
                return Err(Error::<T>::InvalidCondition);
            }
            let op = match op_str {
                "==" => ConditionOp::Eq,
                "!=" => ConditionOp::Ne,
                "<" => ConditionOp::Lt,
                "<=" => ConditionOp::Lte,
                ">" => ConditionOp::Gt,
                ">=" => ConditionOp::Gte,
                _ => return Err(Error::<T>::InvalidCondition),
            };
            let (metric_kind, token) = Self::parse_metric(lhs)?;
            let rhs = Self::parse_u64_ref(rhs_str, Error::<T>::InvalidCondition)?;
            Ok(Condition {
                metric_kind,
                token,
                op,
                rhs,
            })
        }

        fn parse_action(s: &str) -> Result<Action, Error<T>> {
            let s = s.trim();
            if s.is_empty() {
                return Err(Error::<T>::InvalidAction);
            }
            let lower = s.to_lowercase();
            if lower == "none" {
                return Ok(Action::None);
            }
            for (prefix, is_buy) in [("buy", true), ("sell", false)] {
                if lower.starts_with(prefix) {
                    let rest = s[prefix.len()..].trim();
                    if !rest.starts_with('(') {
                        return Err(Error::<T>::InvalidAction);
                    }
                    let close = find_matching(rest, 0).ok_or(Error::<T>::InvalidAction)?;
                    if !rest[close + 1..].trim().is_empty() {
                        return Err(Error::<T>::InvalidAction);
                    }
                    let inner = rest[1..close].trim();
                    let comma = inner.find(',').ok_or(Error::<T>::InvalidAction)?;
                    let token = Self::parse_u64_ref(inner[..comma].trim(), Error::<T>::InvalidAction)?;
                    let amount = Self::parse_u128_ref(inner[comma + 1..].trim(), Error::<T>::InvalidAction)?;
                    return if is_buy {
                        Ok(Action::Buy { token, amount })
                    } else {
                        Ok(Action::Sell { token, amount })
                    };
                }
            }
            for (prefix, is_deposit) in [("deposit", true)] {
                let _ = (prefix, is_deposit);
            }
            // templar actions: deposit/withdraw/loan/repay(ledger, amount)
            for prefix in ["deposit", "withdraw", "loan", "repay"] {
                let lower_prefix = prefix;
                if lower.starts_with(lower_prefix) {
                    let rest = s[prefix.len()..].trim();
                    if !rest.starts_with('(') {
                        return Err(Error::<T>::InvalidAction);
                    }
                    let close = find_matching(rest, 0).ok_or(Error::<T>::InvalidAction)?;
                    if !rest[close + 1..].trim().is_empty() {
                        return Err(Error::<T>::InvalidAction);
                    }
                    let inner = rest[1..close].trim();
                    let comma = inner.find(',').ok_or(Error::<T>::InvalidAction)?;
                    let ledger = Self::parse_u64_ref(inner[..comma].trim(), Error::<T>::InvalidAction)?;
                    let amount = Self::parse_u128_ref(inner[comma + 1..].trim(), Error::<T>::InvalidAction)?;
                    return match prefix {
                        "deposit" => Ok(Action::Deposit { ledger, amount }),
                        "withdraw" => Ok(Action::Withdraw { ledger, amount }),
                        "loan" => Ok(Action::Loan { ledger, amount }),
                        "repay" => Ok(Action::Repay { loan: ledger, amount }),
                        _ => Err(Error::<T>::InvalidAction),
                    };
                }
            }
            let name_expr = if s.starts_with('(') {
                let close = find_matching(s, 0).ok_or(Error::<T>::InvalidAction)?;
                if !s[close + 1..].trim().is_empty() {
                    return Err(Error::<T>::InvalidAction);
                }
                s[1..close].trim()
            } else {
                s
            };
            if name_expr.is_empty() {
                return Err(Error::<T>::InvalidAction);
            }
            let name = Self::parse_name_ref(name_expr, Error::<T>::InvalidAction)?;
            Ok(Action::Task { name })
        }

        fn resolve_u64(ctx: &[(BoundedVec<u8, MaxName>, VariableValue)], name: &BoundedVec<u8, MaxName>) -> Result<u64, Error<T>> {
            for (n, v) in ctx.iter().rev() {
                if n == name {
                    return match v {
                        VariableValue::U64(x) => Ok(*x),
                        VariableValue::U128(x) if *x <= u64::MAX as u128 => Ok(*x as u64),
                        VariableValue::Bytes(b) => {
                            let s = core::str::from_utf8(b).map_err(|_| Error::<T>::TypeMismatch)?;
                            s.trim().parse().map_err(|_| Error::<T>::TypeMismatch)
                        }
                        _ => Err(Error::<T>::TypeMismatch),
                    };
                }
            }
            Err(Error::<T>::VariableNotFound)
        }

        fn resolve_u128(ctx: &[(BoundedVec<u8, MaxName>, VariableValue)], name: &BoundedVec<u8, MaxName>) -> Result<u128, Error<T>> {
            for (n, v) in ctx.iter().rev() {
                if n == name {
                    return match v {
                        VariableValue::U128(x) => Ok(*x),
                        VariableValue::U64(x) => Ok(*x as u128),
                        VariableValue::Bytes(b) => {
                            let s = core::str::from_utf8(b).map_err(|_| Error::<T>::TypeMismatch)?;
                            s.trim().parse().map_err(|_| Error::<T>::TypeMismatch)
                        }
                        _ => Err(Error::<T>::TypeMismatch),
                    };
                }
            }
            Err(Error::<T>::VariableNotFound)
        }

        fn resolve_name(ctx: &[(BoundedVec<u8, MaxName>, VariableValue)], name: &BoundedVec<u8, MaxName>) -> Result<BoundedVec<u8, MaxName>, Error<T>> {
            for (n, v) in ctx.iter().rev() {
                if n == name {
                    return match v {
                        VariableValue::Name(x) => Ok(x.clone()),
                        VariableValue::Bytes(b) => {
                            BoundedVec::<u8, MaxName>::try_from(b.to_vec())
                                .map_err(|_| Error::<T>::TypeMismatch)
                        }
                        _ => Err(Error::<T>::TypeMismatch),
                    };
                }
            }
            Err(Error::<T>::VariableNotFound)
        }

        fn resolve_u64_ref(ctx: &[(BoundedVec<u8, MaxName>, VariableValue)], r: &U64Ref) -> Result<u64, Error<T>> {
            match r {
                U64Ref::Lit(v) => Ok(*v),
                U64Ref::Var(name) => Self::resolve_u64(ctx, name),
            }
        }

        fn resolve_u128_ref(ctx: &[(BoundedVec<u8, MaxName>, VariableValue)], r: &U128Ref) -> Result<u128, Error<T>> {
            match r {
                U128Ref::Lit(v) => Ok(*v),
                U128Ref::Var(name) => Self::resolve_u128(ctx, name),
            }
        }

        fn resolve_name_ref(ctx: &[(BoundedVec<u8, MaxName>, VariableValue)], r: &NameRef) -> Result<BoundedVec<u8, MaxName>, Error<T>> {
            match r {
                NameRef::Lit(v) => Ok(v.clone()),
                NameRef::Var(name) => Self::resolve_name(ctx, name),
            }
        }
    }

    fn find_matching(s: &str, open_idx: usize) -> Option<usize> {
        let bytes = s.as_bytes();
        if bytes.get(open_idx) != Some(&b'(') {
            return None;
        }
        let mut depth = 0i32;
        let mut i = open_idx;
        while i < s.len() {
            let c = bytes[i];
            if c == b'(' {
                depth += 1;
            } else if c == b')' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    fn find_matching_brace(s: &str, open_idx: usize) -> Option<usize> {
        let bytes = s.as_bytes();
        if bytes.get(open_idx) != Some(&b'{') {
            return None;
        }
        let mut depth = 0i32;
        let mut i = open_idx;
        while i < s.len() {
            let c = bytes[i];
            if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn register_target(
            origin: OriginFor<T>,
            name: Vec<u8>,
            kind: TargetKind,
            is_tran_delta: bool,
        ) -> DispatchResult {
            if ensure_signed(origin.clone()).is_err() {
                ensure_root(origin.clone())?;
            }
            let bname = Self::to_bounded_name(name)?;
            let meta = TargetMeta { kind, is_tran_delta };
            Targets::<T>::insert(bname.clone(), meta);
            if is_tran_delta {
                TranDeltas::<T>::insert(bname.clone(), ());
            }
            Self::deposit_event(Event::TargetRegistered { name: bname });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn register_tran_delta(origin: OriginFor<T>, name: Vec<u8>) -> DispatchResult {
            if ensure_signed(origin.clone()).is_err() {
                ensure_root(origin.clone())?;
            }
            let bname = Self::to_bounded_name(name)?;
            if !Targets::<T>::contains_key(&bname) {
                Targets::<T>::insert(bname.clone(), TargetMeta {
                    kind: TargetKind::Generic,
                    is_tran_delta: true,
                });
                Self::deposit_event(Event::TargetRegistered { name: bname.clone() });
            } else {
                Targets::<T>::mutate(&bname, |m| {
                    if let Some(meta) = m {
                        meta.is_tran_delta = true;
                    }
                });
            }
            TranDeltas::<T>::insert(bname.clone(), ());
            Self::deposit_event(Event::TranDeltaRegistered { name: bname });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn submit_query(origin: OriginFor<T>, raw: Vec<u8>, params: Vec<(Vec<u8>, VariableValue)>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let braw = Self::to_bounded_query(raw.clone())?;
            let parsed = Self::parse_raw(&raw)?;
            let bdelta = Self::to_bounded_name(parsed.tran_delta)?;
            let bdep = Self::to_bounded_name(parsed.dependent_task)?;
            let basync = Self::to_bounded_name(parsed.async_task)?;
            if !TranDeltas::<T>::contains_key(&bdelta)
                && !Targets::<T>::get(&bdelta).map(|m| m.is_tran_delta).unwrap_or(false)
            {
                return Err(Error::<T>::TranDeltaNotFound.into());
            }
            if params.len() > <MaxParams as frame_support::traits::Get<u32>>::get() as usize {
                return Err(Error::<T>::TooManyParams.into());
            }
            let mut bparams: BoundedVec<(BoundedVec<u8, MaxName>, VariableValue), MaxParams> = BoundedVec::default();
            for (name, value) in params {
                let bname = Self::to_bounded_name(name)?;
                bparams.try_push((bname, value)).map_err(|_| Error::<T>::TooManyParams)?;
            }
            let id = Self::next_query_id();
            let record = QueryRecord {
                submitter: who,
                raw: braw,
                tran_delta: bdelta.clone(),
                dependent_task: bdep,
                async_task: basync,
                status: QueryStatus::Pending,
                result: None,
                condition: parsed.condition,
                on_true: parsed.on_true,
                on_false: parsed.on_false,
                params: bparams,
            };
            Queries::<T>::insert(id, record);
            Self::deposit_event(Event::QuerySubmitted { id, tran_delta: bdelta });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn set_variable(origin: OriginFor<T>, name: Vec<u8>, value: VariableValue) -> DispatchResult {
            if ensure_signed(origin.clone()).is_err() {
                ensure_root(origin.clone())?;
            }
            let bname = Self::to_bounded_name(name)?;
            Variables::<T>::insert(bname, value);
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn submit_result(origin: OriginFor<T>, query_id: u64, result: Vec<u8>, variable_name: Option<Vec<u8>>) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            let mut rec = Queries::<T>::get(query_id).ok_or(Error::<T>::QueryNotFound)?;
            let bres = Self::to_bounded_result(result.clone())?;
            rec.result = Some(bres.clone());
            rec.status = QueryStatus::Completed;
            Queries::<T>::insert(query_id, rec);
            if let Some(vn) = variable_name {
                let bvn = Self::to_bounded_name(vn)?;
                Variables::<T>::insert(bvn, VariableValue::Bytes(bres));
            }
            Self::deposit_event(Event::QueryCompleted { id: query_id });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn schedule_async(origin: OriginFor<T>, query_id: u64, task_name: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            if !Queries::<T>::contains_key(query_id) {
                return Err(Error::<T>::QueryNotFound.into());
            }
            let bname = Self::to_bounded_name(task_name)?;
            let tid = Self::next_task_id();
            let rec = AsyncTaskRecord {
                owner: who,
                query_id,
                name: bname,
                payload: BoundedVec::default(),
                status: AsyncTaskStatus::Pending,
            };
            AsyncTasks::<T>::insert(tid, rec);
            Self::deposit_event(Event::AsyncScheduled { query: query_id, task: tid });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn complete_async(origin: OriginFor<T>, task_id: u64, output: Vec<u8>) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            let mut rec = AsyncTasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            let bout = Self::to_bounded_result(output)?;
            rec.payload = bout;
            rec.status = AsyncTaskStatus::Completed;
            AsyncTasks::<T>::insert(task_id, rec);
            Self::deposit_event(Event::AsyncCompleted { task: task_id });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn execute_query(origin: OriginFor<T>, query_id: u64) -> DispatchResult {
            let rec0 = Queries::<T>::get(query_id).ok_or(Error::<T>::QueryNotFound)?;
            let effective = if let Ok(w) = ensure_signed(origin.clone()) {
                if w != rec0.submitter {
                    return Err(Error::<T>::PermissionDenied.into());
                }
                w
            } else {
                ensure_root(origin.clone())?;
                rec0.submitter.clone()
            };

            let mut rec = Queries::<T>::get(query_id).ok_or(Error::<T>::QueryNotFound)?;

            let mut ctx: Vec<(BoundedVec<u8, MaxName>, VariableValue)> = Vec::new();
            for (k, v) in Variables::<T>::iter() {
                ctx.push((k, v));
            }
            for (k, v) in rec.params.iter() {
                ctx.push((k.clone(), v.clone()));
            }

            let mut selected = Action::None;
            if let Some(cond) = rec.condition.clone() {
                let token = Self::resolve_u64_ref(&ctx, &cond.token)?;
                let metric_value: u128 = match cond.metric_kind {
                    MetricKind::Price => {
                        let latest = pallet_set_pricing::NextTxId::<T>::get();
                        pallet_set_pricing::Pallet::<T>::recompute_price(token, latest) as u128
                    }
                    MetricKind::Balance => {
                        pallet_set_pricing::TokenBalances::<T>::get(token, &effective).unwrap_or(0)
                    }
                    MetricKind::Time => {
                        pallet_set_time::Pallet::<T>::net_time()
                    }
                };
                let rhs = Self::resolve_u64_ref(&ctx, &cond.rhs)? as u128;
                let result = match cond.op {
                    ConditionOp::Eq => metric_value == rhs,
                    ConditionOp::Ne => metric_value != rhs,
                    ConditionOp::Lt => metric_value < rhs,
                    ConditionOp::Lte => metric_value <= rhs,
                    ConditionOp::Gt => metric_value > rhs,
                    ConditionOp::Gte => metric_value >= rhs,
                };
                Self::deposit_event(Event::ConditionEvaluated { id: query_id, result });
                selected = if result { rec.on_true.clone() } else { rec.on_false.clone() };
            }

            let action_repr = match selected {
                Action::None => Self::to_bounded_name(b"none".to_vec())?,
                Action::Buy { token, amount } => {
                    let rtoken = Self::resolve_u64_ref(&ctx, &token)?;
                    let ramount = Self::resolve_u128_ref(&ctx, &amount)?;
                    let origin_action = frame_system::RawOrigin::Signed(effective.clone()).into();
                    pallet_set_pricing::Pallet::<T>::pump(origin_action, rtoken, ramount)
                        .map_err(|_| Error::<T>::ExecutionFailed)?;
                    Self::to_bounded_name(b"buy".to_vec())?
                }
                Action::Sell { token, amount } => {
                    let rtoken = Self::resolve_u64_ref(&ctx, &token)?;
                    let ramount = Self::resolve_u128_ref(&ctx, &amount)?;
                    let origin_action = frame_system::RawOrigin::Signed(effective.clone()).into();
                    pallet_set_pricing::Pallet::<T>::dump(origin_action, rtoken, ramount)
                        .map_err(|_| Error::<T>::ExecutionFailed)?;
                    Self::to_bounded_name(b"sell".to_vec())?
                }
                Action::Task { name } => {
                    let rname = Self::resolve_name_ref(&ctx, &name)?;
                    let tid = Self::next_task_id();
                    let task_rec = AsyncTaskRecord {
                        owner: effective.clone(),
                        query_id,
                        name: rname.clone(),
                        payload: BoundedVec::default(),
                        status: AsyncTaskStatus::Pending,
                    };
                    AsyncTasks::<T>::insert(tid, task_rec);
                    Self::deposit_event(Event::AsyncScheduled { query: query_id, task: tid });
                    Self::to_bounded_name(rname.to_vec())?
                }
                Action::Deposit { ledger, amount } => {
                    let rledger = Self::resolve_u64_ref(&ctx, &ledger)?;
                    let ramount = Self::resolve_u128_ref(&ctx, &amount)?;
                    let origin_action = frame_system::RawOrigin::Signed(effective.clone()).into();
                    pallet_templar_debt_ledger::Pallet::<T>::deposit(origin_action, rledger, ramount)
                        .map_err(|_| Error::<T>::ExecutionFailed)?;
                    Self::to_bounded_name(b"deposit".to_vec())?
                }
                Action::Withdraw { ledger, amount } => {
                    let rledger = Self::resolve_u64_ref(&ctx, &ledger)?;
                    let ramount = Self::resolve_u128_ref(&ctx, &amount)?;
                    let origin_action = frame_system::RawOrigin::Signed(effective.clone()).into();
                    pallet_templar_debt_ledger::Pallet::<T>::depositor_withdraw(origin_action, rledger, ramount)
                        .map_err(|_| Error::<T>::ExecutionFailed)?;
                    Self::to_bounded_name(b"withdraw".to_vec())?
                }
                Action::Loan { ledger, amount } => {
                    let rledger = Self::resolve_u64_ref(&ctx, &ledger)?;
                    let ramount = Self::resolve_u128_ref(&ctx, &amount)?;
                    let origin_action = frame_system::RawOrigin::Signed(effective.clone()).into();
                    pallet_templar_lending::Pallet::<T>::loan(origin_action, rledger, ramount)
                        .map_err(|_| Error::<T>::ExecutionFailed)?;
                    Self::to_bounded_name(b"loan".to_vec())?
                }
                Action::Repay { loan, amount } => {
                    let rloan = Self::resolve_u64_ref(&ctx, &loan)?;
                    let ramount = Self::resolve_u128_ref(&ctx, &amount)?;
                    let origin_action = frame_system::RawOrigin::Signed(effective.clone()).into();
                    pallet_templar_lending::Pallet::<T>::repay(origin_action, rloan, ramount)
                        .map_err(|_| Error::<T>::ExecutionFailed)?;
                    Self::to_bounded_name(b"repay".to_vec())?
                }
            };

            rec.status = QueryStatus::Completed;
            Queries::<T>::insert(query_id, rec);
            Self::deposit_event(Event::QueryCompleted { id: query_id });
            Self::deposit_event(Event::ActionExecuted { id: query_id, action: action_repr });
            Ok(())
        }
    }
}


#[cfg(test)]
mod templar_tests {
    use super::*;
    use super::pallet as pallet_tan;
    use crate as pallet_set_tan;
    use frame_support::{assert_ok, derive_impl, construct_runtime};
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
            Lending: pallet_templar_lending,
            Tan: pallet,
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
    impl pallet_templar_lending::Config for Test {
        type RuntimeEvent = RuntimeEvent;
    }
    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
    }

    fn new_ext() -> TestExternalities {
        frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
    }

    #[test]
    fn tan_time_metric_and_deposit_action() {
        new_ext().execute_with(|| {
            // time
            assert_ok!(pallet_set_time::Pallet::<Test>::advance_time(frame_system::RawOrigin::Root.into(), 5));
            // ledger schedules allow all
            let s = Schedule::new(0, 0, 0, None);
            let scheds = LedgerSchedules { deposit: s.clone(), withdrawal: s.clone(), loan: s.clone(), repayment: s.clone() };
            assert_ok!(pallet_templar_debt_ledger::Pallet::<Test>::create_ledger(frame_system::RawOrigin::Signed(1).into(), 1, scheds));
            pallet_set_pricing::TokenBalances::<Test>::insert(1, 1, 100u128);
            // register tran delta
            assert_ok!(pallet::Pallet::<Test>::register_tran_delta(frame_system::RawOrigin::Root.into(), b"delta".to_vec()));
            // submit query
            let raw = b"@(delta)::->(dep)<-->(async) if (time(0) >= 0) { deposit(1, 5) }";
            assert_ok!(pallet::Pallet::<Test>::submit_query(frame_system::RawOrigin::Signed(1).into(), raw.to_vec(), vec![]));
            // execute
            assert_ok!(pallet::Pallet::<Test>::execute_query(frame_system::RawOrigin::Signed(1).into(), 1));
        });
    }
}
