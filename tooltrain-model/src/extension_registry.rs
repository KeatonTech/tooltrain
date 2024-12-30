use std::{
    cell::RefCell,
    sync::{Arc, OnceLock, RwLock},
};

use serde::{Deserialize, Serialize};
use type_reg::untagged::TypeReg;

thread_local! {
    static REGISTRY_CACHE: RefCell<Option<(usize, Arc<TypeReg<u16>>)>> = RefCell::new(None);
}

pub struct TooltrainModelExtensionRegistry(
    RwLock<Vec<Box<dyn FnMut(&mut TypeReg<u16>) + Send + Sync>>>,
);

impl TooltrainModelExtensionRegistry {
    fn new() -> Self {
        Self(RwLock::new(vec![]))
    }

    pub fn extend<F>(&self, adder: F)
    where
        F: FnMut(&mut TypeReg<u16>),
        F: Send,
        F: Sync,
        F: 'static,
    {
        self.0.write().unwrap().push(Box::new(adder));
    }

    fn build(&self) -> Arc<TypeReg<u16>> {
        let writer = self.0.write().unwrap();
        if let Some((cache_id, reg)) = REGISTRY_CACHE.with(|cache| cache.borrow().clone()) {
            if cache_id == writer.len() {
                return reg;
            }
        }
        let mut reg = TypeReg::new();
        for adder in self.0.write().unwrap().iter_mut() {
            adder.as_mut()(&mut reg);
        }
        REGISTRY_CACHE.replace(Some((writer.len(), Arc::new(reg))));
        REGISTRY_CACHE.with(|cache| cache.borrow().clone().unwrap().1)
    }
}

pub fn get_global_extension_registry() -> &'static TooltrainModelExtensionRegistry {
    static EXTENSION_REGISTRY: OnceLock<TooltrainModelExtensionRegistry> = OnceLock::new();
    EXTENSION_REGISTRY.get_or_init(|| TooltrainModelExtensionRegistry::new())
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Extras(type_reg::untagged::TypeMap<u16>);

impl<'de> Deserialize<'de> for Extras {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Extras(
            get_global_extension_registry()
                .build()
                .deserialize_map(deserializer)?,
        ))
    }
}
