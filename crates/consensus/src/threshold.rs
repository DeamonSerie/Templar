pub const VALIDATOR_THRESHOLD_NUM: u64 = 2;
pub const VALIDATOR_THRESHOLD_DEN: u64 = 3;
pub const CHECKER_THRESHOLD_NUM: u64 = 2;
pub const CHECKER_THRESHOLD_DEN: u64 = 3;

pub fn approved(validators_total: usize, validators_approve: usize, checkers_total: usize, checkers_approve: usize) -> bool {
    let v_ok = validators_approve * 3 >= validators_total * 2;
    let c_ok = checkers_approve * 3 >= checkers_total * 2;
    v_ok && c_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn threshold() {
        assert!(approved(3, 2, 3, 2));
        assert!(!approved(3, 1, 3, 2));
    }
}
