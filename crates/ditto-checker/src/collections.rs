use std::collections::HashMap;

pub struct PristineMap<K, V>(pub HashMap<K, V>);

pub struct Collision<K, V> {
    pub key: K,
    pub existing_value: V,
    pub new_value: V,
}

impl<K, V> PristineMap<K, V> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl<K, V> PristineMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    pub fn insert_unchecked(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    pub fn insert_with_warning(
        &mut self,
        key: K,
        value: V,
        on_collision: impl FnOnce(Collision<&K, &V>),
    ) -> Option<V> {
        if let Some(existing_value) = self.0.get(&key) {
            on_collision(Collision {
                key: &key,
                existing_value,
                new_value: &value,
            });
        }
        self.0.insert(key, value)
    }

    pub fn insert_else<E>(
        &mut self,
        key: K,
        value: V,
        on_collision: impl FnOnce(Collision<K, V>) -> E,
    ) -> Result<Option<V>, E> {
        if let Some(existing_value) = self.0.remove(&key) {
            Err(on_collision(Collision {
                key,
                existing_value,
                new_value: value,
            }))
        } else {
            Ok(self.0.insert(key, value))
        }
    }

    pub fn extend_unchecked(&mut self, iter: impl std::iter::IntoIterator<Item = (K, V)>) {
        self.0.extend(iter);
    }

    //pub fn extend_with_warnings(
    //    &mut self,
    //    iter: impl std::iter::IntoIterator<Item = (K, V)>,
    //    on_collision: impl FnOnce(Collision<&K, &V>) + Copy,
    //) {
    //    for (key, value) in iter {
    //        self.insert_with_warning(key, value, on_collision);
    //    }
    //}

    pub fn extend_else<E>(
        &mut self,
        iter: impl std::iter::IntoIterator<Item = (K, V)>,
        on_collision: impl FnOnce(Collision<K, V>) -> E + Copy,
    ) -> Result<(), E> {
        for (key, value) in iter {
            self.insert_else(key, value, on_collision)?;
        }
        Ok(())
    }
}
