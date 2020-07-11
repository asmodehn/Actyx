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
use super::{Event, SourceId};
use crate::tagged::EventKey;
use derive_more::{Display, From, Into};
use num_traits::Bounded;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    iter::FromIterator,
    ops::{Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, Sub, SubAssign},
};

/// Event offset within a [`SourceId`](struct.SourceId.html)’s stream or MIN value
///
/// The event offset is not a number, it rather is an identifier that can be compared
/// to other identifiers. There are 2^63 such values. The `incr` and `decr` functions
/// find the successor or predecessor, respectively. `incr` does not return an option
/// because for the use-case of naming events within a stream it is impossible to exhaust
/// the available set of values.
///
/// The MIN value is not a valid offset, it is sorted before [`Offset::ZERO`](struct.Offset.html#const.ZERO).
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    From,
    Into,
    Display,
)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct OffsetOrMin(#[serde(deserialize_with = "i64_from_minus_one")] i64);

fn i64_from_minus_one<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let o = i64::deserialize(d)?;
    if o < -1 {
        Err(D::Error::custom("number below -1"))
    } else {
        Ok(o)
    }
}

impl OffsetOrMin {
    /// Zero offset, equal to [`Offset::ZERO`](struct.Offset.html#const.ZERO)
    pub const ZERO: OffsetOrMin = OffsetOrMin(0);

    /// Maximum possible offset
    ///
    /// the max Offset needs to fit into an i64 and also needs to be losslessly converted into an f64
    /// due to interop with braindead languages that do not have proper integers.
    ///
    /// See https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER
    pub const MAX: OffsetOrMin = OffsetOrMin(9_007_199_254_740_991);

    /// Minimum value, predecessor of the ZERO offset
    pub const MIN: OffsetOrMin = OffsetOrMin(-1);

    /// This function shall only be used from tests to manufacture events where needed.
    ///
    /// It is intentionally hard to extract the wrapped number from this type because
    /// offsets do not support useful arithmetic operations.
    pub fn mk_test(o: u32) -> Self {
        Self(o.into())
    }

    /// Return the successor to this offset, where ZERO succeeds MIN
    pub fn succ(&self) -> Offset {
        if *self == Self::MAX {
            panic!("cannot increment OffsetOrMin({})", self)
        }
        Offset(self.0 + 1)
    }

    /// Return the predecessor to this offset
    pub fn pred(&self) -> Option<Self> {
        if self > &Self::MIN {
            Some(Self(self.0 - 1))
        } else {
            None
        }
    }
}

impl Default for OffsetOrMin {
    fn default() -> Self {
        Self::MIN
    }
}

impl From<Offset> for OffsetOrMin {
    fn from(o: Offset) -> Self {
        Self(o.0)
    }
}

impl PartialEq<Offset> for OffsetOrMin {
    fn eq(&self, other: &Offset) -> bool {
        OffsetOrMin::from(*other) == *self
    }
}

impl PartialOrd<Offset> for OffsetOrMin {
    fn partial_cmp(&self, other: &Offset) -> Option<Ordering> {
        self.partial_cmp(&OffsetOrMin::from(*other))
    }
}

impl Sub for OffsetOrMin {
    type Output = i64;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<u32> for OffsetOrMin {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        if Self::MAX - self < i64::from(rhs) {
            panic!("cannot add {} to OffsetOrMin({})", rhs, self)
        }
        Self(self.0 + i64::from(rhs))
    }
}

impl Bounded for OffsetOrMin {
    fn min_value() -> Self {
        OffsetOrMin::MIN
    }
    fn max_value() -> Self {
        OffsetOrMin::MAX
    }
}

/// Event offset within a [`SourceId`](struct.SourceId.html)’s stream
///
/// The event offset is not a number, it rather is an identifier that can be compared
/// to other identifiers. There are 2^63 such values. The `incr` and `decr` functions
/// find the successor or predecessor, respectively. `incr` does not return an option
/// because for the use-case of naming events within a stream it is impossible to exhaust
/// the available set of values.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, Display,
)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct Offset(#[serde(deserialize_with = "non_negative_i64")] i64);

fn non_negative_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let o = i64::deserialize(d)?;
    if o < 0 {
        Err(D::Error::custom("negative number"))
    } else {
        Ok(o)
    }
}

impl Offset {
    /// Minimum possible offset, also default value
    pub const ZERO: Offset = Offset(0);

    /// Maximum possible offset
    ///
    /// the max Offset needs to fit into an i64 and also needs to be losslessly converted into an f64
    /// due to interop with braindead languages that do not have proper integers.
    ///
    /// See https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER
    pub const MAX: Offset = Offset(9_007_199_254_740_991);

    /// This function shall only be used from tests to manufacture events where needed.
    ///
    /// It is intentionally hard to extract the wrapped number from this type because
    /// offsets do not support useful arithmetic operations.
    pub fn mk_test(o: u32) -> Self {
        Self(o.into())
    }

    #[doc(hidden)]
    pub unsafe fn from_i64(o: i64) -> Self {
        if o < 0 {
            panic!("got negative Offset")
        }
        Self(o)
    }

    /// Fallible conversion from [`OffsetOrMin`](struct.OffsetOrMin.html)
    ///
    /// This returns `None` when presented with `OffsetOrMin::MIN`.
    pub fn from_offset_or_min(o: OffsetOrMin) -> Option<Self> {
        if o >= Self::ZERO {
            Some(Self(o.0))
        } else {
            None
        }
    }

    /// Return the successor to this offset
    pub fn succ(&self) -> Self {
        if *self == Self::MAX {
            panic!("cannot increment Offset({})", self)
        }
        Self(self.0 + 1)
    }

    /// Return the predecessor to this offset
    pub fn pred(&self) -> Option<Self> {
        if self > &Self::ZERO {
            Some(Self(self.0 - 1))
        } else {
            None
        }
    }

    pub fn pred_or_min(&self) -> OffsetOrMin {
        OffsetOrMin(self.0 - 1)
    }
}

impl Default for Offset {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq<OffsetOrMin> for Offset {
    fn eq(&self, other: &OffsetOrMin) -> bool {
        OffsetOrMin::from(*self) == *other
    }
}

impl PartialOrd<OffsetOrMin> for Offset {
    fn partial_cmp(&self, other: &OffsetOrMin) -> Option<Ordering> {
        OffsetOrMin::from(*self).partial_cmp(other)
    }
}

impl Sub for Offset {
    type Output = i64;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<u32> for Offset {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        if Self::MAX - self < i64::from(rhs) {
            panic!("cannot add {} to Offset({})", rhs, self)
        }
        Self(self.0 + i64::from(rhs))
    }
}

impl Bounded for Offset {
    fn min_value() -> Self {
        Offset::ZERO
    }
    fn max_value() -> Self {
        Offset::MAX
    }
}

/// Multi-dimensional cursor for event streams: an `OffsetMap` describes the set of events
/// given by the event streams of each included source up to the associated [`Offset`](struct.Offset.html).
///
/// All stream delivery modes supported by the Event Service respect the order of offsets
/// of the events published by each single ActyxOS node. This order is consistent with the
/// Lamport timestamp order because both numbers are assigned to published events in strictly
/// monotonically increasing fashion, i.e. greater Offset implies greater Lamport timestamp
/// and vice versa.
///
/// > Note that if the `OffsetMap` contains offset 42 for SourceID `"abc"` it denotes that
/// events with offsets 0 through 42 (inclusive) are included within the `OffsetMap`.
///
/// A common usage pattern is to store the `OffsetMap` describing the events already consumed
/// from an event stream together with the computation results from processing those events
/// (preferably within the same database transaction, if applicable). When restarting the
/// process, this `OffsetMap` can be read and the stream can be resumed from where the process
/// left off previously.
///
/// ## Arithmetics
///
/// `OffsetMap` has a partial order: when the set of events described by one is a strict
/// subset of the set of events described by another, then one is said to be _smaller_ than
/// the other. It may be that one `OffsetMap` contains events that the other does not and vice
/// versa, in which case they are incomparable (`partial_cmp` will return `None`).
///
/// An event may be added into an `OffsetMap` to denote that from the event’s source all events
/// up to this one shall be included in the `OffsetMap`.
///
/// ```rust
/// # use actyxos_sdk::event::{Event, OffsetMap, Payload};
/// let mut offsets: OffsetMap = OffsetMap::empty();
/// let event: Event<Payload> = Event::mk_test("semantics", "name", "42").unwrap();
///
/// // keeping track of having seen this event:
/// offsets += &event;
/// assert!(offsets.contains(&event));
/// ```
///
/// The difference of two offset maps yields the number of events contained within the first
/// but not within the second one (i.e. it counts the size of the difference set).
///
/// # Deserialization
///
/// An OffsetMap only contains valid offsets (non-negative numbers), but during deserialization
/// negative values are tolerated and ignored. This is to keep compatibility with previously
/// documented API endpoints.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(from = "HashMap<SourceId, i64>")]
pub struct OffsetMap(HashMap<SourceId, Offset>);

impl OffsetMap {
    /// The empty `OffsetMap` is equivalent to the beginning of time, it does not contain any
    /// event.
    pub fn empty() -> Self {
        Default::default()
    }

    /// Check whether the given Event’s offset and source ID are contained within this `OffsetMap`.
    pub fn contains<T>(&self, event: &Event<T>) -> bool {
        self.0
            .get(&event.stream.source)
            .copied()
            .unwrap_or_default()
            >= event.offset
    }

    /// Check whether the given source contributes to the set of events in this OffsetMap
    pub fn contains_source(&self, source: &SourceId) -> bool {
        self.0.contains_key(source)
    }

    /// Retrieve the offset stored for the given source
    ///
    /// The returned value is `OffsetOrMin::MIN` if nothing is stored for the given source.
    pub fn offset(&self, source: &SourceId) -> OffsetOrMin {
        self.get(source)
            .map(|o| o.into())
            .unwrap_or(OffsetOrMin::MIN)
    }

    /// Retrieves the offset stored for the given source
    pub fn get(&self, source: &SourceId) -> Option<Offset> {
        self.0.get(&source).cloned()
    }

    /// Counts the number of offsets spanned by this OffsetMap.
    pub fn size(&self) -> u64 {
        self - &OffsetMap::empty()
    }

    /// Merge the other OffsetMap into this one, taking the union of their event sets.
    pub fn union_with<'a>(&'a mut self, other: &OffsetMap) {
        for (k, v) in &other.0 {
            self.0
                .entry(*k)
                .and_modify(|me| *me = (*me).max(*v))
                .or_insert(*v);
        }
    }

    /// Compute the union of two sets of events described by OffsetMaps
    pub fn union(&self, other: &OffsetMap) -> OffsetMap {
        let mut copy = self.clone();
        copy.union_with(other);
        copy
    }

    /// Compute the intersection of two sets of events described by OffsetMaps
    pub fn intersection_with(&mut self, other: &OffsetMap) {
        let keys = self.0.keys().cloned().collect::<Vec<_>>();
        for key in keys.into_iter() {
            let offset = other.offset(&key).min(self.offset(&key));
            if let Some(offset) = Offset::from_offset_or_min(offset) {
                self.0.insert(key, offset);
            } else {
                self.0.remove(&key);
            }
        }
    }

    /// Compute the intersection of two sets of events described by OffsetMaps
    pub fn intersection(&self, other: &OffsetMap) -> OffsetMap {
        let left = self.0.keys().collect::<BTreeSet<_>>();
        let right = other.0.keys().collect::<BTreeSet<_>>();
        let keys = left.intersection(&right);
        Self(
            keys.map(|key| {
                (
                    **key,
                    self.0
                        .get(key)
                        .copied()
                        .unwrap_or_default()
                        .min(other.0.get(key).copied().unwrap_or_default()),
                )
            })
            .collect(),
        )
    }

    pub fn into_inner(self) -> HashMap<SourceId, Offset> {
        self.0
    }

    /// An iterator over all sources that contribute events to this OffsetMap
    pub fn sources<'a>(&'a self) -> impl Iterator<Item = SourceId> + 'a {
        self.0.keys().cloned()
    }

    /// An iterator over all sources that contribute events to this OffsetMap including their offset
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (SourceId, Offset)> + 'a {
        self.0.iter().map(|(k, v)| (*k, *v))
    }

    /// Update entry for source if the given offset is larger than the stored one
    /// and return the previous offset for this source
    pub fn update(&mut self, source: SourceId, offset: Offset) -> Option<OffsetOrMin> {
        let previous = self.offset(&source);
        if offset > previous {
            self.0.insert(source, offset);
            Some(previous)
        } else {
            None
        }
    }
}

impl PartialOrd for OffsetMap {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        let lhs = self;
        let mut lt = false;
        let mut eq = false;
        let mut gt = false;
        let mut cross = |a: &OffsetOrMin, b: &OffsetOrMin| -> bool {
            match Ord::cmp(a, b) {
                Ordering::Less => lt = true,
                Ordering::Equal => eq = true,
                Ordering::Greater => gt = true,
            }
            lt && gt
        };
        for (k, a) in &lhs.0 {
            let b = &rhs.offset(k);
            if cross(&OffsetOrMin::from(*a), b) {
                return None;
            }
        }
        for (k, b) in &rhs.0 {
            let a = &lhs.offset(k);
            if cross(a, &OffsetOrMin::from(*b)) {
                return None;
            }
        }
        if lt {
            Some(Ordering::Less)
        } else if gt {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl AsRef<HashMap<SourceId, Offset>> for OffsetMap {
    fn as_ref(&self) -> &HashMap<SourceId, Offset> {
        &self.0
    }
}

impl Default for OffsetMap {
    fn default() -> Self {
        OffsetMap(HashMap::new())
    }
}

impl From<HashMap<SourceId, Offset>> for OffsetMap {
    fn from(map: HashMap<SourceId, Offset>) -> Self {
        Self(map)
    }
}

impl From<HashMap<SourceId, i64>> for OffsetMap {
    fn from(map: HashMap<SourceId, i64>) -> Self {
        map.into_iter()
            .filter_map(|(s, o)| if o >= 0 { Some((s, Offset(o))) } else { None })
            .collect()
    }
}

impl FromIterator<(SourceId, Offset)> for OffsetMap {
    fn from_iter<T: IntoIterator<Item = (SourceId, Offset)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> AddAssign<&Event<T>> for OffsetMap {
    fn add_assign(&mut self, other: &Event<T>) {
        let off = self.0.entry(other.stream.source).or_default();
        if *off < other.offset {
            *off = other.offset;
        }
    }
}

impl AddAssign<&EventKey> for OffsetMap {
    fn add_assign(&mut self, other: &EventKey) {
        let off = self.0.entry(other.source).or_default();
        if *off < other.offset {
            *off = other.offset;
        }
    }
}

impl<T> SubAssign<&Event<T>> for OffsetMap {
    /// Ensure that the given event is no longer contained within this OffsetMap.
    fn sub_assign(&mut self, other: &Event<T>) {
        let off = self.0.entry(other.stream.source).or_default();
        if *off >= other.offset {
            if let Some(o) = other.offset.pred() {
                *off = o;
            } else {
                self.0.remove(&other.stream.source);
            }
        }
    }
}

impl SubAssign<&EventKey> for OffsetMap {
    /// Ensure that the given event is no longer contained within this OffsetMap.
    fn sub_assign(&mut self, other: &EventKey) {
        let off = self.0.entry(other.source).or_default();
        if *off >= other.offset {
            if let Some(o) = other.offset.pred() {
                *off = o;
            } else {
                self.0.remove(&other.source);
            }
        }
    }
}

impl Sub<OffsetMap> for OffsetMap {
    type Output = u64;
    fn sub(self, other: Self) -> u64 {
        &self - &other
    }
}

impl Sub<&OffsetMap> for &OffsetMap {
    type Output = u64;
    fn sub(self, other: &OffsetMap) -> u64 {
        let mut ret = 0;
        for (k, a) in &self.0 {
            let b = &other.0.get(k).copied().unwrap_or_default();
            if a > b {
                ret += (a.0 - b.0) as u64;
            }
        }
        ret
    }
}

impl BitAnd for OffsetMap {
    type Output = OffsetMap;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(&rhs)
    }
}

impl BitAnd for &OffsetMap {
    type Output = OffsetMap;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(rhs)
    }
}

impl BitAndAssign for OffsetMap {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = &*self & &rhs;
    }
}

impl BitOr for OffsetMap {
    type Output = OffsetMap;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.union_with(&rhs);
        self
    }
}

impl BitOr for &OffsetMap {
    type Output = OffsetMap;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOrAssign for OffsetMap {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = &*self | &rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{LamportTimestamp, Payload, StreamInfo, TimeStamp},
        fish_name, semantics, source_id,
    };
    use std::str::FromStr;

    fn mk_event(source: &str, offset: u32) -> Event<Payload> {
        Event {
            lamport: LamportTimestamp::new(1),
            stream: StreamInfo {
                semantics: semantics!("dummy"),
                name: fish_name!("dummy"),
                source: SourceId::from_str(source).unwrap(),
            },
            offset: Offset::mk_test(offset),
            timestamp: TimeStamp::now(),
            payload: Payload::default(),
        }
    }

    #[test]
    #[allow(clippy::eq_op)]
    pub fn must_calculate_offset_map() {
        let ev1 = &mk_event("a", 1);
        let ev2 = &mk_event("b", 2);
        let ev3 = &mk_event("c", 1);

        let empty = &OffsetMap::default();
        let mut map1 = empty.clone();
        map1 += ev1;
        let mut map2 = map1.clone();
        map2 += ev2;
        let mut map3 = map1.clone();
        map3 += ev3;

        assert_eq!(&map2 - &map2, 0);
        assert_eq!(&map2 - &map1, 3);
        assert_eq!(&map2 - empty, 5);

        assert!(map2.contains(ev1));
        assert!(map1.contains(ev1));
        assert!(map2.contains(ev2));
        assert!(!map1.contains(ev2));

        assert!(map1 > *empty);
        assert!(map1 <= map1);
        assert!(map1 >= map1);
        assert!(map3 > map1);
        assert!(map2 > map1);
        assert!(map2.partial_cmp(&map3).is_none());

        // also need to test the consuming Sub impl
        assert_eq!(map1 - map2, 0);
    }

    #[test]
    pub fn must_set_op() {
        let left = OffsetMap::from(
            [
                (source_id!("a"), Offset::mk_test(1)),
                (source_id!("b"), Offset::mk_test(2)),
                (source_id!("c"), Offset::mk_test(3)),
                (source_id!("d"), Offset::mk_test(4)),
            ]
            .iter()
            .copied()
            .collect::<HashMap<_, _>>(),
        );

        let right = OffsetMap::from(
            [
                (source_id!("b"), Offset::mk_test(4)),
                (source_id!("c"), Offset::mk_test(3)),
                (source_id!("d"), Offset::mk_test(2)),
                (source_id!("e"), Offset::mk_test(1)),
            ]
            .iter()
            .copied()
            .collect::<HashMap<_, _>>(),
        );

        let union = OffsetMap::from(
            [
                (source_id!("a"), Offset::mk_test(1)),
                (source_id!("b"), Offset::mk_test(4)),
                (source_id!("c"), Offset::mk_test(3)),
                (source_id!("d"), Offset::mk_test(4)),
                (source_id!("e"), Offset::mk_test(1)),
            ]
            .iter()
            .copied()
            .collect::<HashMap<_, _>>(),
        );

        let intersection = OffsetMap::from(
            [
                (source_id!("b"), Offset::mk_test(2)),
                (source_id!("c"), Offset::mk_test(3)),
                (source_id!("d"), Offset::mk_test(2)),
            ]
            .iter()
            .copied()
            .collect::<HashMap<_, _>>(),
        );

        assert_eq!(left.union(&right), union);
        assert_eq!(left.intersection(&right), intersection);
        assert_eq!(&left | &right, union);
        assert_eq!(left & right, intersection);
    }

    #[test]
    fn must_to_string() {
        assert_eq!(OffsetOrMin(12).to_string(), "12");
        assert_eq!(Offset::mk_test(3).to_string(), "3");
    }
}
