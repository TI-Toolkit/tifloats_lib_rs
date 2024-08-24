use std::ops::{Add, Sub};

const DEC_TO_BCD: [u64; 100] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, //
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, //
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, //
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, //
    0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, //
    0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, //
    0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, //
    0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, //
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, //
    0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, //
];

#[derive(Copy, Clone, Debug)]
pub(crate) struct Mantissa {
    data: u64,
}

impl Mantissa {
    pub const PI: Mantissa = Mantissa {
        data: 0x0031415926535898,
    };

    pub const ONE: Mantissa = Mantissa {
        data: 0x0010000000000000,
    };

    pub const FIVE: Mantissa = Mantissa {
        data: 0x0050000000000000,
    };

    pub const E: Mantissa = Mantissa {
        data: 0x0027182818284590,
    };

    pub const ULP: Mantissa = Mantissa {
        data: 0x0000000000000001,
    };
}

impl Mantissa {
    pub const MASK: u64 = 0x00FFFFFFFFFFFFFF;
    /// The maximum value that the Mantissa can store, in base 10
    pub const MAX_10: u64 = 99999999999999;

    pub fn tens_complement(&self) -> Mantissa {
        let t1 = (!0) - self.data;
        let t2 = t1 + 0x1;
        let t3 = t1 ^ 0x1;
        let t4 = t2 ^ t3;
        let t5 = !t4 & 0x1111111111111110;
        let t6 = (t5 >> 2) | (t5 >> 3);

        Mantissa {
            data: (t2 - t6) & Mantissa::MASK,
        }
    }

    #[cfg(test)]
    pub fn hex(&self) -> String {
        format!("{:X}", self.data)
    }

    pub fn bits(&self) -> u64 {
        self.data
    }

    pub fn is_zero(&self) -> bool {
        self.data == 0
    }

    pub fn msd(&self) -> u8 {
        ((self.data >> (13 * 4)) & 0xFF) as u8
    }

    pub fn from(bits: u64) -> Option<Self> {
        if 0 != (((((bits >> 1) & 0x0077777777777777) + 0x0033333333333333) & 0x0088888888888888)
            | (bits & !Mantissa::MASK))
        {
            None
        } else {
            Some(Mantissa { data: bits })
        }
    }

    pub const fn from_unchecked(bits: u64) -> Self {
        Mantissa { data: bits }
    }

    pub fn check(&self) -> bool {
        0 == (((((self.data >> 1) & 0x0077777777777777) + 0x0033333333333333) & 0x0088888888888888)
            | (self.data & !Mantissa::MASK))
    }

    pub fn to_dec(self) -> u64 {
        let mut output = 0u64;
        for byte in self.data.to_be_bytes() {
            output = output * 100 + (byte - 6 * (byte >> 4)) as u64;
        }

        output
    }

    pub fn from_dec(mut data: u64) -> Self {
        let mut result = 0;
        let mut shift = 0;

        for _ in 0..8 {
            result += (DEC_TO_BCD[(data % 100) as usize]) << shift;
            data /= 100;
            shift += 8;
        }

        Mantissa { data: result }
    }

    pub fn from_dec_normalized(mut data: u64) -> (Self, u8) {
        if data == 0 {
            return (Mantissa::from_unchecked(data), 0)
        }

        let mut mantissa = Mantissa::from_dec(data);

        let mut count = (mantissa.data.leading_zeros()/4) as u8 - 2;

        (mantissa.shl(count), count)
    }
}

impl Add for Mantissa {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.overflowing_add(rhs).0
    }
}

impl PartialEq for Mantissa {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Eq for Mantissa {}

impl Sub for Mantissa {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.overflowing_sub(rhs).0
    }
}

/// # Core operations
impl Mantissa {
    /// Returns the unnormalized sum and the overflow flag.
    pub fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        if rhs.is_zero() {
            return (self, false);
        }

        let t1 = self.data + 0x0066666666666666;
        let t2 = t1 + rhs.data;
        let t3 = t1 ^ rhs.data;
        let t4 = t2 ^ t3;
        let t5 = !t4 & 0x0111111111111110;
        let t6 = (t5 >> 2) | (t5 >> 3);

        let result = t2 - t6;

        (
            Mantissa {
                data: result & Mantissa::MASK,
            },
            (result & !Mantissa::MASK) != 0,
        )
    }

    /// Returns the unnormalized difference and the overflow (sign change) flag.
    ///
    /// Differences can require multiple shls to normalize- consider 102-101=001
    pub fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        if self.data > rhs.data {
            (self.overflowing_add(rhs.tens_complement()).0, false)
        } else {
            (rhs.overflowing_add(self.tens_complement()).0, true)
        }
    }

    /// Returns the unnormalized product and the overflow flag
    pub fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        let full_product = (self.to_dec() as u128) * (rhs.to_dec() as u128);

        let half_product: u64 = (full_product / 10_u128.pow(13)).try_into().unwrap();
        let mantissa = Mantissa::from_dec(half_product);

        (mantissa, half_product > Mantissa::MAX_10)
    }

    /// Returns the unnormalized quotient and a flag indicating if normalization
    /// (via a single right shift) is required.
    pub fn overflowing_div(self, rhs: Self) -> (Self, bool) {
        let dividend = (self.to_dec() as u128) * 10_u128.pow(14);
        let divisor = rhs.to_dec() as u128;

        let quotient = ((dividend + (divisor >> 1)) / divisor).try_into().unwrap();

        let mantissa = Mantissa::from_dec(quotient);

        (mantissa, quotient > Mantissa::MAX_10)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn shr(self, distance: u8) -> Self {
        if distance >= 15 {
            return Mantissa { data: 0 };
        }

        let mut result = self.data >> ((distance * 4) as u64);

        // rounding
        if distance != 0 && (self.data >> (((distance - 1) * 4) as u64) & 0xF) >= 5 {
            result = (Mantissa { data: result } + Mantissa::ULP).data;
        }

        Mantissa { data: result }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn shl(self, distance: u8) -> Self {
        Mantissa {
            data: (self.data << ((distance * 4) as u64)) & Mantissa::MASK,
        }
    }
}

impl Mantissa {
    pub fn digits(&self) -> Vec<u8> {
        let mut nibbles = Vec::with_capacity(16);
        for i in (0..14).rev() {
            let nibble = (self.data >> 4*i) & 0x0F;
            nibbles.push(nibble as u8);
        }

        nibbles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        const SUM: Mantissa = Mantissa {
            data: 0x0058598744820488,
        };

        assert_eq!((Mantissa::PI + Mantissa::E).hex(), SUM.hex());
    }

    #[test]
    fn sub() {
        const PI_PLUS_ONE: Mantissa = Mantissa {
            data: 0x0041415926535898,
        };

        assert_eq!((PI_PLUS_ONE - Mantissa::ONE).hex(), Mantissa::PI.hex());

        let overflowed_sub = Mantissa::PI.overflowing_sub(PI_PLUS_ONE);

        assert!(overflowed_sub.1);
        assert_eq!(overflowed_sub.0.hex(), Mantissa::ONE.hex());
    }

    #[test]
    fn shr() {
        const ONE: Mantissa = Mantissa {
            data: 0x0010000000000000,
        };

        const BASICALLY_TEN: Mantissa = Mantissa {
            data: 0x0099999999999999,
        };

        assert_eq!(BASICALLY_TEN.shr(1).hex(), ONE.hex());
    }

    #[test]
    fn to_from_dec() {
        assert_eq!(Mantissa::from_dec(31415926535898), Mantissa::PI);
        assert_eq!(Mantissa::PI.to_dec(), 31415926535898);
    }

    #[test]
    fn mul() {
        assert_eq!(
            Mantissa::PI.overflowing_mul(Mantissa::ONE),
            (Mantissa::PI, false)
        );

        assert_eq!(
            Mantissa::FIVE.overflowing_mul(Mantissa::FIVE),
            (
                Mantissa {
                    data: 0x0250000000000000
                },
                true
            )
        );

        assert_eq!(
            (Mantissa {
                data: 0x0014285714285714
            })
            .overflowing_mul(Mantissa {
                data: 0x0070000000000000
            }),
            (
                Mantissa {
                    data: 0x0099999999999998
                },
                false
            )
        );
    }

    #[test]
    fn div() {
        assert_eq!(
            Mantissa {
                data: 0x6000000000000000
            }
            .overflowing_div(Mantissa {
                data: 0x7000000000000000
            }),
            (
                Mantissa {
                    data: 0x0085714285714286
                },
                false
            )
        );

        assert_eq!(
            Mantissa {
                data: 0x1000000000000000
            }
            .overflowing_div(Mantissa {
                data: 0x3000000000000000
            }),
            (
                Mantissa {
                    data: 0x0033333333333333
                },
                false
            )
        );

        assert_eq!(
            Mantissa {
                data: 0x0355000000000000
            }
            .overflowing_div(Mantissa {
                data: 0x1130000000000000
            }),
            (
                Mantissa {
                    data: 0x0031415929203540
                },
                false
            )
        );
    }

    #[test]
    fn digits() {
        assert_eq!(
            Mantissa {
                data: 0x0014285714285714
            }
            .digits(),
            vec![1, 4, 2, 8, 5, 7, 1, 4, 2, 8, 5, 7, 1, 4]
        )
    }
}
