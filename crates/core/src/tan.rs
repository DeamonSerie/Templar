use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    Price,
    Balance,
}
pub type Metric = MetricKind;
pub type MetricType = MetricKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum U64Ref {
    Lit(u64),
    Var(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum U128Ref {
    Lit(u128),
    Var(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameRef {
    Lit(String),
    Var(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub metric_kind: MetricKind,
    pub token: U64Ref,
    pub op: ConditionOp,
    pub rhs: U64Ref,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Buy { token: U64Ref, amount: U128Ref },
    Sell { token: U64Ref, amount: U128Ref },
    Task { name: NameRef },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TanQuery {
    pub tran_delta: String,
    pub dependent_task: String,
    pub async_task: String,
    pub interval: Option<(u64, u64)>,
    pub condition: Option<Condition>,
    pub on_true: Action,
    pub on_false: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TanParseError {
    #[error("expected tran delta expression starting with @(")]
    MissingTranDelta,
    #[error("expected :: separator")]
    MissingIntervalSep,
    #[error("expected dependent task ->(...)")]
    MissingDependent,
    #[error("expected async link <-->")]
    MissingAsyncLink,
    #[error("expected async task (...)")]
    MissingAsyncTask,
    #[error("unbalanced parentheses")]
    Unbalanced,
    #[error("trailing input")]
    Trailing,
    #[error("invalid condition")]
    InvalidCondition,
    #[error("invalid action")]
    InvalidAction,
    #[error("invalid variable")]
    InvalidVariable,
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

pub fn parse_tan(input: &str) -> Result<TanQuery, TanParseError> {
    let s = input.trim();
    if !s.starts_with("@(") {
        return Err(TanParseError::MissingTranDelta);
    }
    let open = s.find('(').ok_or(TanParseError::MissingTranDelta)?;
    let close = find_matching(s, open).ok_or(TanParseError::Unbalanced)?;
    let delta = s[open + 1..close].trim().to_string();
    let mut rest = s[close + 1..].trim();
    if !rest.starts_with("::") {
        return Err(TanParseError::MissingIntervalSep);
    }
    rest = rest[2..].trim();
    if !rest.starts_with("->") {
        return Err(TanParseError::MissingDependent);
    }
    rest = rest[2..].trim();
    if !rest.starts_with('(') {
        return Err(TanParseError::MissingDependent);
    }
    let close2 = find_matching(rest, 0).ok_or(TanParseError::Unbalanced)?;
    let dep = rest[1..close2].trim().to_string();
    rest = rest[close2 + 1..].trim();
    if !rest.starts_with("<-->") {
        return Err(TanParseError::MissingAsyncLink);
    }
    rest = rest[4..].trim();
    if !rest.starts_with('(') {
        return Err(TanParseError::MissingAsyncTask);
    }
    let close3 = find_matching(rest, 0).ok_or(TanParseError::Unbalanced)?;
    let async_task = rest[1..close3].trim().to_string();
    let mut trailing = rest[close3 + 1..].trim();
    let mut interval = Some((0, 1));
    if trailing.starts_with('[') {
        let closeb = trailing.find(']').ok_or(TanParseError::Trailing)?;
        let inner = trailing[1..closeb].trim();
        trailing = trailing[closeb + 1..].trim();
        if inner.is_empty() {
            interval = Some((0, 1));
        } else if let Some(comma) = inner.find(',') {
            let a: u64 = inner[..comma].trim().parse().map_err(|_| TanParseError::Trailing)?;
            let b: u64 = inner[comma + 1..].trim().parse().map_err(|_| TanParseError::Trailing)?;
            interval = Some((a, b));
        } else {
            let n: u64 = inner.parse().map_err(|_| TanParseError::Trailing)?;
            interval = Some((0, n));
        }
    }

    let mut condition = None;
    let mut on_true = Action::None;
    let mut on_false = Action::None;

    if !trailing.is_empty() {
        let lower = trailing.to_lowercase();
        if !lower.starts_with("if") {
            return Err(TanParseError::Trailing);
        }
        let mut t = trailing[2..].trim();
        if !t.starts_with('(') {
            return Err(TanParseError::Trailing);
        }
        let cclose = find_matching(t, 0).ok_or(TanParseError::Unbalanced)?;
        let cond_str = t[1..cclose].trim();
        t = t[cclose + 1..].trim();

        let (metric_kind, token, op, rhs) = parse_condition_str(cond_str)?;
        condition = Some(Condition { metric_kind, token, op, rhs });

        if !t.starts_with('{') {
            return Err(TanParseError::Trailing);
        }
        let bclose = find_matching_brace(t, 0).ok_or(TanParseError::Unbalanced)?;
        let action_str = t[1..bclose].trim();
        on_true = parse_action_str(action_str)?;
        t = t[bclose + 1..].trim();

        if t.to_lowercase().starts_with("else") {
            t = t[4..].trim();
            if !t.starts_with('{') {
                return Err(TanParseError::Trailing);
            }
            let bclose2 = find_matching_brace(t, 0).ok_or(TanParseError::Unbalanced)?;
            let action_str2 = t[1..bclose2].trim();
            on_false = parse_action_str(action_str2)?;
            t = t[bclose2 + 1..].trim();
        }

        if !t.is_empty() {
            return Err(TanParseError::Trailing);
        }
    }

    Ok(TanQuery {
        tran_delta: delta,
        dependent_task: dep,
        async_task,
        interval,
        condition,
        on_true,
        on_false,
    })
}

fn parse_metric_str(s: &str) -> Result<(MetricKind, U64Ref), TanParseError> {
    let s = s.trim();
    for (prefix, kind) in [("price", MetricKind::Price), ("balance", MetricKind::Balance)] {
        if let Some(rest) = s.strip_prefix(prefix) {
            let rest = rest.trim();
            if !rest.starts_with('(') {
                return Err(TanParseError::InvalidCondition);
            }
            let close = find_matching(rest, 0).ok_or(TanParseError::InvalidCondition)?;
            let token_str = rest[1..close].trim();
            let token = parse_u64_ref_str(token_str, TanParseError::InvalidCondition)?;
            if !rest[close + 1..].trim().is_empty() {
                return Err(TanParseError::InvalidCondition);
            }
            return Ok((kind, token));
        }
    }
    Err(TanParseError::InvalidCondition)
}

fn parse_u64_ref_str(s: &str, err: TanParseError) -> Result<U64Ref, TanParseError> {
    let s = s.trim();
    if let Some(name) = s.strip_prefix('$') {
        let name = name.trim();
        if name.is_empty() {
            return Err(TanParseError::InvalidVariable);
        }
        Ok(U64Ref::Var(name.to_string()))
    } else {
        let v: u64 = s.parse().map_err(|_| err)?;
        Ok(U64Ref::Lit(v))
    }
}

fn parse_u128_ref_str(s: &str, err: TanParseError) -> Result<U128Ref, TanParseError> {
    let s = s.trim();
    if let Some(name) = s.strip_prefix('$') {
        let name = name.trim();
        if name.is_empty() {
            return Err(TanParseError::InvalidVariable);
        }
        Ok(U128Ref::Var(name.to_string()))
    } else {
        let v: u128 = s.parse().map_err(|_| err)?;
        Ok(U128Ref::Lit(v))
    }
}

fn parse_name_ref_str(s: &str, err: TanParseError) -> Result<NameRef, TanParseError> {
    let s = s.trim();
    if let Some(name) = s.strip_prefix('$') {
        let name = name.trim();
        if name.is_empty() {
            return Err(TanParseError::InvalidVariable);
        }
        Ok(NameRef::Var(name.to_string()))
    } else {
        if s.is_empty() {
            return Err(err);
        }
        Ok(NameRef::Lit(s.to_string()))
    }
}

fn parse_condition_str(s: &str) -> Result<(MetricKind, U64Ref, ConditionOp, U64Ref), TanParseError> {
    let s = s.trim();
    let mut op_pos = None;
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
    let pos = op_pos.ok_or(TanParseError::InvalidCondition)?;
    let lhs = s[..pos].trim();
    let rhs_str = s[pos + op_str.len()..].trim();
    if lhs.is_empty() || rhs_str.is_empty() {
        return Err(TanParseError::InvalidCondition);
    }
    let op = match op_str {
        "==" => ConditionOp::Eq,
        "!=" => ConditionOp::Ne,
        "<" => ConditionOp::Lt,
        "<=" => ConditionOp::Lte,
        ">" => ConditionOp::Gt,
        ">=" => ConditionOp::Gte,
        _ => return Err(TanParseError::InvalidCondition),
    };
    let (metric_kind, token) = parse_metric_str(lhs)?;
    let rhs = parse_u64_ref_str(rhs_str, TanParseError::InvalidCondition)?;
    Ok((metric_kind, token, op, rhs))
}

fn parse_action_str(s: &str) -> Result<Action, TanParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(TanParseError::InvalidAction);
    }
    let lower = s.to_lowercase();
    if lower == "none" {
        return Ok(Action::None);
    }
    for (prefix, is_buy) in [("buy", true), ("sell", false)] {
        if lower.starts_with(prefix) {
            let rest = s[prefix.len()..].trim();
            if !rest.starts_with('(') {
                return Err(TanParseError::InvalidAction);
            }
            let close = find_matching(rest, 0).ok_or(TanParseError::InvalidAction)?;
            if !rest[close + 1..].trim().is_empty() {
                return Err(TanParseError::InvalidAction);
            }
            let inner = rest[1..close].trim();
            let comma = inner.find(',').ok_or(TanParseError::InvalidAction)?;
            let token = parse_u64_ref_str(inner[..comma].trim(), TanParseError::InvalidAction)?;
            let amount = parse_u128_ref_str(inner[comma + 1..].trim(), TanParseError::InvalidAction)?;
            return if is_buy {
                Ok(Action::Buy { token, amount })
            } else {
                Ok(Action::Sell { token, amount })
            };
        }
    }
    let name_expr = if s.starts_with('(') {
        let close = find_matching(s, 0).ok_or(TanParseError::InvalidAction)?;
        if !s[close + 1..].trim().is_empty() {
            return Err(TanParseError::InvalidAction);
        }
        s[1..close].trim()
    } else {
        s
    };
    if name_expr.is_empty() {
        return Err(TanParseError::InvalidAction);
    }
    let name = parse_name_ref_str(name_expr, TanParseError::InvalidAction)?;
    Ok(Action::Task { name })
}

/// Resolve a tran delta expression, unwrapping nested `@(...)` references.
pub fn resolve_tran_delta(input: &str) -> String {
    let s = input.trim();
    if s.starts_with("@(") {
        if let Some(open) = s.find('(') {
            if let Some(close) = find_matching(s, open) {
                return resolve_tran_delta(&s[open + 1..close]);
            }
        }
    }
    s.to_string()
}

/// Returns true if the given value is a tran delta expression (`@(...)`).
pub fn is_tran_delta_expr(input: &str) -> bool {
    let s = input.trim();
    s.starts_with("@(") && s.ends_with(')')
}

/// Unwrap a tran delta expression, returning the inner expression string.
pub fn unwrap_tran_delta(input: &str) -> Option<String> {
    let s = input.trim();
    if !is_tran_delta_expr(s) {
        return None;
    }
    let open = s.find('(')?;
    let close = find_matching(s, open)?;
    Some(s[open + 1..close].trim().to_string())
}

/// Prefix match: target name begins with tran delta name (case-insensitive).
pub fn prefix_match(delta: &str, name: &str) -> bool {
    let d = resolve_tran_delta(delta).to_lowercase();
    name.to_lowercase().starts_with(&d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let q = parse_tan("@(ShimCoin)::->(price_stats)<-->(notify_bot)").unwrap();
        assert_eq!(q.tran_delta, "ShimCoin");
        assert_eq!(q.dependent_task, "price_stats");
        assert_eq!(q.async_task, "notify_bot");
    }

    #[test]
    fn parse_nested() {
        let q = parse_tan("@(@(Foo))::->(@(Bar))<-->(baz)").unwrap();
        assert_eq!(resolve_tran_delta(&q.tran_delta), "Foo");
        assert_eq!(resolve_tran_delta(&q.dependent_task), "Bar");
    }

    #[test]
    fn parse_interval() {
        let q = parse_tan("@(ShimCoin)::->(price_stats)<-->(notify_bot)[5]").unwrap();
        assert_eq!(q.interval, Some((0, 5)));
        let q2 = parse_tan("@(ShimCoin)::->(price_stats)<-->(notify_bot)[]").unwrap();
        assert_eq!(q2.interval, Some((0, 1)));
    }

}
