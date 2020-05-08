/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use fixed::types::extra::*;
use fixed::{traits::ToFixed, FixedI128};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::{self, Display};
use std::iter::{Product, Sum};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Not, Rem, RemAssign, Shl, ShlAssign, Shr,
    ShrAssign, Sub, SubAssign,
};

/// This is a helper type that allows JSON numbers to be decoded in a fashion
/// suitable for differential dataflow: it provides equality, hashing, and
/// Abomonation.
///
/// If the number does not fit into the range of the fixed-point number, it
/// will be [bounded to the valid number range](https://docs.rs/fixed/0.5.5/fixed/struct.FixedI128.html#method.saturating_from_num).
/// `NaN` will be reported as a deserialization error.
///
/// Example usage:
/// ```rust
/// # #[macro_use] extern crate abomonation_derive;
/// use actyxos_sdk::types::FixNum;
/// use actyxos_sdk::types::fixnum_types::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Abomonation, Hash)]
/// struct S {
///     x: FixNum<U5>,
///     y: Option<FixNum<U10>>,
/// }
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixNum<T: LeEqU128>(FixedI128<T>);

impl<T: LeEqU128> Not for FixNum<T> {
    type Output = FixNum<T>;
    fn not(self) -> Self::Output {
        Self(self.0.not())
    }
}

impl<T: LeEqU128> Neg for FixNum<T> {
    type Output = FixNum<T>;
    fn neg(self) -> Self::Output {
        Self(self.0.neg())
    }
}

macro_rules! op1 {
    ($Op:ident $fun:ident) => {
        impl<T: LeEqU128> $Op<FixNum<T>> for FixNum<T> {
            type Output = FixNum<T>;
            fn $fun(self, other: FixNum<T>) -> Self {
                Self((self.0).$fun(other.0))
            }
        }
        impl<T: LeEqU128, U: ToFixed> $Op<U> for FixNum<T> {
            type Output = FixNum<T>;
            fn $fun(self, other: U) -> Self {
                Self((self.0).$fun(FixedI128::saturating_from_num(other)))
            }
        }
        impl<T: LeEqU128> $Op<FixNum<T>> for &FixNum<T> {
            type Output = FixNum<T>;
            fn $fun(self, other: FixNum<T>) -> FixNum<T> {
                FixNum((self.0).$fun(other.0))
            }
        }
        impl<T: LeEqU128, U: ToFixed> $Op<U> for &FixNum<T> {
            type Output = FixNum<T>;
            fn $fun(self, other: U) -> FixNum<T> {
                FixNum((self.0).$fun(FixedI128::saturating_from_num(other)))
            }
        }
        impl<T: LeEqU128> $Op<&FixNum<T>> for FixNum<T> {
            type Output = FixNum<T>;
            fn $fun(self, other: &FixNum<T>) -> Self {
                Self((self.0).$fun(other.0))
            }
        }
        impl<T: LeEqU128> $Op<&FixNum<T>> for &FixNum<T> {
            type Output = FixNum<T>;
            fn $fun(self, other: &FixNum<T>) -> FixNum<T> {
                FixNum((self.0).$fun(other.0))
            }
        }
    };
}

op1!(Add add);
op1!(Sub sub);
op1!(Mul mul);
op1!(Div div);
op1!(Rem rem);

macro_rules! op2 {
    ($Op:ident $fun:ident) => {
        impl<T: LeEqU128> $Op<FixNum<T>> for FixNum<T> {
            fn $fun(&mut self, other: FixNum<T>) {
                (&mut self.0).$fun(other.0);
            }
        }
        impl<T: LeEqU128> $Op<&FixNum<T>> for FixNum<T> {
            fn $fun(&mut self, other: &FixNum<T>) {
                (&mut self.0).$fun(other.0);
            }
        }
        impl<T: LeEqU128, U: ToFixed> $Op<U> for FixNum<T> {
            fn $fun(&mut self, other: U) {
                (&mut self.0).$fun(FixedI128::saturating_from_num(other));
            }
        }
    };
}

op2!(AddAssign add_assign);
op2!(SubAssign sub_assign);
op2!(MulAssign mul_assign);
op2!(DivAssign div_assign);
op2!(RemAssign rem_assign);

macro_rules! op3 {
    ($Op:ident $fun:ident $OpA:ident $funA:ident) => {
        impl<T: LeEqU128> $Op<i32> for FixNum<T> {
            type Output = Self;
            fn $fun(self, other: i32) -> Self {
                Self((self.0).$fun(other))
            }
        }
        impl<T: LeEqU128> $Op<i32> for &FixNum<T> {
            type Output = FixNum<T>;
            fn $fun(self, other: i32) -> FixNum<T> {
                FixNum((self.0).$fun(other))
            }
        }
        impl<T: LeEqU128> $OpA<i32> for FixNum<T> {
            fn $funA(&mut self, other: i32) {
                (&mut self.0).$funA(other)
            }
        }
    };
}

op3!(Shl shl ShlAssign shl_assign);
op3!(Shr shr ShrAssign shr_assign);

impl<T: LeEqU128> Sum for FixNum<T> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        FixNum(FixedI128::<T>::sum(iter.map(|x| x.0)))
    }
}

impl<'a, T: 'a + LeEqU128> Sum<&'a FixNum<T>> for FixNum<T> {
    fn sum<I: Iterator<Item = &'a FixNum<T>>>(iter: I) -> Self {
        FixNum(FixedI128::<T>::sum(iter.map(|x| x.0)))
    }
}

impl<T: LeEqU128> Product for FixNum<T> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        FixNum(FixedI128::<T>::product(iter.map(|x| x.0)))
    }
}

impl<'a, T: 'a + LeEqU128> Product<&'a FixNum<T>> for FixNum<T> {
    fn product<I: Iterator<Item = &'a FixNum<T>>>(iter: I) -> Self {
        FixNum(FixedI128::<T>::product(iter.map(|x| x.0)))
    }
}

impl<T: LeEqU128> FixNum<T> {
    fn from_num<S: ToFixed>(src: S) -> Self {
        Self(FixedI128::saturating_from_num(src))
    }
}

impl<T: LeEqU128> From<FixedI128<T>> for FixNum<T> {
    fn from(x: FixedI128<T>) -> Self {
        Self(x)
    }
}

impl<T: LeEqU128> Display for FixNum<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Construct a FixNum from a numeric input (like an `f64` or `u64`).
///
/// Will panic when called with `NaN` (because that is _not a number_ and you
/// shouldn’t be passing that around).
///
/// ```rust
/// # use actyxos_sdk::types::{FixNum, fixnum, fixnum_types::*};
/// let a: FixNum<U12> = fixnum::<U12, _>(5);
/// let b: FixNum<U12> = a + fixnum(12);
/// assert_eq!(b, fixnum::<U12, _>(17));
/// ```
pub fn fixnum<T: LeEqU128, Src: ToFixed>(src: Src) -> FixNum<T> {
    FixNum::from_num(src)
}

impl<T: LeEqU128> Serialize for FixNum<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_num::<f64>().serialize(serializer)
    }
}

impl<'de, T: LeEqU128> Deserialize<'de> for FixNum<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<FixNum<T>, D::Error> {
        use std::marker::PhantomData;
        struct X<T>(PhantomData<T>);

        impl<'de, T: LeEqU128> Visitor<'de> for X<T> {
            type Value = FixNum<T>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("number")
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(fixnum(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(fixnum(v))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                if v.is_nan() {
                    Err(E::custom("FixNum cannot parse NaN"))
                } else {
                    Ok(fixnum(v))
                }
            }
        }

        deserializer.deserialize_any(X(PhantomData))
    }
}

#[cfg(feature = "dataflow")]
impl<T: LeEqU128> abomonation::Abomonation for FixNum<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    #[cfg_attr(feature = "dataflow", derive(Abomonation))]
    struct S {
        x: FixNum<U5>,
        y: Option<FixNum<U10>>,
    }

    #[test]
    pub fn must_serde() {
        #[allow(clippy::approx_constant)]
        let js = json!({"x": 6213412, "y": 3.1415926});
        let s = serde_json::from_value::<S>(js).unwrap();
        assert_eq!(s.x, fixnum(6213412));
        assert_eq!(
            s.y.unwrap(),
            FixedI128::<U10>::from_bits(0b11__00100_10001).into()
        );
        assert_eq!(
            s.x + FixedI128::<U5>::from_num(0.1),
            FixedI128::<U5>::from_bits(0b101_11101_10011_11001_00100__00011).into()
        );
        let out = serde_json::to_string(&s).unwrap();
        let s2 = serde_json::from_str::<S>(out.as_str()).unwrap();
        assert_eq!(s2, s);
    }

    // this is incorrect on non-x86_64 targets; when encountering this, add a target-conditional
    // implementation of this function
    #[cfg(feature = "dataflow")]
    #[rustfmt::skip]
    fn expected_bits() -> Vec<u8> {
        vec![
            0b100__00011, 0b1_11001_00, 0b1101_1001, 0b101_1, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, //
            1, 0, 0, 0, 0, 0, 0, 0, // Option == Some
            0b100_10001, 0b11__00, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, //
        ]
    }

    #[test]
    #[cfg(feature = "dataflow")]
    pub fn must_abomonate() {
        let mut s = S {
            x: FixNum(FixedI128::<U5>::from_bits(
                0b101_11101_10011_11001_00100__00011,
            )),
            y: Some(FixNum(FixedI128::<U10>::from_bits(0b11__00100_10001))),
        };
        let mut bytes = Vec::new();
        unsafe { abomonation::encode(&s, &mut bytes).unwrap() };
        assert_eq!(bytes, expected_bits());
        bytes[0] += 64;
        assert_eq!(
            s.x,
            FixedI128::<U5>::from_bits(0b101_11101_10011_11001_00100__00011).into()
        );
        s.x += 2;
        let (value, bytes) = unsafe { abomonation::decode::<S>(&mut bytes) }.unwrap();
        assert_eq!(value, &s);
        assert!(bytes.is_empty());
    }

    fn get_value(v: serde_json::Value) -> FixNum<U110> {
        serde_json::from_value(v).unwrap()
    }

    fn get_error(v: serde_json::Value) -> String {
        serde_json::from_value::<FixNum<U110>>(v)
            .unwrap_err()
            .to_string()
    }

    #[test]
    pub fn must_handle_edge_cases() {
        assert_eq!(get_error(json!({})), "invalid type: map, expected number");
        let max = fixnum::<U110, _>(262144);
        assert!(max > fixnum::<U110, _>(131071.9999999));
        assert_eq!(get_value(json!(1000000)), max);
        let min = fixnum::<U110, _>(-262144);
        assert!(min < fixnum::<U110, _>(-131071.9999999));
        assert_eq!(get_value(json!(-1000000)), min);
    }

    #[test]
    #[allow(clippy::eq_op)]
    pub fn must_compute() {
        use crate::types::fixnum_types::*;

        let mut x = fixnum::<U30, _>(12);
        assert_eq!(x + 3, fixnum(15));
        assert_eq!(x + x, fixnum(24));
        assert_eq!(x - 3, fixnum(9));
        assert_eq!(x - x, fixnum(0));
        assert_eq!(x * 3, fixnum(36));
        assert_eq!(x * x, fixnum(144));
        assert_eq!(x / 4, fixnum(3));
        assert_eq!(x / x, fixnum(1));
        assert_eq!(x >> 3, fixnum(1.5));
        assert_eq!(x << 1, fixnum(24));
        assert_eq!(x % 5, fixnum(2));
        assert_eq!(x % fixnum(7), fixnum(5));

        x += -5.5;
        x -= fixnum(2);
        x /= 8;
        x *= fixnum(1.75);
        x %= fixnum(3);
        x >>= 5;
        x <<= 8;

        assert_eq!(x, fixnum(63) / fixnum(8));
        assert_eq!(!x, fixnum(-7.875000001));
        assert_eq!(-x, fixnum(-63) >> 3);

        let v: Vec<FixNum<U10>> = vec![fixnum(1), fixnum(2), fixnum(3)];
        assert_eq!(v.iter().sum::<FixNum<U10>>(), fixnum(6));
        assert_eq!(v.clone().into_iter().sum::<FixNum<U10>>(), fixnum(6));
        assert_eq!(v.iter().product::<FixNum<U10>>(), fixnum(6));
        assert_eq!(v.clone().into_iter().product::<FixNum<U10>>(), fixnum(6));

        let v = v.iter().map(|x| x + 3).collect::<Vec<_>>();
        assert_eq!(v.into_iter().sum::<FixNum<U10>>(), fixnum(15));
    }
}
