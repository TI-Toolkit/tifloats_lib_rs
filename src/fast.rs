use std::cmp::Ordering;

use crate::{FloatError, TIFloat};

// shh, clippy, I know this is shady
#[allow(clippy::derive_ord_xor_partial_ord)]
#[derive(PartialEq, PartialOrd)]
pub struct Float(f64);

impl TIFloat for Float {
    fn is_negative(&self) -> bool {
        self.0.is_sign_negative()
    }

    fn negate(&mut self) {
        self.0 = -self.0;
    }

    fn mark_complex_half(&mut self) {
        /* no-op */
    }

    fn try_add(&self, other: &Self) -> Result<Self, FloatError> {
        Ok(Float(self.0 + other.0))
    }

    fn try_sub(&self, other: &Self) -> Result<Self, FloatError> {
        Ok(Float(self.0 - other.0))
    }

    fn try_mul(&self, other: &Self) -> Result<Self, FloatError> {
        Ok(Float(self.0 * other.0))
    }

    fn try_div(&self, other: &Self) -> Result<Self, FloatError> {
        let result = self.0 / other.0;

        if result.is_nan() {
            Err(FloatError::DivideByZero)
        } else {
            Ok(Float(result))
        }
    }
}

impl Eq for Float {}

impl Ord for Float {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}
