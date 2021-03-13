use crate::{
    tagged::{AppId, Tag},
    LamportTimestamp, TimeStamp,
};

mod parser;
pub use parser::expression;

#[derive(Debug, PartialEq)]
pub enum Expression {
    Simple(SimpleExpr),
    Query(Query),
}

#[derive(Debug, PartialEq)]
pub struct Query {
    from: TagExpr,
    ops: Vec<Operation>,
}

#[derive(Debug, PartialEq)]
pub enum Operation {
    Filter(SimpleExpr),
    Select(SimpleExpr),
}

#[derive(Debug, PartialEq)]
pub enum TagExpr {
    Or(Box<(TagExpr, TagExpr)>),
    And(Box<(TagExpr, TagExpr)>),
    Tag(Tag),
    AllEvents,
    IsLocal,
    FromTime(TimeStamp),
    ToTime(TimeStamp),
    FromLamport(LamportTimestamp),
    ToLamport(LamportTimestamp),
    AppId(AppId),
}

impl TagExpr {
    pub fn and(self, other: TagExpr) -> Self {
        TagExpr::And(Box::new((self, other)))
    }
    pub fn or(self, other: TagExpr) -> Self {
        TagExpr::Or(Box::new((self, other)))
    }
}

// this will obviously need to be implemented for real sometime, with arbitrary precision
#[derive(Debug, PartialEq, Clone)]
pub enum Number {
    Decimal(f64),
    Natural(u64),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Index {
    Ident(String),
    Number(u64),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Path {
    head: String,
    tail: Vec<Index>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Object {
    props: Vec<(String, SimpleExpr)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Array {
    items: Vec<SimpleExpr>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SimpleExpr {
    Path(Path),
    Number(Number),
    String(String),
    Object(Object),
    Array(Array),
    Add(Box<(SimpleExpr, SimpleExpr)>),
    Sub(Box<(SimpleExpr, SimpleExpr)>),
    Mul(Box<(SimpleExpr, SimpleExpr)>),
    Div(Box<(SimpleExpr, SimpleExpr)>),
    Mod(Box<(SimpleExpr, SimpleExpr)>),
    Pow(Box<(SimpleExpr, SimpleExpr)>),
    And(Box<(SimpleExpr, SimpleExpr)>),
    Or(Box<(SimpleExpr, SimpleExpr)>),
    Not(Box<SimpleExpr>),
    Xor(Box<(SimpleExpr, SimpleExpr)>),
    Lt(Box<(SimpleExpr, SimpleExpr)>),
    Le(Box<(SimpleExpr, SimpleExpr)>),
    Gt(Box<(SimpleExpr, SimpleExpr)>),
    Ge(Box<(SimpleExpr, SimpleExpr)>),
    Eq(Box<(SimpleExpr, SimpleExpr)>),
    Ne(Box<(SimpleExpr, SimpleExpr)>),
}

#[allow(clippy::clippy::should_implement_trait)]
impl SimpleExpr {
    pub fn add(self, other: SimpleExpr) -> Self {
        SimpleExpr::Add(Box::new((self, other)))
    }
    pub fn sub(self, other: SimpleExpr) -> Self {
        SimpleExpr::Sub(Box::new((self, other)))
    }
    pub fn mul(self, other: SimpleExpr) -> Self {
        SimpleExpr::Mul(Box::new((self, other)))
    }
    pub fn div(self, other: SimpleExpr) -> Self {
        SimpleExpr::Div(Box::new((self, other)))
    }
    pub fn modulo(self, other: SimpleExpr) -> Self {
        SimpleExpr::Mod(Box::new((self, other)))
    }
    pub fn pow(self, other: SimpleExpr) -> Self {
        SimpleExpr::Pow(Box::new((self, other)))
    }
    pub fn and(self, other: SimpleExpr) -> Self {
        SimpleExpr::And(Box::new((self, other)))
    }
    pub fn or(self, other: SimpleExpr) -> Self {
        SimpleExpr::Or(Box::new((self, other)))
    }
    pub fn xor(self, other: SimpleExpr) -> Self {
        SimpleExpr::Xor(Box::new((self, other)))
    }
    pub fn lt(self, other: SimpleExpr) -> Self {
        SimpleExpr::Lt(Box::new((self, other)))
    }
    pub fn le(self, other: SimpleExpr) -> Self {
        SimpleExpr::Le(Box::new((self, other)))
    }
    pub fn gt(self, other: SimpleExpr) -> Self {
        SimpleExpr::Gt(Box::new((self, other)))
    }
    pub fn ge(self, other: SimpleExpr) -> Self {
        SimpleExpr::Ge(Box::new((self, other)))
    }
    pub fn eq(self, other: SimpleExpr) -> Self {
        SimpleExpr::Eq(Box::new((self, other)))
    }
    pub fn ne(self, other: SimpleExpr) -> Self {
        SimpleExpr::Ne(Box::new((self, other)))
    }
}

#[cfg(test)]
mod for_tests {
    use super::*;

    impl Query {
        pub fn new(from: TagExpr) -> Self {
            Self { from, ops: vec![] }
        }
        pub fn push(&mut self, op: Operation) {
            self.ops.push(op);
        }
        pub fn with_op(self, op: Operation) -> Self {
            let Self { from, mut ops } = self;
            ops.push(op);
            Self { from, ops }
        }
    }

    pub trait ToIndex {
        fn into(&self) -> Index;
    }
    impl ToIndex for &str {
        fn into(&self) -> Index {
            Index::Ident((*self).to_owned())
        }
    }
    impl ToIndex for u64 {
        fn into(&self) -> Index {
            Index::Number(*self)
        }
    }

    impl Path {
        pub fn with(head: impl Into<String>, tail: &[&dyn ToIndex]) -> SimpleExpr {
            SimpleExpr::Path(Self {
                head: head.into(),
                tail: tail.iter().map(|x| (*x).into()).collect(),
            })
        }
        pub fn ident(ident: impl Into<String>) -> SimpleExpr {
            SimpleExpr::Path(Self {
                head: ident.into(),
                tail: vec![],
            })
        }
    }

    impl Object {
        pub fn with(props: &[(&str, SimpleExpr)]) -> SimpleExpr {
            SimpleExpr::Object(Object {
                props: props.iter().map(|(x, e)| ((*x).to_owned(), e.clone())).collect(),
            })
        }
    }

    impl Array {
        pub fn with(items: &[SimpleExpr]) -> SimpleExpr {
            SimpleExpr::Array(Array { items: items.to_vec() })
        }
    }
}
