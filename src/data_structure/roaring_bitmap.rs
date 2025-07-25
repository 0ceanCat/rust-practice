use std::collections::BTreeMap;

pub trait Container {}

pub struct ArrayContainer {
    array: Vec<u16>
}

impl ArrayContainer {
    const MAX_SIZE: usize = 2;
}

impl Container for ArrayContainer {}

pub struct BitMapContainer {
    bitmap: [u64; 1024],
}

impl Container for BitMapContainer {}

pub struct RoaringBitmap {
    pub(crate) containers: BTreeMap<u16, Box<dyn Container>>
}