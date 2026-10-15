use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use stary_mask::Mask;

/// The masks painted by the user, by the session they belong to. A job takes its mask from here.
#[derive(Default)]
pub struct MaskStore {
    masks: Mutex<HashMap<String, Arc<Mask>>>,
}

impl MaskStore {
    pub fn set(&self, session: String, mask: Mask) {
        if let Ok(mut masks) = self.masks.lock() {
            masks.insert(session, Arc::new(mask));
        }
    }

    pub fn get(&self, session: &str) -> Option<Arc<Mask>> {
        self.masks.lock().ok()?.get(session).cloned()
    }

    pub fn remove(&self, session: &str) {
        if let Ok(mut masks) = self.masks.lock() {
            masks.remove(session);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mask_is_found_by_its_session_until_it_is_removed() {
        let store = MaskStore::default();
        store.set("a".into(), Mask::new(1, 1, vec![1.0]).unwrap());
        assert!(store.get("a").is_some());
        assert!(store.get("b").is_none());
        store.remove("a");
        assert!(store.get("a").is_none());
    }

    #[test]
    fn a_new_mask_replaces_the_old_one() {
        let store = MaskStore::default();
        store.set("a".into(), Mask::new(1, 1, vec![1.0]).unwrap());
        store.set("a".into(), Mask::new(2, 1, vec![0.0, 0.0]).unwrap());
        assert_eq!(store.get("a").unwrap().width(), 2);
    }
}
