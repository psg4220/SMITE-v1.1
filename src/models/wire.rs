//! Wire transfer models

/// Direction of wire transfer
#[derive(Debug, Clone, Copy)]
pub enum WireDirection {
    /// Transfer from UnbelievaBoat to SMITE (add to SMITE)
    In,
    /// Transfer from SMITE to UnbelievaBoat (subtract from SMITE)
    Out,
}

/// Result of a wire transfer operation
#[derive(Debug, Clone)]
pub struct WireResult {
    pub smite_balance: f64,
    pub ub_balance: i64,
}
