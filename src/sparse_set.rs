use std::mem::{self, MaybeUninit};
use std::{marker::PhantomData, ptr::NonNull};
use rj::AsAny;
use crate::Entity;
use crate::EcsContainer;
use crate::DynEcsContainer;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub(crate) struct GenerationalIndex(u32);
impl GenerationalIndex {
    pub fn new(idx: u32, generation: u16) -> Self {
        let idx_top_12_bits = 0xFFF00000 & idx;
        let generation_top_4_bits = 0xF000 & generation;
        assert_eq!(idx_top_12_bits, 0);
        assert_eq!(generation_top_4_bits, 0);

        let new_id = idx | (generation as u32) << 20;

        Self(new_id)
    }

    pub fn null() -> Self {
	    Self::new(0xFFFFF, 0xFFF)
    }

    pub fn index(&self) -> usize { (self.0 & 0xFFFFF) as usize }
    pub fn index_32(&self) -> u32{ self.0 & 0xFFFFF }
    pub fn generation(&self) -> u16{ (self.0 >> 20) as u16 }
}

impl std::fmt::Display for GenerationalIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	    write!(f, "(Generation {}, ID {})", self.generation(), self.index_32())
    }
}
impl Default for GenerationalIndex {
    fn default() -> Self {
        Self::null()
    }
}


pub struct SparseSet<T> {
    sparse: Vec<GenerationalIndex>,
    dense: Vec<(GenerationalIndex, MaybeUninit<T>)>,
    total: usize,
} impl<T> SparseSet<T> {
    pub fn new() -> Self { Self { sparse: Vec::new(), dense: Vec::new(), total: 0 }}

    pub(crate) fn insert(&mut self, index: GenerationalIndex, value: T) {
        if self.contains(index) {
            unsafe {*self.dense[self.sparse[index.index()].index()].1.assume_init_mut() = value};
            return;
        }
        let dense_location = self.total;
        assert!(self.dense.len() >= dense_location);
        if dense_location >= self.dense.len() {
	        self.dense.push((index, MaybeUninit::new(value)));
        } else {
            self.dense[dense_location].0 = index;
            unsafe {*self.dense[dense_location].1.assume_init_mut() = value};
        }

	    let sparse_location = index.index() as usize;

	    self.sparse.reserve(sparse_location);

	    while self.sparse.len() <= sparse_location {
	        self.sparse.push(GenerationalIndex::null());
	    }
	    self.sparse[sparse_location] = GenerationalIndex::new(dense_location as u32, index.generation());
	    self.total += 1;
    }

    pub fn remove(&mut self, index: GenerationalIndex) {
        if !self.contains(index) { return; }

        let sparse_index = index.index();
        let dense_index = self.sparse[sparse_index];
        let dense_location = dense_index.index();
        let last_element = mem::replace(
                &mut self.dense[self.total-1], 
                (GenerationalIndex::null(), MaybeUninit::uninit()));
        println!("self.total is {}, last element has index of {}", self.total, last_element.0);
        self.sparse[last_element.0.index()] = 
            GenerationalIndex::new(dense_index.index_32(), last_element.0.generation());
        self.dense[dense_location] = last_element;
        self.total -= 1;
    }

    fn contains(&self, index: GenerationalIndex) -> bool {
        if self.total == 0 { return false; }
        let sparse_index = index.index();
        if self.sparse.len() <= sparse_index { return false; }
        let dense_index = self.sparse[sparse_index];
        if dense_index == GenerationalIndex::null() { return false; }
        let dense_index = dense_index.index();
        if dense_index >= self.dense.len() { return false }
        if self.dense[dense_index].0 != index { return false; }

        true
    }

    unsafe fn get_unchecked(&self, index: GenerationalIndex) -> &T {
        let sparse_index = index.index();
        let dense_index = self.sparse[sparse_index].index();

        unsafe {self.dense[dense_index].1.assume_init_ref()}
    }

    unsafe fn get_mut_unchecked(&mut self, index: GenerationalIndex) -> &mut T {
        let sparse_index = index.index();
        let dense_index = self.sparse[sparse_index].index();

        unsafe {self.dense[dense_index].1.assume_init_mut()}
    }

    pub(crate) fn get(&self, index: GenerationalIndex) -> Option<&T> {
        if !self.contains(index) { None }
        else { Some( unsafe {self.get_unchecked(index)}) }
    }

    pub(crate) fn get_mut(&mut self, index: GenerationalIndex) -> Option<&mut T> {
        if !self.contains(index) { None }
        else { Some( unsafe {self.get_mut_unchecked(index)}) }
    }

    pub(crate) fn get_disjoint_mut<const N: usize>(&mut self, indices: [GenerationalIndex; N]) -> [&mut T; N] {
        let mut usize_indices: [usize; N] = [0; N];
        for i in 0..N { usize_indices[i] = indices[i].index(); }
        let dense_indices: [&mut GenerationalIndex; N] = self.sparse.get_disjoint_mut(usize_indices).unwrap();
        for i in 0..N { usize_indices[i] = dense_indices[i].index(); }

        self.dense.get_disjoint_mut(usize_indices).unwrap().map(|x|  unsafe {x.1.assume_init_mut()})
    }

    pub fn len(&self) -> usize {
        self.total
    }

    fn indices(&self) -> IndexIterator<'_, T> {
        IndexIterator { container: self, idx: 0 }
    }

    fn iter(&self) -> DenseIterator<'_, T> {
        DenseIterator { container: self, idx: 0 }
    }

    fn iter_mut(&mut self) -> DenseMutIterator<'_, T> {
        DenseMutIterator {
            ptr: NonNull::from(self.dense.as_mut_slice()).cast(),
            idx: 0,
            size: self.total,
            _phantom: PhantomData,
        }
    }
}

pub(crate) struct IndexIterator<'a, T> {
    container: &'a SparseSet<T>,
    idx: usize,
}
impl <'a, T> Iterator for IndexIterator<'a, T> {
    type Item = GenerationalIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.container.len() {
            let ret = Some(self.container.sparse[self.container.dense[self.idx].0.index()]);
            self.idx += 1;
            ret
        }
        else { None }
    }
}
pub(crate) struct EntityIterator<'a, T> {
    container: &'a SparseSet<T>,
    idx: usize,
}
impl <'a, T> Iterator for EntityIterator<'a, T> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.container.len() {
            let ret = Some(Entity::from(self.container.sparse[self.container.dense[self.idx].0.index()]));
            self.idx += 1;
            ret
        }
        else { None }
    }
}
impl <'a, T> From<IndexIterator<'a, T>> for EntityIterator<'a, T> {
    fn from(value: IndexIterator<'a, T>) -> Self {
        Self {
            container: value.container,
            idx: value.idx
        }
    }
}

pub(crate) struct EntityComponentIterator<'a, T> {
    container: &'a SparseSet<T>,
    idx: usize,
}
impl <'a, T> Iterator for EntityComponentIterator<'a, T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.container.len() {
            let ret = Some((
                Entity::from(self.container.sparse[self.container.dense[self.idx].0.index()]),
                unsafe {self.container.dense[self.idx].1.assume_init_ref()}
            ));
            self.idx += 1;
            ret
        }
        else { None }
    }
}
impl <'a, T> From<DenseIterator<'a, T>> for EntityComponentIterator<'a, T> {
    fn from(value: DenseIterator<'a, T>) -> Self {
        Self {
            container: value.container,
            idx: value.idx
        }
    }
}

pub(crate) struct EntityComponentMutIterator<'a, T> {
    ptr: NonNull<(GenerationalIndex, T)>,
    idx: usize,
    size: usize,
    _phantom: PhantomData<&'a mut T>
}
impl <'a, T> Iterator for EntityComponentMutIterator<'a, T> {
    type Item = (Entity, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.idx < self.size {
                let ret = Some((
                    Entity::from((*self.ptr.as_ptr().add(self.idx)).0),
                    &mut(*self.ptr.as_ptr().add(self.idx)).1
                ));
                self.idx += 1;
                ret
            } else { None }
        }
    }
}
impl <'a, T> From<DenseMutIterator<'a, T>> for EntityComponentMutIterator<'a, T> {
    fn from(value: DenseMutIterator<'a, T>) -> Self {
        Self {
            ptr: value.ptr,
            idx: value.idx,
            size: value.size,
            _phantom: PhantomData,
        }
    }
}

pub struct DenseIterator<'a, T> {
    container: &'a SparseSet<T>,
    idx: usize,
}

impl<'a, T> Iterator for DenseIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.container.len() {
            let ret = Some(unsafe {self.container.dense[self.idx].1.assume_init_ref()});
            self.idx += 1;
            ret
        } else { None }
    }
}

pub struct DenseMutIterator<'a, T> {
    ptr: NonNull<(GenerationalIndex, T)>,
    idx: usize,
    size: usize,
    _phantom: PhantomData<&'a mut T>
}


impl<'a, T> Iterator for DenseMutIterator<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.size {
            let ret = Some( unsafe { &mut(*self.ptr.as_ptr().add(self.idx)).1 });
            self.idx += 1;
            ret
        } else { None }
    }
}

impl<'a, T> IntoIterator for &'a SparseSet<T> {
    type Item = &'a T;
    type IntoIter = DenseIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SparseSet<T> {
    type Item = &'a mut T;
    type IntoIter = DenseMutIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}


impl<T: 'static> AsAny for SparseSet<T> {
    fn as_any(self: &Self) -> &dyn std::any::Any { self }
    fn as_any_mut(self: &mut Self) -> &mut dyn std::any::Any { self }
}

impl<T: 'static> DynEcsContainer for SparseSet<T> {
    fn len(&self) -> usize {
        self.total
    }
    fn contains_entity(&self, entity: Entity) -> bool {
        self.contains(entity.index())
    }
}


impl<T: 'static> EcsContainer for SparseSet<T> {
    type Item = T;

    fn entities(&self) -> impl Iterator<Item=Entity> {
        EntityIterator::from(self.indices())
    }
    fn get(&self, ett: Entity) -> Option<&Self::Item> {
        self.get(ett.index())
    }

    fn get_mut(&mut self, ett: Entity) -> Option<&mut Self::Item> {
        self.get_mut(ett.index())
    }

    fn components(&mut self) -> impl Iterator<Item=&Self::Item> {
        self.iter()
    }

    fn components_mut(&mut self) -> impl Iterator<Item=&mut Self::Item> {
        self.iter_mut()
    }
    fn entities_components(&self) -> impl Iterator<Item=(Entity, &Self::Item)> {
        EntityComponentIterator::from(self.iter())
    }
    fn entities_components_mut(&mut self) -> impl Iterator<Item=(Entity, &mut Self::Item)> {
        EntityComponentMutIterator::from(self.iter_mut())
    }
}
