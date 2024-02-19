use std::collections::BTreeMap;

use crate::bindings::{commander::base::streams::ValueEvent, DataType, Value};
use anyhow::{anyhow, Error};
use tokio::sync::broadcast::{channel, Sender};

pub(crate) type OutputId = u32;

#[derive(Clone, Debug)]
pub struct OutputSpec {
    pub name: String,
    pub description: String,
    pub data_type: DataType,
    pub value: Option<Value>,
    pub updates: Sender<ValueEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum SpecChange {
    Added(OutputId),
    Updated(OutputId),
    Removed(OutputId),
}

pub(crate) struct Outputs {
    pub(crate) state: BTreeMap<OutputId, OutputSpec>,
    pub(crate) updates: Sender<SpecChange>,
}

impl Default for Outputs {
    fn default() -> Self {
        let (updates, _) = channel(128);
        Outputs {
            state: BTreeMap::new(),
            updates,
        }
    }
}

impl Outputs {
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_output(
        &mut self,
        name: String,
        description: String,
        data_type: DataType,
        value: Option<Value>,
    ) -> OutputId {
        let next_index = self.state.last_key_value().map(|(k, _)| k + 1).unwrap_or(0);
        let (tx, _rx) = channel(1);
        self.state.insert(
            next_index,
            OutputSpec {
                name,
                description,
                data_type,
                value,
                updates: tx,
            },
        );
        let _ = self.updates.send(SpecChange::Added(next_index));
        next_index
    }

    pub(crate) fn remove_output(&mut self, id: OutputId) {
        self.state.remove(&id);
        let _ = self.updates.send(SpecChange::Removed(id));
    }

    pub(crate) fn update_output(&mut self, id: OutputId, event: ValueEvent) -> Result<(), Error> {
        let output = self.state.get_mut(&id).unwrap();
        match event.clone() {
            ValueEvent::Add(value) => match &mut output.value {
                Some(Value::TableValue(existing_table)) => {
                    if let Value::TableValue(added) = value {
                        existing_table.extend(added.into_iter());
                    } else {
                        return Err(anyhow!("Add must contain a TableValue"));
                    }
                }
                None => {
                    output.value = Some(value);
                }
                _ => {
                    return Err(anyhow!("Invalid value type"));
                }
            },
            ValueEvent::Set(value) => {
                output.value = Some(value);
            }
            ValueEvent::Clear => {
                output.value = None;
            }
        }
        let _ = output.updates.send(event);
        let _ = self.updates.send(SpecChange::Updated(id));
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> BTreeMap<OutputId, OutputSpec> {
        self.state
            .iter()
            .map(|(id, spec)| (*id, spec.clone()))
            .collect()
    }
}
