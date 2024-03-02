use std::sync::Arc;

use crate::Value;
use anyhow::{anyhow, Error};
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub enum ListChange {
    Add(Arc<Value>),
    Pop(Arc<Value>),
    HasMorePages(bool),
    Clear,
    Destroy,
}

#[derive(Debug)]
pub struct ListStream {
    value: Vec<Arc<Value>>,
    updates: broadcast::Sender<ListChange>,
    has_more_rows: bool,
    page_load_sender: broadcast::Sender<u32>,
}

impl ListStream {
    pub(crate) fn new() -> Self {
        let (updates, _) = broadcast::channel::<ListChange>(128);
        let (page_load_sender, _) = broadcast::channel::<u32>(32);
        ListStream {
            value: vec![],
            updates,
            has_more_rows: false,
            page_load_sender,
        }
    }

    pub fn snapshot(&self) -> Vec<Arc<Value>> {
        self.value.to_vec()
    }

    pub(crate) fn add(&mut self, value: Value) -> Result<(), Error> {
        let value_arc = Arc::new(value);
        self.value.push(value_arc.clone());
        let _ = self.updates.send(ListChange::Add(value_arc));
        Ok(())
    }

    pub(crate) fn pop(&mut self) -> Result<(), Error> {
        if let Some(pop) = self.value.pop() {
            let _ = self.updates.send(ListChange::Pop(pop));
            Ok(())
        } else {
            Err(anyhow!("Cannot pop values from an empty list"))
        }
    }

    pub(crate) fn clear(&mut self) -> Result<(), Error> {
        self.value.clear();
        let _ = self.updates.send(ListChange::Clear);
        Ok(())
    }

    pub(crate) fn destroy(&mut self) -> Result<(), Error> {
        self.value.clear();
        let _ = self.updates.send(ListChange::Destroy);
        Ok(())
    }

    pub(crate) fn set_has_more_rows(&mut self, has_more_pages: bool) -> Result<(), Error> {
        self.has_more_rows = has_more_pages;
        let _ = self.updates.send(ListChange::HasMorePages(has_more_pages));
        Ok(())
    }

    pub fn request_page(&mut self, limit: u32) -> Result<bool, Error> {
        if !self.has_more_rows {
            return Ok(false);
        }

        self.page_load_sender.send(limit)?;
        Ok(true)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ListChange> {
        self.updates.subscribe()
    }

    pub(crate) fn get_page_request_stream(&self) -> broadcast::Receiver<u32> {
        self.page_load_sender.subscribe()
    }
}
