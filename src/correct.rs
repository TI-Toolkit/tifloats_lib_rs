mod mantissa;

use std::cmp::Ordering;

use mantissa::Mantissa;

use bitflags::bitflags;

use crate::{FloatError, TIFloat};

#[macro_export]
macro_rules! tifloat {
    (-$mantissa:literal * 10 ^ $exponent:literal) => {{
        let float = Float::from(true, 0x80 + ($exponent), $mantissa);

        float
    }};

    ($mantissa:literal * 10 ^ $exponent:literal) => {{
        let float = Float::from(false, 0x80 + ($exponent), $mantissa);

        float
    }};
}

bitflags! {
    struct Flags: u8 {
        /// If this bit is set, the number is undefined (used for initial sequence values)
        const UNDEFINED = 0x02;
        /// If both bits 2 and 3 are set and bit 1 is clear, the number is half of a complex variable.
        const COMPLEX_HALF = 0x0C;
        /// Uncertain. Most likely if set, the number has not been modified since the last graph.
        const IDK = 0x40;
        /// If set, the number is negative.
        const NEGATIVE = 0x80;
    }
}

#[derive(PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Float {
    flags: Flags,
    exponent: u8,
    mantissa: Mantissa,
}

impl Float {
    const EXPONENT_NORM: u8 = 0x80;
    const EXPONENT_MAX: u8 = Float::EXPONENT_NORM + 99;
    const EXPONENT_MIN: u8 = Float::EXPONENT_NORM - 99;

    fn measure(&self) -> u128 {
        ((!self.is_negative() as u128) << 127)
            | ((self.exponent as u128) << 56)
            | self.mantissa.bits() as u128
    }

    fn negated(&self) -> Self {
        Float {
            flags: self.flags ^ Flags::NEGATIVE,
            ..*self
        }
    }

    /// Intended for use with the tifloat! macro
    pub fn from(negative: bool, exponent: u8, mantissa: u64) -> Float {
        Float {
            flags: if negative {
                Flags::NEGATIVE
            } else {
                Flags::empty()
            },
            exponent,
            mantissa: Mantissa::from(mantissa),
        }
    }
}

impl TIFloat for Float {
    fn is_negative(&self) -> bool {
        self.flags.contains(Flags::NEGATIVE)
    }

    fn negate(&mut self) {
        self.flags ^= Flags::NEGATIVE;
    }

    fn mark_complex_half(&mut self) {
        self.flags &= Flags::COMPLEX_HALF;
    }

    fn try_add(&self, other: &Self) -> Result<Self, FloatError> {
        let (a, b) = if self.exponent < other.exponent {
            (other, self)
        } else {
            (self, other)
        };

        let b_mantissa = b.mantissa.shr(a.exponent - b.exponent);
        let mut exponent = a.exponent;

        if a.is_negative() == b.is_negative() {
            // add mantissas
            let (mut mantissa, overflow) = a.mantissa.overflowing_add(b_mantissa);

            if overflow {
                exponent += 1;

                mantissa = mantissa.shr(1) + Mantissa::ONE;
            }

            if exponent > Float::EXPONENT_MAX {
                Err(FloatError::Overflow)
            } else {
                Ok(Float {
                    flags: a.flags,
                    exponent,
                    mantissa,
                })
            }
        } else {
            // subtract mantissas
            let (mut mantissa, overflow) = a.mantissa.overflowing_sub(b_mantissa);

            let mut flags = a.flags;
            if overflow {
                flags ^= Flags::NEGATIVE;
            }

            while mantissa.msd() == 0 {
                exponent -= 1;

                mantissa = mantissa.shl(1);
            }

            if exponent < Float::EXPONENT_MIN {
                Err(FloatError::Overflow)
            } else {
                Ok(Float {
                    flags,
                    exponent,
                    mantissa,
                })
            }
        }
    }

    fn try_sub(&self, other: &Self) -> Result<Self, FloatError> {
        self.try_add(&other.negated())
    }

    fn try_mul(&self, other: &Self) -> Result<Self, FloatError> {
        let mut exponent = self.exponent + other.exponent - Float::EXPONENT_NORM;

        let (mut mantissa, shift) = self.mantissa.overflowing_mul(other.mantissa);

        if shift {
            exponent += 1;

            mantissa = mantissa.shr(1);
        }

        if !(Float::EXPONENT_MIN..Float::EXPONENT_MAX).contains(&exponent) {
            Err(FloatError::Overflow)
        } else {
            Ok(Float {
                flags: self.flags ^ (other.flags & Flags::NEGATIVE),
                exponent,
                mantissa,
            })
        }
    }

    fn try_div(&self, other: &Self) -> Result<Self, FloatError> {
        let exponent = self.exponent - other.exponent + Float::EXPONENT_NORM;

        let (mut mantissa, needs_norm) = self.mantissa.overflowing_div(other.mantissa);

        if needs_norm {
            mantissa = mantissa.shr(1);
        }

        if !(Float::EXPONENT_MIN..Float::EXPONENT_MAX).contains(&exponent) {
            Err(FloatError::Overflow)
        } else {
            Ok(Float {
                flags: self.flags,
                exponent,
                mantissa,
            })
        }
    }
}

impl PartialOrd for Float {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.measure().partial_cmp(&other.measure())
    }
}

impl Ord for Float {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_add_sub() {
        let large = tifloat!(0x50000000000000 * 10 ^ 5);
        let neg_large = large.negated();
        let small = tifloat!(0x50000000000000 * 10 ^ 4);
        let neg_small = small.negated();

        let combinations = [
            (&large, &small, tifloat!(0x55000000000000 * 10 ^ 5)),
            (&small, &large, tifloat!(0x55000000000000 * 10 ^ 5)),
            (&neg_large, &neg_small, tifloat!(-0x55000000000000 * 10 ^ 5)),
            (&neg_small, &neg_large, tifloat!(-0x55000000000000 * 10 ^ 5)),
            (&small, &small, tifloat!(0x10000000000000 * 10 ^ 5)),
            (&large, &large, tifloat!(0x10000000000000 * 10 ^ 6)),
            (&neg_small, &neg_small, tifloat!(-0x10000000000000 * 10 ^ 5)),
            (&neg_large, &neg_large, tifloat!(-0x10000000000000 * 10 ^ 6)),
            (&large, &neg_small, tifloat!(0x45000000000000 * 10 ^ 5)),
            (&neg_large, &small, tifloat!(-0x45000000000000 * 10 ^ 5)),
        ];

        for combination in combinations {
            assert_eq!(
                combination.0.try_add(combination.1).ok().unwrap(),
                combination.2
            );
        }
    }

    #[test]
    fn try_mul_div() {
        // trust me it works (i hope)
    }
}
