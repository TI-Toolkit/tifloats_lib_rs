use std::{
    cmp::Ordering,
    ops::{Add, Div, Mul, Neg, Sub},
};

use crate::mantissa::Mantissa;

use bitflags::bitflags;

use crate::FloatError;

#[macro_export]
macro_rules! tifloat {
    (-$mantissa:literal * 10 ^ $exponent:literal) => {{
        let float = Float::new_unchecked(true, $exponent, $mantissa);

        float
    }};

    ($mantissa:literal * 10 ^ $exponent:literal) => {{
        let float = Float::new_unchecked(false, $exponent, $mantissa);

        float
    }};
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Flags: u8 {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseFloatError {
    InvalidFlags,
    InvalidExponent,
    InvalidMantissa,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
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

    /// Intended for use with the tifloat! macro
    pub fn new(negative: bool, exponent: i8, mantissa: u64) -> Result<Self, FloatError> {
        Self::new_unchecked(negative, exponent, mantissa).check()
    }

    pub fn new_unchecked(negative: bool, exponent: i8, mantissa: u64) -> Self {
        Float {
            flags: if negative {
                Flags::NEGATIVE
            } else {
                Flags::empty()
            },
            exponent: (exponent as u8).wrapping_add(Self::EXPONENT_NORM),
            mantissa: Mantissa::from(mantissa).unwrap(),
        }
    }

    /// Convenience method to produce the appropriate packed-BCD mantissa from a
    /// sequence of decimal digits, read from left to right (MSD = `digits[0]`).
    pub fn mantissa_from(digits: &[u8]) -> u64 {
        let digits = Vec::from(digits);

        let dec = digits
            .iter()
            .take(14)
            .enumerate()
            .map(|(index, &value)| -> u64 { (1 << (4 * (13 - index as u64))) * value as u64 })
            .sum::<u64>();

        if digits.len() >= 15 && digits[14] >= 5 {
            (Mantissa::from(dec).unwrap() + Mantissa::ULP).bits()
        } else {
            dec
        }
    }

    /// Given a Float, produces byte representation (flags at index zero).
    pub fn to_raw_bytes(&self) -> [u8; 9] {
        let mut result = vec![self.flags.bits(), self.exponent];

        result.extend(&self.mantissa.bits().to_be_bytes()[1..=7]);

        result.try_into().unwrap()
    }

    /// Given the byte representation (flags at index zero), produces a Float.
    pub fn from_raw_bytes(bytes: [u8; 9]) -> Result<Self, ParseFloatError> {
        let flags = Flags::from_bits(bytes[0]).ok_or(ParseFloatError::InvalidFlags)?;
        let exponent = bytes[1];

        if !(Float::EXPONENT_MIN..Float::EXPONENT_MAX).contains(&exponent) {
            return Err(ParseFloatError::InvalidExponent);
        }

        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[1..]);

        let mantissa = Mantissa::from(u64::from_be_bytes(arr) & Mantissa::MASK)
            .ok_or(ParseFloatError::InvalidMantissa)?;

        Ok(Float {
            flags,
            exponent,
            mantissa,
        })
    }

    /// Checks if this float's exponent is within the allowed range
    pub fn check(self) -> Result<Self, FloatError> {
        if (Self::EXPONENT_MIN..=Self::EXPONENT_MAX).contains(&self.exponent) {
            Ok(self)
        } else {
            Err(FloatError::Overflow)
        }
    }
}

impl Float {
    pub fn is_negative(&self) -> bool {
        self.flags.contains(Flags::NEGATIVE)
    }

    pub fn mark_complex_half(&mut self) {
        self.flags &= Flags::COMPLEX_HALF;
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

impl Neg for Float {
    type Output = Float;

    fn neg(self) -> Float {
        Float {
            flags: self.flags ^ Flags::NEGATIVE,
            ..self
        }
    }
}

impl Add<Float> for Float {
    type Output = Result<Float, FloatError>;

    fn add(self, rhs: Float) -> Self::Output {
        let (a, b) = if self.exponent < rhs.exponent {
            (rhs, self)
        } else {
            (self, rhs)
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
}

impl Sub<Float> for Float {
    type Output = Result<Float, FloatError>;

    fn sub(self, rhs: Float) -> Self::Output {
        self + -rhs
    }
}

impl Mul for Float {
    type Output = Result<Float, FloatError>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut exponent = self.exponent + rhs.exponent - Float::EXPONENT_NORM;

        let (mut mantissa, shift) = self.mantissa.overflowing_mul(rhs.mantissa);

        if shift {
            exponent += 1;

            mantissa = mantissa.shr(1);
        }

        if !(Float::EXPONENT_MIN..Float::EXPONENT_MAX).contains(&exponent) {
            Err(FloatError::Overflow)
        } else {
            Ok(Float {
                flags: self.flags ^ (rhs.flags & Flags::NEGATIVE),
                exponent,
                mantissa,
            })
        }
    }
}

impl Div for Float {
    type Output = Result<Float, FloatError>;

    fn div(self, rhs: Self) -> Self::Output {
        let exponent = self.exponent - rhs.exponent + Float::EXPONENT_NORM;

        let (mut mantissa, needs_norm) = self.mantissa.overflowing_div(rhs.mantissa);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_exponents() {
        assert_eq!(
            tifloat!(0x10000000000000 * 10 ^ -2).exponent,
            Float::EXPONENT_NORM - 2
        );
    }

    #[test]
    fn try_add_sub() {
        let large = tifloat!(0x50000000000000 * 10 ^ 5);
        let neg_large = -large;
        let small = tifloat!(0x50000000000000 * 10 ^ 4);
        let neg_small = -small;

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
                (*combination.0 + *combination.1).ok().unwrap(),
                combination.2
            );
        }
    }

    #[test]
    fn raw_bytes() {
        let float = tifloat!(-0x55000000000000 * 10 ^ 5);

        let repr = [0x80, 0x85, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        assert_eq!(float.to_raw_bytes(), repr);
        assert_eq!(Float::from_raw_bytes(repr).ok().unwrap(), float);
    }

    #[test]
    fn mantissa_from() {
        let cases = [
            (vec![5], 0x50000000000000_u64),
            (vec![1, 2, 3, 4, 5, 6, 7, 8, 9], 0x12345678900000),
            (
                vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 9, 9],
                0x12345678901240,
            ),
        ];

        for (digits, expected) in cases {
            assert_eq!(Float::mantissa_from(&digits), expected);
        }
    }
}
