use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Error};
use tokio::sync::broadcast;

use crate::bindings::streaming_outputs::TreeNode;

#[derive(Clone, Debug)]
pub enum TreeChange {
    Add {
        parent: Option<String>,
        children: Vec<Arc<TreeNode>>,
    },
    Remove(Arc<TreeNode>),
    Clear,
    Destroy,
}

#[derive(Clone, Debug)]
pub struct TreeStreamNode {
    pub value: Arc<TreeNode>,
    pub children: Vec<TreeStreamNode>,
}

#[derive(Debug)]
pub struct TreeStream {
    nodes: HashMap<String, Arc<TreeNode>>,
    edges: HashMap<Option<String>, Vec<String>>,
    updates: broadcast::Sender<TreeChange>,
    load_children_sender: broadcast::Sender<String>,
}

impl TreeStream {
    pub(crate) fn new() -> Self {
        let (updates, _) = broadcast::channel::<TreeChange>(128);
        let (load_children_sender, _) = broadcast::channel::<String>(32);
        TreeStream {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            updates,
            load_children_sender,
        }
    }

    pub fn snapshot(&self) -> Vec<TreeStreamNode> {
        self.subtree(&None)
    }

    fn subtree(&self, root: &Option<String>) -> Vec<TreeStreamNode> {
        self.edges
            .get(root)
            .map(|edges| {
                edges
                    .iter()
                    .map(|id| self.nodes.get(id).unwrap())
                    .cloned()
                    .map(|value| TreeStreamNode {
                        children: self.subtree(&Some(value.id.clone())),
                        value,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn add(
        &mut self,
        parent: Option<String>,
        children: Vec<TreeNode>,
    ) -> Result<(), Error> {
        if parent.is_some() && !self.nodes.contains_key(parent.as_ref().unwrap()) {
            return Err(anyhow!(
                "Could not add children to non-existent parent {:?}",
                parent
            ));
        }

        let node_arcs: Vec<Arc<TreeNode>> = children.into_iter().map(Arc::new).collect();
        self.nodes.extend(
            node_arcs
                .iter()
                .cloned()
                .map(|node| (node.id.clone(), node)),
        );
        self.edges
            .entry(parent.clone())
            .or_default()
            .extend(node_arcs.iter().map(|n| n.id.clone()));
        let _ = self.updates.send(TreeChange::Add {
            parent,
            children: node_arcs,
        });
        Ok(())
    }

    pub(crate) fn remove(&mut self, id: String) -> Result<(), Error> {
        let Some(node) = self.nodes.remove(&id) else {
            return Err(anyhow!("Could not remove non-existent node {:?}", id));
        };

        if let Some(child_ids) = self.edges.remove(&Some(id)) {
            for child in child_ids {
                self.remove(child)?;
            }
        }

        let _ = self.updates.send(TreeChange::Remove(node));
        Ok(())
    }

    pub(crate) fn clear(&mut self) -> Result<(), Error> {
        self.nodes.clear();
        self.edges.clear();
        let _ = self.updates.send(TreeChange::Clear);
        Ok(())
    }

    pub(crate) fn destroy(&mut self) -> Result<(), Error> {
        self.nodes.clear();
        self.edges.clear();
        let _ = self.updates.send(TreeChange::Destroy);
        Ok(())
    }

    pub fn request_children(&mut self, parent: String) -> Result<bool, Error> {
        if !self.nodes.contains_key(&parent) {
            return Ok(false);
        }

        self.load_children_sender.send(parent)?;
        Ok(true)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TreeChange> {
        self.updates.subscribe()
    }

    pub(crate) fn get_request_children_stream(&self) -> broadcast::Receiver<String> {
        self.load_children_sender.subscribe()
    }
}
