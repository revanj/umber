pub mod sparse_set;

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
