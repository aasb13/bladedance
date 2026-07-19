// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.
//
// Ports the data-structure/algorithm core of Extensible and ExtensionManager
// (include/extensible.h, src/extensible.cpp) to Rust. The virtual-dispatch
// shell (ExtensionItem and its subclasses in include/extension.h) stays in
// C++ for now because it is templated on arbitrary value types and calls
// back into Module/Server, neither of which is ported yet. This module only
// takes over the storage engine: the flat key->value map per Extensible
// instance, and the name->ExtensionItem registry in ExtensionManager.
//
// Original `insp::flat_map` is a sorted-vector-backed map, so representing
// both containers as plain Vec<(key, value)> with linear scan is a faithful,
// not just convenient, port -- for the small number of extensions typically
// attached to a single User/Channel/Membership, linear scan is also what the
// original effectively did.

use std::ffi::{c_char, CStr};
use std::os::raw::c_void;

use crate::hashcomp::equals as insensitive_equals;

// ---------------------------------------------------------------------
// Per-Extensible storage: ExtensionItem* (as usize) -> void* (as usize)
// ---------------------------------------------------------------------

pub struct ExtensibleStore {
    entries: Vec<(usize, usize)>,
}

impl ExtensibleStore {
    fn new() -> Self {
        ExtensibleStore { entries: Vec::new() }
    }

    fn get_raw(&self, key: usize) -> usize {
        self.entries
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or(0)
    }

    /// Sets `key` to `value`, returning the previous value (0 if unset).
    fn set_raw(&mut self, key: usize, value: usize) -> usize {
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            let old = slot.1;
            slot.1 = value;
            old
        } else {
            self.entries.push((key, value));
            0
        }
    }

    /// Removes `key`, returning the value that was set (0 if unset).
    fn unset_raw(&mut self, key: usize) -> usize {
        if let Some(pos) = self.entries.iter().position(|(k, _)| *k == key) {
            self.entries.remove(pos).1
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_new() -> *mut ExtensibleStore {
    Box::into_raw(Box::new(ExtensibleStore::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_free(store: *mut ExtensibleStore) {
    if store.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(store));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_get_raw(store: *const ExtensibleStore, key: usize) -> usize {
    if store.is_null() {
        return 0;
    }
    unsafe { (*store).get_raw(key) }
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_set_raw(
    store: *mut ExtensibleStore,
    key: usize,
    value: usize,
) -> usize {
    if store.is_null() {
        return 0;
    }
    unsafe { (*store).set_raw(key, value) }
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_unset_raw(store: *mut ExtensibleStore, key: usize) -> usize {
    if store.is_null() {
        return 0;
    }
    unsafe { (*store).unset_raw(key) }
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_is_empty(store: *const ExtensibleStore) -> bool {
    if store.is_null() {
        return true;
    }
    unsafe { (*store).entries.is_empty() }
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_len(store: *const ExtensibleStore) -> usize {
    if store.is_null() {
        return 0;
    }
    unsafe { (*store).entries.len() }
}

/// Reads the entry at `idx` into `out_key`/`out_value`. Returns false (and
/// leaves the outputs untouched) if `idx` is out of range, so C++ can drive
/// a simple `for (idx = 0; extensible_store_entry_at(...); idx++)` loop, or
/// pre-read `extensible_store_len` and iterate up to it.
#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_entry_at(
    store: *const ExtensibleStore,
    idx: usize,
    out_key: *mut usize,
    out_value: *mut usize,
) -> bool {
    if store.is_null() {
        return false;
    }
    let entries = unsafe { &(*store).entries };
    match entries.get(idx) {
        Some((k, v)) => {
            unsafe {
                *out_key = *k;
                *out_value = *v;
            }
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn extensible_store_clear(store: *mut ExtensibleStore) {
    if store.is_null() {
        return;
    }
    unsafe {
        (*store).entries.clear();
    }
}

// ---------------------------------------------------------------------
// ExtensionManager registry: name (case-insensitive) -> ExtensionItem*
// ---------------------------------------------------------------------

pub struct ExtensionManagerStore {
    // (name, ExtensionItem* as usize, creator Module* as usize)
    types: Vec<(String, usize, usize)>,
}

impl ExtensionManagerStore {
    fn new() -> Self {
        ExtensionManagerStore { types: Vec::new() }
    }

    fn find_index(&self, name: &str) -> Option<usize> {
        self.types.iter().position(|(n, _, _)| insensitive_equals(n, name))
    }

    /// Registers `item` under `name`/`creator`. Returns false if a
    /// registration with the same (case-insensitive) name already exists,
    /// matching ExtensionManager::Register's `map::emplace` semantics.
    fn register(&mut self, name: &str, item: usize, creator: usize) -> bool {
        if self.find_index(name).is_some() {
            return false;
        }
        self.types.push((name.to_string(), item, creator));
        true
    }

    fn get_item(&self, name: &str) -> usize {
        self.find_index(name).map(|i| self.types[i].1).unwrap_or(0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn extension_manager_new() -> *mut ExtensionManagerStore {
    Box::into_raw(Box::new(ExtensionManagerStore::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn extension_manager_free(mgr: *mut ExtensionManagerStore) {
    if mgr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(mgr));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn extension_manager_register(
    mgr: *mut ExtensionManagerStore,
    name: *const c_char,
    item: usize,
    creator: usize,
) -> bool {
    if mgr.is_null() || name.is_null() {
        return false;
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    unsafe { (*mgr).register(name, item, creator) }
}

#[unsafe(no_mangle)]
pub extern "C" fn extension_manager_get_item(
    mgr: *const ExtensionManagerStore,
    name: *const c_char,
) -> usize {
    if mgr.is_null() || name.is_null() {
        return 0;
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    unsafe { (*mgr).get_item(name) }
}

#[unsafe(no_mangle)]
pub extern "C" fn extension_manager_len(mgr: *const ExtensionManagerStore) -> usize {
    if mgr.is_null() {
        return 0;
    }
    unsafe { (*mgr).types.len() }
}

/// Reads the (name-as-owned-c-string-not-provided, item, creator) triple at
/// `idx`. Name is not returned here (callers needing it use
/// `extension_manager_get_item`/`register` by name instead); this is used
/// for the `GetExts()` enumeration, which on the C++ side only ever reads
/// the ExtensionItem* (see core_reloadmodule.cpp), so we hand back the item
/// pointer and let C++ read the ExtensionItem's own `name` field if needed.
#[unsafe(no_mangle)]
pub extern "C" fn extension_manager_entry_at(
    mgr: *const ExtensionManagerStore,
    idx: usize,
    out_item: *mut usize,
) -> bool {
    if mgr.is_null() {
        return false;
    }
    let types = unsafe { &(*mgr).types };
    match types.get(idx) {
        Some((_, item, _)) => {
            unsafe {
                *out_item = *item;
            }
            true
        }
        None => false,
    }
}

/// Removes every registration whose creator matches `module_ptr`, writing
/// the removed ExtensionItem* values into `out_buf` (capacity `buf_len`,
/// which callers should size to at least `extension_manager_len(mgr)` to
/// guarantee no truncation). Returns the number of items removed and
/// written. Mirrors ExtensionManager::BeginUnregister.
#[unsafe(no_mangle)]
pub extern "C" fn extension_manager_begin_unregister(
    mgr: *mut ExtensionManagerStore,
    module_ptr: usize,
    out_buf: *mut usize,
    buf_len: usize,
) -> usize {
    if mgr.is_null() {
        return 0;
    }
    let store = unsafe { &mut *mgr };
    let mut written = 0usize;
    let mut i = 0;
    while i < store.types.len() {
        if store.types[i].2 == module_ptr {
            let (_, item, _) = store.types.remove(i);
            if written < buf_len {
                unsafe {
                    *out_buf.add(written) = item;
                }
                written += 1;
            }
        } else {
            i += 1;
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_set_get_unset() {
        let mut s = ExtensibleStore::new();
        assert_eq!(s.get_raw(1), 0);
        assert_eq!(s.set_raw(1, 42), 0);
        assert_eq!(s.get_raw(1), 42);
        assert_eq!(s.set_raw(1, 43), 42);
        assert_eq!(s.unset_raw(1), 43);
        assert_eq!(s.get_raw(1), 0);
        assert_eq!(s.unset_raw(1), 0);
    }

    #[test]
    fn manager_register_case_insensitive() {
        let mut m = ExtensionManagerStore::new();
        assert!(m.register("foo-bar", 100, 1));
        assert!(!m.register("FOO-BAR", 200, 1));
        assert_eq!(m.get_item("Foo-Bar"), 100);
    }

    #[test]
    fn manager_begin_unregister() {
        let mut m = ExtensionManagerStore::new();
        m.register("a", 1, 10);
        m.register("b", 2, 20);
        m.register("c", 3, 10);
        let mut buf = [0usize; 4];
        let n = extension_manager_begin_unregister(&mut m, 10, buf.as_mut_ptr(), buf.len());
        assert_eq!(n, 2);
        assert_eq!(&buf[..2], &[1usize, 3usize]);
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.types[0].0, "b");
    }
}
