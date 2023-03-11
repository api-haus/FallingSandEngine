use std::{
    collections::HashMap,
    fmt::Debug,
    hash::{BuildHasher, BuildHasherDefault, Hash},
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct ChunkManager<D> {
    chunks: HashMap<ChunkKey, Chunk<D>, ahash::RandomState>,
}

pub type ChunkKey = (i32, i32);

#[derive(Debug)]
pub struct Chunk<D> {
    chunk_x: i32,
    chunk_y: i32,

    pub data: D,
}

impl<D> Deref for Chunk<D> {
    type Target = D;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<D> DerefMut for Chunk<D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Default)]
pub struct PassThroughHasherI32I32(([i32; 2], usize));

impl std::hash::Hasher for PassThroughHasherI32I32 {
    #[inline]
    fn finish(&self) -> u64 {
        u64::from_le_bytes(unsafe {
            // Safety: casting [i32; 2] to u64 is fine
            *(self.0 .0.as_ptr() as *const () as *const _)
        })
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        *unsafe { self.0 .0.get_unchecked_mut(self.0 .1) } = i;
        self.0 .1 = if self.0 .1 == 0 {
            1
        } else {
            panic!("cannot be called more than twice")
        };
    }

    #[inline]
    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("PassThroughHasherI32I32 only supports (i32, i32)")
    }
}

impl<D> ChunkManager<D> {
    #[inline]
    pub fn new() -> Self {
        Self { chunks: HashMap::default() }
    }

    #[inline]
    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            chunks: HashMap::with_capacity_and_hasher(capacity, ahash::RandomState::default()),
        }
    }

    #[inline]
    pub fn insert(&mut self, chunk_pos: (i32, i32), data: D) {
        self.chunks.insert(
            chunk_pos,
            Chunk { chunk_x: chunk_pos.0, chunk_y: chunk_pos.1, data },
        );
    }

    #[inline]
    pub fn contains(&self, chunk_pos: (i32, i32)) -> bool {
        self.chunks.contains_key(&chunk_pos)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    #[inline]
    pub fn chunk_at_mut_with_surrounding(
        &mut self,
        chunk_pos: (i32, i32),
        cb: impl FnOnce(&mut Chunk<D>, [Option<&Chunk<D>>; 8]),
    ) {
        if let Some(mut this) = self.chunks.remove(&chunk_pos) {
            let surrounding = [
                self.chunk_at((chunk_pos.0 - 1, chunk_pos.1 - 1)),
                self.chunk_at((chunk_pos.0, chunk_pos.1 - 1)),
                self.chunk_at((chunk_pos.0 + 1, chunk_pos.1 - 1)),
                self.chunk_at((chunk_pos.0 - 1, chunk_pos.1)),
                self.chunk_at((chunk_pos.0 + 1, chunk_pos.1)),
                self.chunk_at((chunk_pos.0 - 1, chunk_pos.1 + 1)),
                self.chunk_at((chunk_pos.0, chunk_pos.1 + 1)),
                self.chunk_at((chunk_pos.0 + 1, chunk_pos.1 + 1)),
            ];

            cb(&mut this, surrounding);

            self.chunks.insert(chunk_pos, this);
        }
    }

    #[inline]
    pub fn each_chunk_mut_with_surrounding(
        &mut self,
        cb: impl Fn(&mut Chunk<D>, [Option<&Chunk<D>>; 8]),
    ) {
        let keys = self.keys();
        for k in keys {
            // Safety: we are iterating though keys, so the value must exist
            let mut this = unsafe { self.chunks.remove(&k).unwrap_unchecked() };

            let surrounding = [
                self.chunk_at((k.0 - 1, k.1 - 1)),
                self.chunk_at((k.0, k.1 - 1)),
                self.chunk_at((k.0 + 1, k.1 - 1)),
                self.chunk_at((k.0 - 1, k.1)),
                self.chunk_at((k.0 + 1, k.1)),
                self.chunk_at((k.0 - 1, k.1 + 1)),
                self.chunk_at((k.0, k.1 + 1)),
                self.chunk_at((k.0 + 1, k.1 + 1)),
            ];

            cb(&mut this, surrounding);

            self.chunks.insert(k, this);
        }
    }

    /// # Safety
    /// Raw access to the chunks map makes it possible to move [`Chunk`]s to invalid keys.
    #[inline]
    pub unsafe fn raw(&self) -> &HashMap<ChunkKey, Chunk<D>, ahash::RandomState> {
        &self.chunks
    }

    /// # Safety
    /// Raw access to the chunks map makes it possible to move [`Chunk`]s to invalid keys.
    #[inline]
    pub unsafe fn raw_mut(&mut self) -> &mut HashMap<ChunkKey, Chunk<D>, ahash::RandomState> {
        &mut self.chunks
    }
}

impl<D> Default for ChunkManager<D> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<D> Chunk<D> {
    #[inline]
    pub fn chunk_x(&self) -> i32 {
        self.chunk_x
    }

    #[inline]
    pub fn chunk_y(&self) -> i32 {
        self.chunk_y
    }
}

pub type BoxedIterator<'a, I> = Box<dyn Iterator<Item = I> + 'a>;

pub trait ChunkQuery<'a, D: 'a> {
    fn chunk_at(&self, chunk_pos: ChunkKey) -> Option<&Chunk<D>>;
    fn chunk_at_mut(&mut self, chunk_pos: ChunkKey) -> Option<&mut Chunk<D>>;

    // TODO: use ATs since impl can't be used here
    fn chunks_iter(&self) -> BoxedIterator<&Chunk<D>>;
    fn chunks_iter_mut(&mut self) -> BoxedIterator<&mut Chunk<D>>;
    fn kv_iter(&self) -> BoxedIterator<(ChunkKey, &Chunk<D>)>;
    fn kv_iter_mut(&mut self) -> BoxedIterator<(ChunkKey, &mut Chunk<D>)>;

    fn keys(&self) -> Vec<ChunkKey>;
    fn query_one(&mut self, chunk_pos: ChunkKey) -> Option<ChunkQueryOne<D>>;

    #[inline]
    fn query_each(&mut self, mut cb: impl FnMut(ChunkQueryOne<D>)) {
        let keys = self.keys();
        for k in keys {
            // we're iterating keys so we know they're valid
            let query = unsafe { self.query_one(k).unwrap_unchecked() };
            cb(query);
        }
    }

    #[inline]
    fn chunk_at_with_others(
        &self,
        chunk_pos: (i32, i32),
    ) -> Option<(&Chunk<D>, BoxedIterator<&Chunk<D>>)> {
        // TODO: is there a way to partition into two iterators or something instead of collecting?
        let (one, others) = self
            .chunks_iter()
            .partition::<Vec<_>, _>(|ch| ch.chunk_x == chunk_pos.0 && ch.chunk_y == chunk_pos.1);
        one.into_iter()
            .next()
            .map(|ch| (ch, Box::new(others.into_iter()) as _))
    }

    #[inline]
    fn chunk_at_with_others_mut(
        &mut self,
        chunk_pos: (i32, i32),
    ) -> Option<(&mut Chunk<D>, BoxedIterator<&mut Chunk<D>>)> {
        // TODO: is there a way to partition into two iterators or something instead of collecting?
        let (one, others) = self
            .chunks_iter_mut()
            .partition::<Vec<_>, _>(|ch| ch.chunk_x == chunk_pos.0 && ch.chunk_y == chunk_pos.1);
        one.into_iter()
            .next()
            .map(|ch| (ch, Box::new(others.into_iter()) as _))
    }
}

pub struct ChunkQueryOne<'a, D> {
    key: ChunkKey,
    chunks: BorrowOrOwnMap<'a, ChunkKey, Chunk<D>, ahash::RandomState>,
}

enum BorrowOrOwnMap<'a, K, V, H = std::collections::hash_map::RandomState> {
    BorrowOwned(&'a mut HashMap<K, V, H>),
    OwnBorrowed(HashMap<K, &'a mut V, H>),
}

impl<K: Eq + Hash + Copy, V, H: BuildHasher> BorrowOrOwnMap<'_, K, V, H> {
    #[inline]
    fn get(&self, key: &K) -> Option<&V> {
        match self {
            BorrowOrOwnMap::BorrowOwned(m) => m.get(key),
            BorrowOrOwnMap::OwnBorrowed(m) => m.get(key).map(|v| &**v),
        }
    }

    #[inline]
    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        match self {
            BorrowOrOwnMap::BorrowOwned(m) => m.get_mut(key),
            BorrowOrOwnMap::OwnBorrowed(m) => m.get_mut(key).map(|v| &mut **v),
        }
    }

    #[inline]
    fn values(&self) -> BoxedIterator<&V> {
        match self {
            BorrowOrOwnMap::BorrowOwned(m) => Box::new(m.values()),
            BorrowOrOwnMap::OwnBorrowed(m) => Box::new(m.values().map(|v| &**v)),
        }
    }

    #[inline]
    fn values_mut(&mut self) -> BoxedIterator<&mut V> {
        match self {
            BorrowOrOwnMap::BorrowOwned(m) => Box::new(m.values_mut()),
            BorrowOrOwnMap::OwnBorrowed(m) => Box::new(m.values_mut().map(|v| &mut **v)),
        }
    }

    #[inline]
    fn keys(&self) -> BoxedIterator<&K> {
        match self {
            BorrowOrOwnMap::BorrowOwned(m) => Box::new(m.keys()),
            BorrowOrOwnMap::OwnBorrowed(m) => Box::new(m.keys()),
        }
    }

    #[inline]
    fn iter(&self) -> BoxedIterator<(K, &V)> {
        match self {
            BorrowOrOwnMap::BorrowOwned(m) => Box::new(m.iter().map(|(k, v)| (*k, v))),
            BorrowOrOwnMap::OwnBorrowed(m) => Box::new(m.iter().map(|(k, v)| (*k, &**v))),
        }
    }

    #[inline]
    fn iter_mut(&mut self) -> BoxedIterator<(K, &mut V)> {
        match self {
            BorrowOrOwnMap::BorrowOwned(m) => Box::new(m.iter_mut().map(|(k, v)| (*k, v))),
            BorrowOrOwnMap::OwnBorrowed(m) => Box::new(m.iter_mut().map(|(k, v)| (*k, &mut **v))),
        }
    }
}

impl<'a, D> ChunkQueryOne<'a, D> {
    #[inline]
    pub fn one(&mut self) -> &mut Chunk<D> {
        self.chunks
            .get_mut(&self.key)
            .expect("ChunkQueryOne had invalid key")
    }

    #[inline]
    pub fn for_each_with<I, A: Accessor<I>>(
        &mut self,
        get_accessor: impl Fn(&mut D) -> &mut A,
        mut cb: impl FnMut(&mut I, &mut [&mut Chunk<D>]),
    ) {
        for k in get_accessor(&mut self.one().data).keys() {
            let mut this = get_accessor(&mut self.one().data).remove(&k);

            let (this_chunk, others) = self.chunk_at_with_others_mut(self.key).unwrap();

            let mut others = std::iter::once(this_chunk)
                .chain(others)
                .collect::<Vec<_>>();

            cb(&mut this, &mut others);

            get_accessor(&mut self.one().data).insert(k, this);
        }
    }
}

impl<'a, D: 'a, T: Deref<Target = Chunk<D>> + DerefMut> ChunkQuery<'a, D> for [T] {
    #[inline]
    fn chunk_at(&self, chunk_pos: ChunkKey) -> Option<&Chunk<D>> {
        self.iter()
            .map(|t| &**t)
            .find(|ch| ch.chunk_x() == chunk_pos.0 && ch.chunk_y() == chunk_pos.1)
    }

    #[inline]
    fn chunk_at_mut(&mut self, chunk_pos: ChunkKey) -> Option<&mut Chunk<D>> {
        self.iter_mut()
            .map(|t| &mut **t)
            .find(|ch| ch.chunk_x() == chunk_pos.0 && ch.chunk_y() == chunk_pos.1)
    }

    #[inline]
    fn chunks_iter(&self) -> BoxedIterator<&Chunk<D>> {
        Box::new(self.iter().map(|t| &**t))
    }

    #[inline]
    fn chunks_iter_mut(&mut self) -> BoxedIterator<&mut Chunk<D>> {
        Box::new(self.iter_mut().map(|t| &mut **t))
    }

    #[inline]
    fn kv_iter(&self) -> BoxedIterator<(ChunkKey, &Chunk<D>)> {
        Box::new(
            self.iter()
                .map(|t| &**t)
                .map(|ch| ((ch.chunk_x(), ch.chunk_y()), ch)),
        )
    }

    #[inline]
    fn kv_iter_mut(&mut self) -> BoxedIterator<(ChunkKey, &mut Chunk<D>)> {
        Box::new(
            self.iter_mut()
                .map(|t| &mut **t)
                .map(|ch| ((ch.chunk_x(), ch.chunk_y()), ch)),
        )
    }

    #[inline]
    fn keys(&self) -> Vec<ChunkKey> {
        self.iter()
            .map(|t| &**t)
            .map(|ch| (ch.chunk_x(), ch.chunk_y()))
            .collect()
    }

    #[inline]
    fn query_one(&mut self, chunk_pos: ChunkKey) -> Option<ChunkQueryOne<D>> {
        if self.chunk_at(chunk_pos).is_some() {
            let map = self
                .iter_mut()
                .map(|c| &mut **c)
                .map(|c| ((c.chunk_x(), c.chunk_y()), c))
                .collect();
            Some(ChunkQueryOne {
                key: chunk_pos,
                chunks: BorrowOrOwnMap::OwnBorrowed(map),
            })
        } else {
            None
        }
    }
}

impl<'a, D: 'a> ChunkQuery<'a, D> for ChunkManager<D> {
    #[inline]
    fn chunk_at(&self, chunk_pos: ChunkKey) -> Option<&Chunk<D>> {
        self.chunks.get(&chunk_pos)
    }

    #[inline]
    fn chunk_at_mut(&mut self, chunk_pos: ChunkKey) -> Option<&mut Chunk<D>> {
        self.chunks.get_mut(&chunk_pos)
    }

    #[inline]
    fn chunks_iter(&self) -> BoxedIterator<&Chunk<D>> {
        Box::new(self.chunks.values())
    }

    fn chunks_iter_mut(&mut self) -> BoxedIterator<&mut Chunk<D>> {
        Box::new(self.chunks.values_mut())
    }

    #[inline]
    fn kv_iter(&self) -> BoxedIterator<(ChunkKey, &Chunk<D>)> {
        Box::new(self.chunks.iter().map(|(k, v)| (*k, v)))
    }

    #[inline]
    fn kv_iter_mut(&mut self) -> BoxedIterator<(ChunkKey, &mut Chunk<D>)> {
        Box::new(self.chunks.iter_mut().map(|(k, v)| (*k, v)))
    }

    #[inline]
    fn keys(&self) -> Vec<ChunkKey> {
        self.chunks.keys().copied().collect()
    }

    #[inline]
    fn query_one(&mut self, chunk_pos: ChunkKey) -> Option<ChunkQueryOne<D>> {
        if self.chunk_at(chunk_pos).is_some() {
            Some(ChunkQueryOne {
                key: chunk_pos,
                chunks: BorrowOrOwnMap::BorrowOwned(&mut self.chunks),
            })
        } else {
            None
        }
    }
}

impl<'a, D> ChunkQuery<'a, D> for ChunkQueryOne<'a, D> {
    #[inline]
    fn chunk_at(&self, chunk_pos: ChunkKey) -> Option<&Chunk<D>> {
        self.chunks.get(&chunk_pos)
    }

    #[inline]
    fn chunk_at_mut(&mut self, chunk_pos: ChunkKey) -> Option<&mut Chunk<D>> {
        self.chunks.get_mut(&chunk_pos)
    }

    #[inline]
    fn chunks_iter(&self) -> BoxedIterator<&Chunk<D>> {
        Box::new(self.chunks.values())
    }

    #[inline]
    fn chunks_iter_mut(&mut self) -> BoxedIterator<&mut Chunk<D>> {
        Box::new(self.chunks.values_mut())
    }

    #[inline]
    fn kv_iter(&self) -> BoxedIterator<(ChunkKey, &Chunk<D>)> {
        Box::new(self.chunks.iter())
    }

    #[inline]
    fn kv_iter_mut(&mut self) -> BoxedIterator<(ChunkKey, &mut Chunk<D>)> {
        Box::new(self.chunks.iter_mut())
    }

    #[inline]
    fn keys(&self) -> Vec<ChunkKey> {
        self.chunks.keys().copied().collect()
    }

    #[inline]
    fn query_one(&mut self, chunk_pos: ChunkKey) -> Option<ChunkQueryOne<D>> {
        if self.chunk_at(chunk_pos).is_some() {
            Some(ChunkQueryOne {
                key: chunk_pos,
                chunks: match &mut self.chunks {
                    BorrowOrOwnMap::BorrowOwned(m) => BorrowOrOwnMap::BorrowOwned(&mut **m),
                    BorrowOrOwnMap::OwnBorrowed(m) => BorrowOrOwnMap::OwnBorrowed(
                        m.iter_mut().map(|(k, v)| (*k, &mut **v)).collect(),
                    ),
                },
            })
        } else {
            None
        }
    }
}

pub trait Accessor<V> {
    type K;

    fn keys(&self) -> Vec<Self::K>;
    fn remove(&mut self, key: &Self::K) -> V;
    fn insert(&mut self, key: Self::K, value: V);
}

impl<T> Accessor<T> for Vec<T> {
    type K = usize;

    #[inline]
    fn keys(&self) -> Vec<Self::K> {
        (0..self.len()).collect::<Vec<_>>()
    }

    #[inline]
    fn remove(&mut self, key: &Self::K) -> T {
        self.remove(*key)
    }

    #[inline]
    fn insert(&mut self, key: Self::K, value: T) {
        self.insert(key, value);
    }
}

#[cfg(test)]
#[allow(unused)]
mod test {
    use std::cell::Cell;

    use crate::{Chunk, ChunkManager, ChunkQuery, ChunkQueryOne};

    #[derive(Debug)]
    struct Data {
        items: Vec<Cell<i32>>,
    }

    trait ChunkQueryExt {
        fn get_item(&mut self, chunk_x: i32, chunk_y: i32, idx: usize) -> Option<&mut Cell<i32>>;
    }

    impl<'a, Q: ChunkQuery<'a, Data>> ChunkQueryExt for Q {
        fn get_item(&mut self, chunk_x: i32, chunk_y: i32, idx: usize) -> Option<&mut Cell<i32>> {
            self.chunks_iter_mut()
                .find(|ch| ch.chunk_x() == chunk_x && ch.chunk_y() == chunk_y)
                .and_then(|ch| ch.data.items.get_mut(idx))
        }
    }

    #[test]
    fn test() {
        impl<'a> ChunkManager<Data> {
            fn all_items_mut(&mut self) -> impl Iterator<Item = &mut Cell<i32>> {
                self.chunks_iter_mut()
                    .flat_map(|ch| ch.data.items.iter_mut())
            }

            fn items_with_others(&'a mut self, chunk_pos: (i32, i32)) {
                let (one, others) = self.chunk_at_with_others_mut(chunk_pos).unwrap();
                let cl = |ch: &'a mut Chunk<Data>| &mut ch.data.items;
                let i_one = cl(one);
                let i_others = others.map(cl);
            }
        }

        let mut cm = ChunkManager::<Data>::new();

        cm.insert((0, 0), Data { items: [0, 1, 2].map(|n| n.into()).to_vec() });
        cm.insert((1, 0), Data { items: [0, 1, 2].map(|n| n.into()).to_vec() });
        cm.insert((0, 1), Data { items: [0, 1, 2].map(|n| n.into()).to_vec() });

        cm.query_each(|mut q| {
            q.for_each_with(
                |ch| &mut ch.items,
                |item, chunks| {
                    for ch in chunks.chunks_iter_mut() {}
                    for ch in chunks.iter_mut() {}

                    let (this, others) = chunks.split_first().unwrap();

                    println!("{} {} @ {item:?}", this.chunk_x(), this.chunk_y());

                    // let look_x = this.chunk_x() + 1;
                    // let look_y = this.chunk_y();

                    // if let Some(mut q) = chunks.query_one(look_x, look_y) {
                    //     q.for_each_with(|ch| &mut ch.items, |item, chunks| {
                    //         let (this, others) = chunks.split_first().unwrap();
                    //         println!("-> {} {} @ {item:?}", this.chunk_x(), this.chunk_y());
                    //         // println!("{item:?} {item2:?}");
                    //     });
                    // } else {
                    //     println!(" (no chunk at {} {})", look_x, look_y);
                    // }

                    chunks.query_each(|mut q| {
                        q.for_each_with(
                            |ch| &mut ch.items,
                            |item, chunks| {
                                let (this, others) = chunks.split_first().unwrap();
                                println!("-> {} {} @ {item:?}", this.chunk_x(), this.chunk_y());
                                // println!("{item:?} {item2:?}");
                            },
                        );
                    });
                },
            );
        });
    }

    fn test2<D>(cm: &mut ChunkManager<D>) {
        for ch in cm.chunks_iter_mut() {}

        for ch in cm.query_one((0, 0)).unwrap().chunks_iter_mut() {}
    }
}