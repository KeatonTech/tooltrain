use std::sync::Arc;

use crate::Value;
use anyhow::Error;
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub enum ValueChange {
    Set(Arc<Value>),
    Destroy,
}

#[derive(Debug)]
pub struct ValueStream {
    value: Option<Arc<Value>>,
    updates: broadcast::Sender<ValueChange>,
}

impl ValueStream {
    pub(crate) fn new(initial: Option<Value>) -> Self {
        let (updates, _) = broadcast::channel::<ValueChange>(128);
        ValueStream {
            value: initial.map(Arc::new),
            updates,
        }
    }

    pub fn snapshot(&self) -> Option<Arc<Value>> {
        self.value.clone()
    }

    pub(crate) fn set(&mut self, value: Value) -> Result<(), Error> {
        let value_arc = Arc::new(value);
        self.value = Some(value_arc.clone());
        let _ = self.updates.send(ValueChange::Set(value_arc));
        Ok(())
    }

    pub(crate) fn destroy(&mut self) -> Result<(), Error> {
        self.value = None;
        let _ = self.updates.send(ValueChange::Destroy);
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ValueChange> {
        self.updates.subscribe()
    }
}
