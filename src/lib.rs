#[cfg(fast)]
mod fast;

#[cfg(fast)]
pub use fast::Float;

#[cfg(not(fast))]
mod correct;

#[cfg(not(fast))]
pub use correct::*;

pub mod error;
pub use error::FloatError;

pub trait TIFloat: Sized + Ord {
    fn is_negative(&self) -> bool;

    /// Negates this float in-place.
    fn negate(&mut self);

    fn mark_complex_half(&mut self);

    fn try_add(&self, other: &Self) -> Result<Self, FloatError>;
    fn try_sub(&self, other: &Self) -> Result<Self, FloatError>;
    fn try_mul(&self, other: &Self) -> Result<Self, FloatError>;
    fn try_div(&self, other: &Self) -> Result<Self, FloatError>;
}
