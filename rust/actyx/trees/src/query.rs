use std::{
    cmp::Ord,
    collections::BTreeSet,
    iter::FromIterator,
    ops::{Range, RangeFrom, RangeTo},
};

use actyxos_sdk::{language, LamportTimestamp, TagSet, Timestamp};
use banyan::{
    index::{BranchIndex, CompactSeq, LeafIndex},
    query::Query,
};
use maplit::btreeset;
use range_collections::RangeSet;

use crate::{
    axtrees::{AxTrees, TagsSummaries},
    dnf::Dnf,
    tag_index::IndexSet,
    TagIndex,
};

#[derive(Debug, Clone)]
pub struct LamportQuery(RangeSet<LamportTimestamp>);

impl From<Range<LamportTimestamp>> for LamportQuery {
    fn from(value: Range<LamportTimestamp>) -> Self {
        Self(value.into())
    }
}

impl Query<AxTrees> for LamportQuery {
    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        let lamport = &index.summaries.lamport;
        for i in 0..lamport.len().min(matching.len()) {
            matching[i] = matching[i] && !self.0.is_disjoint(&lamport[i].clone().into());
        }
    }

    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        let lamport = &index.keys.lamport;
        for i in 0..lamport.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&lamport[i]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeQuery(RangeSet<Timestamp>);

impl From<Range<Timestamp>> for TimeQuery {
    fn from(value: Range<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl From<RangeSet<Timestamp>> for TimeQuery {
    fn from(value: RangeSet<Timestamp>) -> Self {
        Self(value)
    }
}

impl From<RangeFrom<Timestamp>> for TimeQuery {
    fn from(value: RangeFrom<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl From<RangeTo<Timestamp>> for TimeQuery {
    fn from(value: RangeTo<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl Query<AxTrees> for TimeQuery {
    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        let time = &index.summaries.time;
        for i in 0..time.len().min(matching.len()) {
            matching[i] = matching[i] && !self.0.is_disjoint(&time[i].clone().into());
        }
    }

    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        let time = &index.keys.time;
        for i in 0..time.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&time[i]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct OffsetQuery(RangeSet<u64>);

impl From<Range<u64>> for OffsetQuery {
    fn from(value: Range<u64>) -> Self {
        Self(value.into())
    }
}
impl From<RangeSet<u64>> for OffsetQuery {
    fn from(value: RangeSet<u64>) -> Self {
        Self(value)
    }
}

impl From<RangeFrom<u64>> for OffsetQuery {
    fn from(value: RangeFrom<u64>) -> Self {
        Self(value.into())
    }
}

impl From<RangeTo<u64>> for OffsetQuery {
    fn from(value: RangeTo<u64>) -> Self {
        Self(value.into())
    }
}

impl Query<AxTrees> for OffsetQuery {
    fn intersecting(&self, offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        let range = offset..offset + index.count;
        if self.0.is_disjoint(&range.into()) {
            for e in matching.iter_mut() {
                *e = false;
            }
        }
    }

    fn containing(&self, mut offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        for i in 0..index.keys.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&offset);
            offset += 1;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagsQuery(Vec<TagSet>);

impl TagsQuery {
    pub fn new(dnf: Vec<TagSet>) -> Self {
        Self(dnf)
    }

    pub fn from_expr(tag_expr: &language::TagExpr, local: bool) -> Option<Self> {
        let dnf = Dnf::from(tag_expr);

        if dnf
            .0
            .iter()
            .any(|atoms| atoms == &btreeset!(language::TagAtom::AllEvents))
        {
            return Some(TagsQuery::all());
        }

        if !local && dnf.0.iter().all(|atoms| atoms.contains(&language::TagAtom::IsLocal)) {
            return None;
        }

        let tag_set = |atoms: BTreeSet<language::TagAtom>| {
            if atoms == btreeset!(language::TagAtom::AllEvents) {
                return Some(TagSet::empty());
            }

            let mut tags = TagSet::empty();
            let mut is_local = false;
            for atom in atoms {
                match atom {
                    language::TagAtom::Tag(tag) => tags.insert(tag),
                    language::TagAtom::AllEvents => {}
                    language::TagAtom::IsLocal => is_local = true,
                    language::TagAtom::FromTime(_) => {}
                    language::TagAtom::ToTime(_) => {}
                    language::TagAtom::FromLamport(_) => {}
                    language::TagAtom::ToLamport(_) => {}
                    language::TagAtom::AppId(_) => {}
                }
            }
            if !is_local || local {
                Some(tags)
            } else {
                None
            }
        };

        let res: TagsQuery = dnf.0.into_iter().filter_map(tag_set).collect();
        Some(if res.tags().contains(&TagSet::empty()) {
            Self::all()
        } else {
            res
        })
    }

    pub fn all() -> Self {
        Self(vec![TagSet::empty()])
    }
    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn tags(&self) -> &[TagSet] {
        self.0.as_slice()
    }

    fn set_matching(&self, index: &TagIndex, matching: &mut [bool]) {
        // lookup all strings and translate them into indices.
        // if a single index does not match, the query can not match at all.
        let lookup = |s: &TagSet| -> Option<IndexSet> {
            s.iter()
                .map(|t| index.tags.find(&t).map(|x| x as u32))
                .collect::<Option<_>>()
        };
        // translate the query from strings to indices
        let query = self.0.iter().filter_map(lookup).collect::<Vec<_>>();
        // only look at bits that are currently set, set them to false if they do not match
        for i in 0..index.elements.len().min(matching.len()) {
            if matching[i] {
                matching[i] = query.iter().any(|x| x.is_subset(&index.elements[i]));
            }
        }
    }
}

impl Query<AxTrees> for TagsQuery {
    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        self.set_matching(&index.keys.tags, matching);
    }

    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        if let TagsSummaries::Complete(index) = &index.summaries.tags {
            self.set_matching(index, matching);
        }
    }
}

impl FromIterator<TagSet> for TagsQuery {
    fn from_iter<T: IntoIterator<Item = TagSet>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::{
        language::{TagAtom, TagExpr},
        tags, Tag,
    };

    fn l(tag: &'static str) -> TagExpr {
        TagExpr::Atom(TagAtom::Tag(Tag::new(tag.to_owned()).unwrap()))
    }

    fn assert_match(index: &TagIndex, expr: &TagExpr, expected: Vec<bool>) {
        let query = TagsQuery::from_expr(expr, true).unwrap();
        let mut matching = vec![true; expected.len()];
        query.set_matching(index, &mut matching);
        assert_eq!(matching, expected);
    }

    #[test]
    fn test_matching_1() {
        let index = TagIndex::from_elements(&[tags!("a"), tags!("a", "b"), tags!("a"), tags!("a", "b")]);
        let expr = l("a") | l("b");
        assert_match(&index, &expr, vec![true, true, true, true]);
        let expr = l("a") & l("b");
        assert_match(&index, &expr, vec![false, true, false, true]);
        let expr = l("c") & l("d");
        assert_match(&index, &expr, vec![false, false, false, false]);
    }

    #[test]
    fn test_matching_2() {
        let index = TagIndex::from_elements(&[tags!("a", "b"), tags!("b", "c"), tags!("c", "a"), tags!("a", "b")]);
        let expr = l("a") | l("b") | l("c");
        assert_match(&index, &expr, vec![true, true, true, true]);
        let expr = l("a") & l("b");
        assert_match(&index, &expr, vec![true, false, false, true]);
        let expr = l("a") & l("b") & l("c");
        assert_match(&index, &expr, vec![false, false, false, false]);
    }

    #[test]
    fn test_from_expr() {
        let test_expr = |local: bool, tag_expr: &'static str, expected: Option<Vec<TagSet>>| {
            let tag_expr = tag_expr.parse::<TagExpr>().unwrap();
            let actual = TagsQuery::from_expr(&tag_expr, local).map(|q| q.tags().to_vec());
            assert_eq!(actual, expected, "tag_expr: {:?}", tag_expr);
        };

        test_expr(true, "allEvents", Some(vec![tags!()]));
        test_expr(true, "allEvents | 'a'", Some(vec![tags!()]));
        test_expr(true, "isLocal", Some(vec![tags!()]));
        test_expr(true, "isLocal & 'a'", Some(vec![tags!("a")]));
        test_expr(true, "isLocal | 'a'", Some(vec![tags!()]));
        test_expr(true, "isLocal & 'b' | 'a'", Some(vec![tags!("a"), tags!("b")]));
        test_expr(true, "'a'", Some(vec![tags!("a")]));

        test_expr(false, "allEvents", Some(vec![tags!()]));
        test_expr(false, "allEvents | 'a'", Some(vec![tags!()]));
        test_expr(false, "isLocal", None);
        test_expr(false, "isLocal & 'a'", None);
        test_expr(false, "isLocal | 'a'", Some(vec![tags!("a")]));
        test_expr(false, "isLocal & 'b' | 'a'", Some(vec![tags!("a")]));
        test_expr(false, "'a'", Some(vec![tags!("a")]));
    }
}
