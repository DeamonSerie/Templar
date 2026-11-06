use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("out of fuel")]
    OutOfFuel,
    #[error("reverted")]
    Reverted,
    #[error("invalid bytecode")]
    InvalidBytecode,
}

#[derive(Clone, Debug)]
pub struct VmState {
    pub storage: Vec<u8>,
}

pub trait VmExecutor {
    fn execute(&self, bytecode: &[u8], fuel: u64, input: &[u8]) -> Result<Vec<u8>, VmError>;
}

pub struct MockExecutor;

impl VmExecutor for MockExecutor {
    fn execute(&self, bytecode: &[u8], fuel: u64, _input: &[u8]) -> Result<Vec<u8>, VmError> {
        if bytecode.is_empty() {
            return Err(VmError::InvalidBytecode);
        }
        // charge 1 fuel per byte
        if bytecode.len() as u64 > fuel {
            return Err(VmError::OutOfFuel);
        }
        Ok(bytecode.to_vec())
    }
}

pub struct FuelMeter {
    pub remaining: u64,
}

impl FuelMeter {
    pub fn new(fuel: u64) -> Self {
        Self { remaining: fuel }
    }
    pub fn charge(&mut self, cost: u64) -> Result<(), VmError> {
        if self.remaining < cost {
            return Err(VmError::OutOfFuel);
        }
        self.remaining -= cost;
        Ok(())
    }
}
