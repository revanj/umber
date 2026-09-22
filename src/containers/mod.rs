use rj::AsAny;

use crate::Entity;

pub mod sparse_set;
pub mod global;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct GenerationalIndex(u32);
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

pub trait DynEcsContainer: AsAny {
    fn is_resource(&self) -> bool;
    fn as_component(&mut self) -> Option<&mut dyn DynEcsEntityContainer>;
}

pub trait EcsContainer: DynEcsContainer {
    type Item;
}

pub trait EcsGlobalContainer: EcsContainer {
    fn get(&self) -> &Self::Item;
    fn get_mut(&mut self) -> &mut Self::Item;
}

pub trait DynEcsEntityContainer: DynEcsContainer {
    fn len(&self) -> usize; 
    fn contains_entity(&self, entity: Entity) -> bool;
    fn entities_vec(&self) -> Vec<Entity>;
}

pub trait EcsEntityContainer: DynEcsEntityContainer  + EcsContainer {
    fn entities(&self) -> impl Iterator<Item=Entity>;
    fn get(&self, ett: Entity) -> Option<&Self::Item>;
    fn get_mut(&mut self, ett: Entity) -> Option<&mut Self::Item>;
    fn components(&mut self) -> impl Iterator<Item=&Self::Item>; 
    fn components_mut(&mut self) -> impl Iterator<Item=&mut Self::Item>; 
    fn entities_components(&self) -> impl Iterator<Item=(Entity, &Self::Item)>;
    fn entities_components_mut(&mut self) -> impl Iterator<Item=(Entity, &mut Self::Item)>;
}
