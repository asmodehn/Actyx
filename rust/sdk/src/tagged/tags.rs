use crate::{
    event::{FishName, Semantics},
    types::ArcVal,
    ParseError,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    ops::{Add, Deref},
    sync::Arc,
};

/// Macro for constructing a [`Tag`](event/struct.Tag.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{tag, tagged::Tag};
/// let tag: Tag = tag!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{tag, tagged::Tag};
/// let tag: Tag = tag!("");
/// ```
#[macro_export]
macro_rules! tag {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use std::convert::TryFrom;
        $crate::tagged::Tag::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a set of [`Tag`](event/struct.Tag.html) values.
///
/// The values accepted are either
///  - non-empty string literals
///  - normal expressions (enclosed in parens if multiple tokens)
///
/// ```rust
/// use actyxos_sdk::{tag, tags, semantics, event::Semantics, tagged::{Tag, TagSet}};
/// use std::collections::BTreeSet;
///
/// let sem: Semantics = semantics!("b");
/// let tags: TagSet = tags!("a", sem);
///
/// let mut expected = BTreeSet::new();
/// expected.insert(tag!("a"));
/// expected.insert(tag!("semantics:b"));
/// assert_eq!(tags, TagSet::from(expected));
/// ```
#[macro_export]
macro_rules! tags {
    ($($expr:expr),*) => {{
        let mut tags = Vec::new();
        $(
            {
                mod y {
                    $crate::assert_len! { $expr, 1..,
                        // if it is a string literal, then we know it is not empty
                        pub fn x(z: &str) -> $crate::tagged::Tag {
                            use ::std::convert::TryFrom;
                            $crate::tagged::Tag::try_from(z).unwrap()
                        },
                        // if it is not a string literal, require an infallible conversion
                        pub fn x(z: impl Into<$crate::tagged::Tag>) -> $crate::tagged::Tag {
                            z.into()
                        }
                    }
                }
                tags.push(y::x($expr));
            }
        )*
        $crate::tagged::TagSet::from(tags)
    }};
    ($($x:tt)*) => {
        compile_error!("This macro supports only string literals or expressions in parens.")
    }
}

mk_scalar!(
    /// Arbitrary metadata-string for an Event. Website documentation pending.
    ///
    /// You may most conveniently construct values of this type with the [`tag!`](../macro.tag.html) macro.
    struct Tag, EmptyTag
);

impl From<&Semantics> for Tag {
    fn from(value: &Semantics) -> Self {
        Tag::new(format!("semantics:{}", value.as_str())).unwrap()
    }
}

impl From<Semantics> for Tag {
    fn from(value: Semantics) -> Self {
        Tag::from(&value)
    }
}

impl From<&FishName> for Tag {
    fn from(value: &FishName) -> Self {
        Tag::new(format!("fish_name:{}", value.as_str())).unwrap()
    }
}

impl From<FishName> for Tag {
    fn from(value: FishName) -> Self {
        Tag::from(&value)
    }
}

/// Concatenate another part to this tag
///
/// ```
/// # use actyxos_sdk::{tag, tagged::Tag};
/// let user_tag = tag!("user:") + "Bob";
/// let machine_tag = tag!("machine:") + format!("{}-{}", "thing", 42);
///
/// assert_eq!(user_tag, tag!("user:Bob"));
/// assert_eq!(machine_tag, tag!("machine:thing-42"));
/// ```
///
/// This will never panic because the initial tag is already proven to be a valid tag.
impl<T: Into<String>> Add<T> for Tag {
    type Output = Tag;
    fn add(self, rhs: T) -> Self::Output {
        Tag::new(self.0.to_string() + rhs.into().as_str()).unwrap()
    }
}

/// A set of tags in canonical iteration order
///
/// All constructors and serialization ensure that tags appear only once and in string sort order.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(from = "Vec<Tag>")]
pub struct TagSet(Vec<Tag>);

impl From<Vec<Tag>> for TagSet {
    fn from(mut v: Vec<Tag>) -> Self {
        v.sort_unstable();
        v.dedup();
        Self(v)
    }
}

impl From<&[Tag]> for TagSet {
    fn from(v: &[Tag]) -> Self {
        let v = Vec::from(v);
        Self::from(v)
    }
}

impl From<BTreeSet<Tag>> for TagSet {
    fn from(v: BTreeSet<Tag>) -> Self {
        Self(v.into_iter().collect())
    }
}

impl From<&BTreeSet<Tag>> for TagSet {
    fn from(v: &BTreeSet<Tag>) -> Self {
        Self(v.iter().cloned().collect())
    }
}

impl Default for TagSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl TagSet {
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn insert(&mut self, tag: Tag) {
        if let Err(idx) = self.0.binary_search(&tag) {
            self.0.insert(idx, tag);
        }
    }

    pub fn remove(&mut self, tag: &Tag) {
        if let Ok(idx) = self.0.binary_search(tag) {
            self.0.remove(idx);
        }
    }

    pub fn contains(&self, tag: &Tag) -> bool {
        self.0.binary_search(tag).is_ok()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = Tag> + 'a {
        self.0.iter().cloned()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Add for &TagSet {
    type Output = TagSet;
    fn add(self, rhs: Self) -> Self::Output {
        let mut v = Vec::with_capacity(self.len() + rhs.len());
        let mut left = self.iter();
        let mut right = rhs.iter();
        let mut ll = left.next();
        let mut rr = right.next();
        loop {
            match (ll, rr) {
                (Some(l), Some(r)) => match l.cmp(&r) {
                    std::cmp::Ordering::Less => {
                        v.push(l);
                        ll = left.next();
                        rr = Some(r);
                    }
                    std::cmp::Ordering::Equal => {
                        v.push(l);
                        ll = left.next();
                        rr = right.next();
                    }
                    std::cmp::Ordering::Greater => {
                        v.push(r);
                        ll = Some(l);
                        rr = right.next();
                    }
                },
                (Some(l), None) => {
                    v.push(l);
                    v.extend(left);
                    break;
                }
                (None, Some(r)) => {
                    v.push(r);
                    v.extend(right);
                    break;
                }
                _ => break,
            }
        }
        TagSet(v)
    }
}

impl Add for TagSet {
    type Output = TagSet;
    fn add(self, rhs: Self) -> Self::Output {
        (&self).add(&rhs)
    }
}

impl Add<Tag> for TagSet {
    type Output = TagSet;
    fn add(mut self, rhs: Tag) -> Self::Output {
        self.insert(rhs);
        self
    }
}

impl Add<Tag> for &TagSet {
    type Output = TagSet;
    fn add(self, rhs: Tag) -> Self::Output {
        let mut ret = self.clone();
        ret.insert(rhs);
        ret
    }
}

impl Add<&Tag> for TagSet {
    type Output = TagSet;
    fn add(mut self, rhs: &Tag) -> Self::Output {
        self.insert(rhs.clone());
        self
    }
}

impl Add<&Tag> for &TagSet {
    type Output = TagSet;
    fn add(self, rhs: &Tag) -> Self::Output {
        let mut ret = self.clone();
        ret.insert(rhs.clone());
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fish_name, semantics};

    #[test]
    fn semantics_to_tag() {
        let semantics = semantics!("test");

        assert_eq!("semantics:test", Tag::from(&semantics).as_str());
        assert_eq!("semantics:test", Tag::from(semantics).as_str());
    }

    #[test]
    fn fish_name_to_tag() {
        let fish_name = fish_name!("test");

        assert_eq!("fish_name:test", Tag::from(&fish_name).as_str());
        assert_eq!("fish_name:test", Tag::from(fish_name).as_str());
    }

    #[test]
    fn make_tags() {
        let mut tags = BTreeSet::new();
        tags.insert(tag!("a"));
        tags.insert(tag!("semantics:b"));
        assert_eq!(tags!("a", semantics!("b")), TagSet::from(tags));
    }

    #[test]
    fn tagset_is_set() {
        let a = tag!("a");
        let b = tag!("b");
        let c = tag!("c");

        assert_eq!(
            tags!("c", "b", "c", "a")
                .iter()
                .map(|t| t.as_str().to_owned())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
        assert_eq!(tags!("a", "b", "c", "b"), tags!("c", "b", "a"));
        assert_eq!(tags!("a", "b") + tags!("b", "c"), tags!("a", "b", "c"));
        assert_eq!(tags!("a", "c") + &b, tags!("a", "b", "c"));

        let mut t = TagSet::empty();
        t.insert(a.clone());
        assert!(t.contains(&a));
        assert!(!t.contains(&c));
        t.insert(b);
        t.insert(c);
        t.remove(&a);
        assert_eq!(t, tags!("c", "b"));
    }
}
