use crate::{
    eval::Context,
    operation::{Operation, Processor},
    value::Value,
};
use actyx_sdk::language;
use swarm::event_store_ref::EventStoreRef;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Query {
    pub features: Vec<String>,
    pub from: language::TagExpr,
    pub stages: Vec<Operation>,
}

impl Query {
    /// run a query in the given evaluation context and collect all results
    pub async fn eval(query: &language::Query, cx: &Context<'_>) -> Result<Value, anyhow::Error> {
        todo!()
    }

    pub fn make_feeder(&self, store: EventStoreRef) -> Feeder {
        let processors = self.stages.iter().map(|op| op.make_processor()).collect();
        Feeder { processors, store }
    }

    pub fn feeder_from(stages: &[language::Operation], store: EventStoreRef) -> Feeder {
        let processors = stages
            .iter()
            .map(|s| Operation::from(s.clone()).make_processor())
            .collect();
        Feeder { processors, store }
    }
}

pub struct Feeder {
    processors: Vec<Box<dyn Processor>>,
    store: EventStoreRef,
}
impl Feeder {
    pub async fn feed(&mut self, input: Option<Value>) -> Vec<Result<Value, anyhow::Error>> {
        // create storage space for one context per stage
        let mut ctx = Vec::<Option<Context<'_>>>::new();
        ctx.resize_with(self.processors.len(), || None);
        let mut ctx = &mut ctx[..];

        // create the outermost context, stored on the stack
        let mut cx = Context::new(input.as_ref().map(|x| x.key()).unwrap_or_default(), &self.store);

        // set up per-iteration state
        let mut cx = &mut cx; // reference to the current context
        let mut input = vec![Ok(input).transpose()]; // inputs to be delivered to the current stage

        for op in self.processors.iter_mut() {
            // create fresh child context, stored in the ctx slice
            let (curr_ctx, rest) = ctx.split_first_mut().unwrap();
            ctx = rest;
            *curr_ctx = Some(cx.child());
            cx = curr_ctx.as_mut().unwrap();
            // then feed all inputs
            let mut output = vec![];
            for input in input {
                match input {
                    Some(Ok(v)) => {
                        cx.bind("_", v);
                        output.extend(op.apply(cx).await.into_iter().map(Some));
                    }
                    None => {
                        output.extend(op.flush(cx).await.into_iter().map(Some));
                        output.push(None);
                    }
                    Some(Err(e)) => output.push(Some(Err(e))),
                }
            }
            input = output;
            if input.is_empty() {
                break;
            }
        }

        input.into_iter().flatten().collect()
    }
}

impl From<language::Query> for Query {
    fn from(q: language::Query) -> Self {
        let stages = q.ops.into_iter().map(Operation::from).collect();
        let from = q.from;
        let features = q.features;
        Self { features, from, stages }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::{language::SortKey, NodeId};

    fn key() -> SortKey {
        SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
        }
    }
    fn store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
    }

    async fn feed(q: &str, v: &str) -> Vec<String> {
        let q = Query::from(q.parse::<language::Query>().unwrap());
        let v = Context::new(key(), &store()).eval(&v.parse().unwrap()).await.unwrap();
        q.make_feeder(store())
            .feed(Some(v))
            .await
            .into_iter()
            .map(|v| v.map(|v| v.value().to_string()).unwrap_or_else(|e| e.to_string()))
            .collect()
    }

    #[tokio::test]
    async fn query() {
        assert_eq!(
            feed("FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2", "3").await,
            Vec::<String>::new()
        );
        assert_eq!(
            feed("FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2", "2").await,
            vec!["4"]
        );
    }

    #[tokio::test]
    async fn select_multi() {
        assert_eq!(feed("FROM allEvents SELECT _, _ * 1.5", "42").await, vec!["42", "63.0"]);
        assert_eq!(
            feed(
                "FROM allEvents SELECT _.x, _.y, _.z FILTER _ = 'a' SELECT _, 42",
                "{x:'a' y:'b'}"
            )
            .await,
            vec!["\"a\"", "42", r#"path .z does not exist in value {"x": "a", "y": "b"}"#]
        );
    }
}
