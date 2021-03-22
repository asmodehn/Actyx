/*
 * Copyright 2021 Actyx AG
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
use std::{
    fmt::{self, Debug, Display, Formatter},
    ops::{Add, Sub},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, TimeZone, Utc};
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};

/// Microseconds since the UNIX epoch, without leap seconds and in UTC
///
/// ```
/// use actyxos_sdk::Timestamp;
/// use chrono::{DateTime, Utc, TimeZone};
///
/// let timestamp = Timestamp::now();
/// let micros_since_epoch: u64 = timestamp.into();
/// let date_time: DateTime<Utc> = timestamp.into();
///
/// assert_eq!(timestamp.as_i64() * 1000, date_time.timestamp_nanos());
/// assert_eq!(Timestamp::from(date_time), timestamp);
/// ```
#[derive(Copy, Clone, Debug, Default, From, Into, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn now() -> Timestamp {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).expect("Time went waaaay backwards");
        Timestamp::new(duration.as_micros() as u64)
    }
    #[deprecated(since = "0.2.1", note = "use .into()")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

impl Into<DateTime<Utc>> for Timestamp {
    fn into(self) -> DateTime<Utc> {
        Utc.timestamp((self.0 / 1_000_000) as i64, (self.0 % 1_000_000) as u32 * 1000)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt.timestamp_nanos() as u64 / 1000)
    }
}

impl Sub<u64> for Timestamp {
    type Output = Timestamp;
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = i64;
    fn sub(self, rhs: Timestamp) -> Self::Output {
        self.0 as i64 - rhs.0 as i64
    }
}

impl Add<u64> for Timestamp {
    type Output = Timestamp;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

/// A logical timestamp taken from a [`Lamport clock`](https://en.wikipedia.org/wiki/Lamport_timestamps)
///
/// The lamport clock in an ActyxOS system is increased by the ActyxOS node whenever:
///
/// - an event is emitted
/// - a heartbeat is received
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default, From, Into)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct LamportTimestamp(u64);

impl LamportTimestamp {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn incr(self) -> Self {
        LamportTimestamp::new(self.0 + 1)
    }
    #[deprecated(since = "0.2.1", note = "use .into()")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

impl Display for LamportTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "LT({})", Into::<u64>::into(*self))
    }
}

impl Add<u64> for LamportTimestamp {
    type Output = LamportTimestamp;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}