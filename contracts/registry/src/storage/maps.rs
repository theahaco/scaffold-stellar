#![allow(clippy::must_use_candidate, clippy::missing_errors_doc)]
use core::marker::PhantomData;

use soroban_sdk::{Env, IntoVal, TryFromVal, Val};

pub trait ToStorageKey<Key: IntoVal<Env, Val> + Clone> {
    fn to_key(env: &Env, k: &Key) -> Val;
}

#[derive(Clone)]
pub struct PersistentMap<K, V, W>
where
    K: IntoVal<Env, Val> + Clone,
    W: ToStorageKey<K>,
    V: IntoVal<Env, Val> + TryFromVal<Env, Val>,
{
    env: Env,
    k: PhantomData<K>,
    v: PhantomData<V>,
    w: PhantomData<W>,
}

impl<K, V, W> PersistentMap<K, V, W>
where
    K: IntoVal<Env, Val> + Clone,
    W: ToStorageKey<K>,
    V: IntoVal<Env, Val> + TryFromVal<Env, Val>,
{
    pub fn new(env: &Env) -> Self {
        Self {
            env: env.clone(),
            k: PhantomData,
            v: PhantomData,
            w: PhantomData,
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let k = W::to_key(&self.env, key);
        self.env.storage().persistent().get(&k)
    }

    pub fn set(&mut self, key: &K, value: &V) {
        let k = W::to_key(&self.env, key);
        self.env.storage().persistent().set(&k, value);
    }

    pub fn has(&self, key: &K) -> bool {
        let k = W::to_key(&self.env, key);
        self.env.storage().persistent().has(&k)
    }

    pub fn extend_ttl(&self, key: &K, threshold: u32, extend_to: u32) {
        let k = W::to_key(&self.env, key);
        self.env
            .storage()
            .persistent()
            .extend_ttl(&k, threshold, extend_to);
    }
}
