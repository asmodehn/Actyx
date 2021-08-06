use crate::{
    operation::AggrState,
    value::{Value, ValueKind},
};
use actyx_sdk::language::{BinOp, Ind, Index, Num, SimpleExpr, SortKey};
use anyhow::{anyhow, bail, ensure};
use cbor_data::{CborBuilder, CborOwned, Encoder, WithOutput, Writer};
use futures::{future::BoxFuture, FutureExt};
use std::{cmp::Ordering, collections::BTreeMap};

pub struct Context<'a> {
    sort_key: SortKey,
    bindings: BTreeMap<String, Value>,
    parent: Option<&'a Context<'a>>,
    aggregation: Option<&'a mut AggrState>,
}

impl<'a> Context<'a> {
    pub fn new(sort_key: SortKey) -> Self {
        Self {
            sort_key,
            bindings: BTreeMap::new(),
            parent: None,
            aggregation: None,
        }
    }

    pub fn child(&'a self) -> Self {
        Self {
            sort_key: self.sort_key,
            bindings: BTreeMap::new(),
            parent: Some(self),
            aggregation: None,
        }
    }

    #[cfg(test)]
    pub fn incr(&mut self) {
        self.sort_key.lamport = self.sort_key.lamport.incr();
    }

    pub fn value(&self, f: impl FnOnce(CborBuilder<WithOutput>) -> CborOwned) -> Value {
        Value::new(self.sort_key, f)
    }

    pub fn number(&self, n: &Num) -> Value {
        match n {
            Num::Decimal(d) => Value::new(self.sort_key, |b| b.encode_f64(*d)),
            Num::Natural(n) => Value::new(self.sort_key, |b| b.encode_u64(*n)),
        }
    }

    pub fn bind(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.insert(name.into(), value);
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.and_then(|c| c.lookup(name)))
    }

    pub fn bind_aggregation(&mut self, state: &'a mut AggrState) {
        self.aggregation = Some(state);
    }

    pub fn eval<'b>(&'b mut self, expr: &'b SimpleExpr) -> BoxFuture<'b, anyhow::Result<Value>> {
        async move {
            match expr {
                SimpleExpr::Variable(v) => {
                    let v = self.lookup(v).ok_or_else(|| anyhow!("variable '{}' is not bound", v))?;
                    Ok(self.value(|b| b.write_trusting(v.as_slice())))
                }
                SimpleExpr::Indexing(Ind { head, tail }) => {
                    let mut v = self.eval(head).await?;
                    for i in tail.iter() {
                        v = match i {
                            Index::String(s) => v.index(s)?,
                            Index::Number(n) => v.index(&format!("{}", n))?,
                            Index::Expr(e) => {
                                let idx = self.eval(e).await?;
                                match idx.kind() {
                                    ValueKind::Number => v.index(&format!("{}", idx.value()))?,
                                    ValueKind::String => v.index(idx.as_str()?)?,
                                    _ => bail!("cannot index by {}", idx.value()),
                                }
                            }
                        };
                    }
                    Ok(self.value(|b| b.write_trusting(v.as_slice())))
                }
                SimpleExpr::Number(n) => match n {
                    Num::Decimal(d) => Ok(self.value(|b| b.encode_f64(*d))),
                    Num::Natural(n) => Ok(self.value(|b| b.encode_u64(*n))),
                },
                SimpleExpr::String(s) => Ok(self.value(|b| b.encode_str(s))),
                SimpleExpr::Object(a) => {
                    let mut v = BTreeMap::new();
                    for (n, e) in a.props.iter() {
                        let key = match n {
                            Index::String(s) => s.clone(),
                            Index::Number(n) => n.to_string(),
                            Index::Expr(e) => {
                                let k = self.eval(e).await?;
                                k.as_str()
                                    .map(|s| s.to_owned())
                                    .or_else(|_| k.as_number().map(|n| n.to_string()))?
                            }
                        };
                        v.insert(key, self.eval(e).await?);
                    }
                    Ok(self.value(|b| {
                        b.encode_dict(|b| {
                            for (name, item) in v.iter() {
                                b.with_key(name, |b| b.write_trusting(item.as_slice()));
                            }
                        })
                    }))
                }
                SimpleExpr::Array(a) => {
                    let mut v = vec![];
                    for e in a.items.iter() {
                        v.push(self.eval(e).await?)
                    }
                    Ok(self.value(|b| {
                        b.encode_array(|b| {
                            for item in v.iter() {
                                b.write_trusting(item.as_slice());
                            }
                        })
                    }))
                }
                SimpleExpr::Null => Ok(self.value(|b| b.write_null(None))),
                SimpleExpr::Bool(f) => Ok(self.value(|b| b.write_bool(*f, None))),
                SimpleExpr::Cases(v) => {
                    for (pred, expr) in v.iter() {
                        let pred = self.eval(pred).await.and_then(|v| v.as_bool());
                        if pred.unwrap_or(false) {
                            return self.eval(expr).await;
                        }
                    }
                    Err(anyhow!("no case matched"))
                }
                SimpleExpr::Not(a) => {
                    let v = !self.eval(a).await?.as_bool()?;
                    Ok(self.value(|b| b.encode_bool(v)))
                }
                SimpleExpr::BinOp(b) => self.bin_op(&b.1, b.0, &b.2).await,
                SimpleExpr::AggrOp(a) => {
                    // AggrOp can only occur in the expression of an AGGREGATE clause, which will only be
                    // evaluated within Aggregate::flush(), meaning that the context will have a parent
                    // and a bound aggregation state
                    let idx = self
                        .aggregation
                        .as_ref()
                        .ok_or_else(|| anyhow!("no aggregation state"))?
                        .binary_search_by_key(&&**a, |x| &x.0)
                        .map_err(|_| anyhow!("no aggregation result for {}({})", a.0.as_str(), a.1))?;
                    // evaluate in the parent context as there is no additional value inside this child
                    // context, and we already have self.aggregation borrowed
                    self.aggregation.as_mut().unwrap()[idx].1.flush(self.parent.unwrap())
                }
                SimpleExpr::FuncCall(f) => match f.name.as_str() {
                    "IsDefined" => {
                        ensure!(
                            f.args.len() == 1,
                            "wrong number of arguments: 'IsDefined' takes 1 argument but {} were provided",
                            f.args.len()
                        );
                        let defined = self.eval(&f.args[0]).await.is_ok();
                        Ok(self.value(|b| b.encode_bool(defined)))
                    }
                    _ => Err(anyhow!("undefined function '{}'", f.name)),
                },
            }
        }
        .boxed()
    }

    async fn bin_op<'b>(&'b mut self, l: &'b SimpleExpr, op: BinOp, r: &'b SimpleExpr) -> anyhow::Result<Value> {
        match op {
            BinOp::Add => self.eval(l).await?.add(&self.eval(r).await?),
            BinOp::Sub => self.eval(l).await?.sub(&self.eval(r).await?),
            BinOp::Mul => self.eval(l).await?.mul(&self.eval(r).await?),
            BinOp::Div => self.eval(l).await?.div(&self.eval(r).await?),
            BinOp::Mod => self.eval(l).await?.modulo(&self.eval(r).await?),
            BinOp::Pow => self.eval(l).await?.pow(&self.eval(r).await?),
            BinOp::And => {
                let v = self.eval(l).await?.as_bool()? && self.eval(r).await?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Or => {
                let v = self.eval(l).await?.as_bool()? || self.eval(r).await?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Xor => {
                let v = self.eval(l).await?.as_bool()? ^ self.eval(r).await?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Lt => {
                let left = self.eval(l).await?;
                let right = self.eval(r).await?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Less)
                    .ok_or_else(|| anyhow!("cannot compare {} < {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Le => {
                let left = self.eval(l).await?;
                let right = self.eval(r).await?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Greater)
                    .ok_or_else(|| anyhow!("cannot compare {} ≤ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Gt => {
                let left = self.eval(l).await?;
                let right = self.eval(r).await?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Greater)
                    .ok_or_else(|| anyhow!("cannot compare {} > {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Ge => {
                let left = self.eval(l).await?;
                let right = self.eval(r).await?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Less)
                    .ok_or_else(|| anyhow!("cannot compare {} ≥ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Eq => {
                let left = self.eval(l).await?;
                let right = self.eval(r).await?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Equal)
                    .ok_or_else(|| anyhow!("cannot compare {} = {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Ne => {
                let left = self.eval(l).await?;
                let right = self.eval(r).await?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Equal)
                    .ok_or_else(|| anyhow!("cannot compare {} ≠ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Alt => match self.eval(l).await {
                Ok(v) => Ok(v),
                Err(_) => self.eval(r).await,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::NodeId;

    use super::*;
    use futures::executor::block_on;
    use quickcheck::{quickcheck, TestResult};
    use spectral::{assert_that, string::StrAssertions};

    async fn eval(cx: &mut Context<'_>, s: &str) -> anyhow::Result<String> {
        cx.eval(&s.parse()?).await.map(|x| x.value().to_string())
    }

    fn eval_bool(cx: &mut Context<'_>, s: &str) -> bool {
        block_on(eval(cx, s)).unwrap().parse::<bool>().unwrap()
    }

    fn ctx() -> Context<'static> {
        Context::new(SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
        })
    }

    #[tokio::test]
    async fn simple() {
        let mut cx = ctx();
        cx.bind(
            "x",
            Value::new(cx.sort_key, |b| {
                b.encode_dict(|b| {
                    b.with_key("y", |b| b.encode_u64(42));
                })
            }),
        );

        assert_eq!(eval(&mut cx, "5+2.1+x.y").await.unwrap(), "49.1");

        assert_eq!(eval(&mut cx, "x").await.unwrap(), "{\"y\": 42}");

        let err = eval(&mut cx, "5+x").await.unwrap_err().to_string();
        assert!(err.contains("{\"y\": 42} is not a number"), "didn’t match: {}", err);

        let err = eval(&mut cx, "y").await.unwrap_err().to_string();
        assert!(err.contains("variable 'y' is not bound"), "didn’t match: {}", err);

        let err = eval(&mut cx, "x.a").await.unwrap_err().to_string();
        assert!(
            err.contains("path .a does not exist in value {\"y\": 42}"),
            "didn’t match: {}",
            err
        );
    }

    #[tokio::test]
    async fn primitives() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "NULL").await.unwrap(), "null");
        assert_eq!(eval(&mut cx, "TRUE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "1.23").await.unwrap(), "1.23");
        assert_eq!(
            eval(&mut cx, "12345678901234567890").await.unwrap(),
            "12345678901234567890"
        );
        assert_eq!(eval(&mut cx, "''").await.unwrap(), "\"\"");
        assert_eq!(eval(&mut cx, "\"\"").await.unwrap(), "\"\"");
        assert_eq!(eval(&mut cx, "'hello'").await.unwrap(), "\"hello\"");
        assert_eq!(eval(&mut cx, "\"hello\"").await.unwrap(), "\"hello\"");
        assert_eq!(eval(&mut cx, r#"'h"ell''o'"#).await.unwrap(), r#""h\"ell\'o""#);
        assert_eq!(eval(&mut cx, r#""h""ell'o""#).await.unwrap(), r#""h\"ell\'o""#);
    }

    #[tokio::test]
    async fn boolean() {
        let mut cx = ctx();

        assert_eq!(eval(&mut cx, "FALSE ∧ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ∧ TRUE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE ∧ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE ∧ TRUE").await.unwrap(), "true");

        assert_eq!(eval(&mut cx, "FALSE ∨ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ∨ TRUE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ∨ FALSE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ∨ TRUE").await.unwrap(), "true");

        assert_eq!(eval(&mut cx, "FALSE ⊻ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ⊻ TRUE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ⊻ FALSE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ⊻ TRUE").await.unwrap(), "false");

        assert_eq!(eval(&mut cx, "!FALSE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "¬TRUE").await.unwrap(), "false");

        // check short-circuit behaviour
        assert_eq!(eval(&mut cx, "FALSE & 12").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE | 12").await.unwrap(), "true");

        assert_that(&eval(&mut cx, "NULL & x").await.unwrap_err().to_string()).contains("null is not a bool");
        assert_that(&eval(&mut cx, "FALSE | 12").await.unwrap_err().to_string()).contains("12 is not a bool");
        assert_that(&eval(&mut cx, "!'a'").await.unwrap_err().to_string()).contains("\"a\" is not a bool");
    }

    #[tokio::test]
    async fn compare() {
        let mut cx = ctx();

        assert_eq!(
            eval(&mut cx, "NULL = NULL ∧ NULL ≥ NULL ∧ NULL ≤ NULL").await.unwrap(),
            "true"
        );
        assert_eq!(
            eval(&mut cx, "NULL ≠ NULL ∨ NULL > NULL ∨ NULL < NULL").await.unwrap(),
            "false"
        );

        #[allow(clippy::bool_comparison)]
        fn prop_bool(left: bool, right: bool) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_bool(left)));
            cx.bind("b", cx.value(|b| b.encode_bool(right)));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_bool as fn(bool, bool) -> bool);

        fn prop_u64(left: u64, right: u64) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_u64(left)));
            cx.bind("b", cx.value(|b| b.encode_u64(right)));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_u64 as fn(u64, u64) -> bool);

        #[allow(clippy::float_cmp)]
        fn prop_f64(left: f64, right: f64) -> TestResult {
            if left.is_nan() || right.is_nan() {
                return TestResult::discard();
            }
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_f64(left)));
            cx.bind("b", cx.value(|b| b.encode_f64(right)));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            TestResult::passed()
        }
        quickcheck(prop_f64 as fn(f64, f64) -> TestResult);

        fn prop_str(left: String, right: String) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_str(left.as_str())));
            cx.bind("b", cx.value(|b| b.encode_str(right.as_str())));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_str as fn(String, String) -> bool);

        assert_that(&eval(&mut cx, "NULL > 12").await.unwrap_err().to_string()).contains("cannot compare");
    }

    #[tokio::test]
    async fn constructors() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[0]").await.unwrap(), "1");
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[1]").await.unwrap(), "\"x\"");
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[2]").await.unwrap(), "null");
        assert_eq!(
            eval(&mut cx, "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE }).one")
                .await
                .unwrap(),
            "1"
        );
        assert_eq!(
            eval(&mut cx, "({ one: 1 ['two']: 'x' [('three')]: NULL, [4]: TRUE }).two")
                .await
                .unwrap(),
            "\"x\""
        );
        assert_eq!(
            eval(
                &mut cx,
                "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE }).three"
            )
            .await
            .unwrap(),
            "null"
        );
        assert_eq!(
            eval(&mut cx, "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE })[4]")
                .await
                .unwrap(),
            "true"
        );

        assert_that(&eval(&mut cx, "{'x':1}").await.unwrap_err().to_string()).contains("expected ident");
    }

    #[tokio::test]
    async fn arithmetic() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "1+2").await.unwrap(), "3");
        assert_eq!(eval(&mut cx, "1+2*3^2%5").await.unwrap(), "4");
        assert_eq!(eval(&mut cx, "1.0+2.0*3.0^2.0%5.0").await.unwrap(), "4.0");

        assert_that(
            &eval(&mut cx, "12345678901234567890 + 12345678901234567890")
                .await
                .unwrap_err()
                .to_string(),
        )
        .contains("integer overflow");
        assert_that(&eval(&mut cx, "10.0 ^ 400").await.unwrap_err().to_string()).contains("floating-point overflow");
        assert_that(&eval(&mut cx, "10.0 / 0").await.unwrap_err().to_string()).contains("floating-point overflow");
        assert_that(&eval(&mut cx, "0.0 / 0").await.unwrap_err().to_string()).contains("not a number");
    }

    #[tokio::test]
    async fn indexing() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "([42])[0]").await.unwrap(), "42");
        assert_eq!(eval(&mut cx, "([42])[1-1]").await.unwrap(), "42");
        assert_eq!(eval(&mut cx, "({x:12}).x").await.unwrap(), "12");
        assert_eq!(eval(&mut cx, "({x:12})['x']").await.unwrap(), "12");
        assert_eq!(eval(&mut cx, "({x:12})[('x')]").await.unwrap(), "12");
    }

    #[tokio::test]
    async fn cases() {
        let mut cx = ctx();
        assert_eq!(
            eval(&mut cx, "CASE 5 ≤ 5 => 42 CASE TRUE => NULL ENDCASE")
                .await
                .unwrap(),
            "42"
        );
        assert_eq!(
            eval(&mut cx, "CASE 5 < 5 => 42 CASE TRUE => NULL ENDCASE")
                .await
                .unwrap(),
            "null"
        );
        assert_eq!(
            eval(&mut cx, "CASE 'a' => 'b' CASE TRUE => 'c' ENDCASE").await.unwrap(),
            "\"c\""
        );
        assert_eq!(
            eval(&mut cx, "CASE a => 'b' CASE TRUE => 'c' ENDCASE").await.unwrap(),
            "\"c\""
        );

        assert_that(&eval(&mut cx, "CASE FALSE => 1 ENDCASE").await.unwrap_err().to_string())
            .contains("no case matched");
    }

    #[tokio::test]
    async fn alternative() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "5 // 6").await.unwrap(), "5");
        assert_eq!(eval(&mut cx, "(5).a // 6").await.unwrap(), "6");
    }

    #[tokio::test]
    async fn builtin_functions() {
        let mut cx = ctx();

        assert_eq!(eval(&mut cx, "IsDefined(1)").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "IsDefined(1 + '')").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "IsDefined(1 + '' // FALSE)").await.unwrap(), "true");
        assert_that(&eval(&mut cx, "IsDefined()").await.unwrap_err().to_string()).contains("wrong number of arguments");
        assert_that(&eval(&mut cx, "IsDefined(1, 2)").await.unwrap_err().to_string())
            .contains("wrong number of arguments");
    }
}
