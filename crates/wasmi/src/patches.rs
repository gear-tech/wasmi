//! Gear patches to WASMI

use crate::{
    global::GlobalEntity,
    store::{StoreIdx, Stored},
    Global,
    GlobalIdx,
};
use alloc::rc::Rc;
use core::{cell::RefCell, marker::PhantomData};
use wasmi_arena::Arena;

type GlobalsArena = Arena<GlobalIdx, GlobalEntity>;

#[derive(Debug, Clone)]
pub struct Globals {
    store_idx: StoreIdx,
    arena: Rc<RefCell<GlobalsArena>>,
    // to be sure the type is neither Sync nor Send
    // because we don't want the executor to run into UB
    // as its cache may have a mutable pointer to global
    _pd: PhantomData<*mut ()>,
}

impl Globals {
    pub(crate) fn new(store_idx: StoreIdx) -> Self {
        Self {
            store_idx,
            arena: Rc::default(),
            _pd: PhantomData,
        }
    }

    pub(crate) fn alloc(&mut self, entity: GlobalEntity) -> GlobalIdx {
        self.arena.borrow_mut().alloc(entity)
    }

    fn unwrap_stored(&self, stored: &Stored<GlobalIdx>) -> GlobalIdx {
        stored.entity_index(self.store_idx).unwrap_or_else(|| {
            panic!(
                "entity reference ({:?}) does not belong to store {:?}",
                stored, self.store_idx,
            )
        })
    }

    pub fn resolve(&self, global: &Global) -> GlobalEntity {
        let idx = self.unwrap_stored(global.as_inner());
        self.arena
            .borrow()
            .get(idx)
            .unwrap_or_else(|| panic!("failed to resolve stored entity: {idx:?}"))
            .clone()
    }

    pub fn resolve_mut_with<F, R>(&self, global: &Global, f: F) -> R
    where
        F: FnOnce(&mut GlobalEntity) -> R,
    {
        let idx = self.unwrap_stored(global.as_inner());
        let mut arena = self.arena.borrow_mut();
        let entity = arena
            .get_mut(idx)
            .unwrap_or_else(|| panic!("failed to resolve stored entity: {idx:?}"));
        f(entity)
    }
}
