use std::sync::Arc;

use crate::Value;

mod list;
mod tree;
mod value;

use anyhow::{anyhow, Error};
pub use list::{ListStream, ListChange};
use tokio::sync::broadcast;
pub use tree::{TreeStream, TreeChange, TreeStreamNode};
pub use value::{ValueStream, ValueChange};


#[derive(Debug)]
pub enum DataStream {
    List(ListStream),
    Tree(TreeStream),
    Value(ValueStream)
}

#[derive(Debug)]
pub enum DataStreamUpdate {
    List(broadcast::Receiver<ListChange>),
    Tree(broadcast::Receiver<TreeChange>),
    Value(broadcast::Receiver<ValueChange>)
}

#[derive(Clone, Debug)]
pub enum DataStreamSnapshot {
    List(Vec<Arc<Value>>),
    Tree(Vec<TreeStreamNode>),
    Value(Option<Arc<Value>>)
}

impl DataStream {
    pub fn try_get_list(&self) -> Result<&ListStream, Error> {
        match self {
            DataStream::List(l) => Ok(l),
            _ => Err(anyhow!("DataStream is not a List"))
        }
    }

    pub fn try_get_list_mut(&mut self) -> Result<&mut ListStream, Error> {
        match self {
            DataStream::List(l) => Ok(l),
            _ => Err(anyhow!("DataStream is not a List"))
        }
    }

    pub fn try_get_tree(&self) -> Result<&TreeStream, Error> {
        match self {
            DataStream::Tree(t) => Ok(t),
            _ => Err(anyhow!("DataStream is not a Tree"))
        }
    }

    pub fn try_get_tree_mut(&mut self) -> Result<&mut TreeStream, Error> {
        match self {
            DataStream::Tree(t) => Ok(t),
            _ => Err(anyhow!("DataStream is not a Tree"))
        }
    }

    pub fn try_get_value(&self) -> Result<&ValueStream, Error> {
        match self {
            DataStream::Value(v) => Ok(v),
            _ => Err(anyhow!("DataStream is not a Value"))
        }
    }

    pub fn try_get_value_mut(&mut self) -> Result<&mut ValueStream, Error> {
        match self {
            DataStream::Value(v) => Ok(v),
            _ => Err(anyhow!("DataStream is not a Value"))
        }
    }

    pub fn subscribe(&self) -> DataStreamUpdates {
        match self {
            DataStream::List(l) => DataStreamUpdates::List(l.subscribe()),
            DataStream::Tree(t) => DataStreamUpdates::Tree(t.subscribe()),
            DataStream::Value(v) => DataStreamUpdates::Value(v.subscribe())
        }
    }

    pub fn snapshot(&self) -> DataStreamSnapshot {
        match self {
            DataStream::List(l) => DataStreamSnapshot::List(l.snapshot()),
            DataStream::Tree(t) => DataStreamSnapshot::Tree(t.snapshot()),
            DataStream::Value(v) => DataStreamSnapshot::Value(v.snapshot())
        }
    }

    pub fn destroy(self) -> Result<(), Error> {
        match self {
            DataStream::List(mut l) => l.destroy(),
            DataStream::Tree(mut t) => t.destroy(),
            DataStream::Value(mut v) => v.destroy()
        }
    }
}