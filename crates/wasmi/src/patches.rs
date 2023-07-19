//! Gear patches to WASMI

use crate::{
    global::{GlobalEntity, GlobalIdx},
    store::{StoreIdx, Stored},
    Global,
};
use alloc::sync::Arc;
use core::marker::PhantomData;
use spin::RwLock;
use wasmi_arena::Arena;

#[derive(Debug)]
pub struct Globals {
    inner: InnerGlobals,
    // make the type neither Sync nor Send
    // because we don't want the executor to run into UB
    // as its cache may have a mutable pointer to global
    _pd: PhantomData<*mut ()>,
}

impl Globals {
    pub fn resolve(&self, global: &Global) -> GlobalEntity {
        self.inner.resolve(global)
    }

    pub fn resolve_mut_with<F, R>(&self, global: &Global, f: F) -> R
    where
        F: FnOnce(&mut GlobalEntity) -> R,
    {
        self.inner.resolve_mut_with(global, f)
    }
}

type GlobalsArena = Arena<GlobalIdx, GlobalEntity>;

#[derive(Debug)]
pub(crate) struct InnerGlobals {
    store_idx: StoreIdx,
    pub(crate) arena: Arc<RwLock<GlobalsArena>>,
}

impl InnerGlobals {
    pub(crate) fn new(store_idx: StoreIdx) -> Self {
        Self {
            store_idx,
            arena: Arc::default(),
        }
    }

    fn unwrap_stored(&self, stored: &Stored<GlobalIdx>) -> GlobalIdx {
        stored.entity_index(self.store_idx).unwrap_or_else(|| {
            panic!(
                "entity reference ({:?}) does not belong to store {:?}",
                stored, self.store_idx,
            )
        })
    }

    pub(crate) fn resolve(&self, global: &Global) -> GlobalEntity {
        let idx = self.unwrap_stored(global.as_inner());
        self.arena
            .read()
            .get(idx)
            .unwrap_or_else(|| panic!("failed to resolve stored entity: {idx:?}"))
            .clone()
    }

    pub(crate) fn resolve_mut_with<F, R>(&self, global: &Global, f: F) -> R
    where
        F: FnOnce(&mut GlobalEntity) -> R,
    {
        let idx = self.unwrap_stored(global.as_inner());
        let mut arena = self.arena.write();
        let entity = arena
            .get_mut(idx)
            .unwrap_or_else(|| panic!("failed to resolve stored entity: {idx:?}"));
        f(entity)
    }

    pub(crate) fn outer_globals(&self) -> Globals {
        Globals {
            inner: Self {
                store_idx: self.store_idx,
                arena: self.arena.clone(),
            },
            _pd: PhantomData,
        }
    }
}
