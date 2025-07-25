use std::any::Any;
use std::collections::BTreeMap;

pub trait Container: Any {
    fn add(&mut self, value: u16) -> bool;

    fn contains(&self, value: u16) -> bool;

    fn cardinality (&self) -> usize;

    fn iter(&self) -> Box<dyn Iterator<Item = u16> + '_>;

    fn as_any(&self) -> &dyn Any;
}

const ARRAY_MAX_SIZE: usize = 4096;
const BITMAP_SIZE: usize = 1024;
const U64_BITS: usize = 64;

pub struct ArrayContainer {
    array: Vec<u16>
}

impl ArrayContainer {
    pub fn new() -> ArrayContainer {
        ArrayContainer {
            array: Vec::new()
        }
    }
}

impl Container for ArrayContainer {
    fn add(&mut self, value: u16) -> bool {
        match self.array.binary_search(&value) {
            Ok(_) => {
                false
            }
            Err(idx) => {
                self.array.insert(idx, value);
                true
            }
        }
    }

    fn contains(&self, value: u16) -> bool {
        self.array.binary_search(&value).is_ok()
    }

    fn cardinality (&self) -> usize {
        self.array.len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = u16> + '_> {
        Box::new(self.array.iter().copied())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct BitmapContainer {
    bitmap: [u64; BITMAP_SIZE],
    cardinality: usize
}

impl BitmapContainer {
    pub fn new() -> BitmapContainer {
        BitmapContainer {
            bitmap: [0; BITMAP_SIZE],
            cardinality: 0
        }
    }

    pub fn from_iter(iter: impl Iterator<Item=u16>) -> BitmapContainer {
        let mut bitmap_container = BitmapContainer::new();
        for x in iter {
            bitmap_container.add(x);
        }
        bitmap_container
    }

    fn find_position_in_bitmap(key: u16) -> (usize, usize) {
        let bucket = key as usize / U64_BITS;
        let idx_inside_bucket = key as usize % U64_BITS;
        (bucket, idx_inside_bucket)
    }
}

impl Container for BitmapContainer {
    fn add(&mut self, key: u16) -> bool {
        let (bucket, idx_inside_bucket) = Self::find_position_in_bitmap(key);
        let result = self.bitmap[bucket] & (1 << idx_inside_bucket) == 0;
        self.bitmap[bucket] |= 1 << idx_inside_bucket;
        if result {
            self.cardinality += 1;
        }
        result
    }

    fn contains(&self, key: u16) -> bool {
        let (bucket, idx_inside_bucket) = Self::find_position_in_bitmap(key);
        self.bitmap[bucket] & (1 << idx_inside_bucket) != 0
    }

    fn cardinality(&self) -> usize {
        self.cardinality
    }

    fn iter(&self) -> Box<dyn Iterator<Item=u16> + '_> {
        Box::new(BitmapIterator::new(&self.bitmap))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct BitmapIterator<'a> {
    bitmap: &'a [u64; BITMAP_SIZE],
    bucket_idx: usize,
    bit_idx: usize,
}

impl<'a> BitmapIterator<'a> {
    pub fn new(bitmap: &'a [u64; BITMAP_SIZE]) -> BitmapIterator {
        BitmapIterator {
            bitmap,
            bucket_idx: 0,
            bit_idx: 0,
        }
    }
}

impl<'a> Iterator for BitmapIterator<'a> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        while self.bucket_idx < BITMAP_SIZE {
            let bucket = self.bitmap[self.bucket_idx];
            while self.bit_idx < U64_BITS {
                let bit = self.bit_idx;
                self.bit_idx += 1;
                if (bucket & (1u64 << bit)) != 0 {
                    return Some((self.bucket_idx * U64_BITS + bit) as u16);
                }
            }
            self.bucket_idx += 1;
            self.bit_idx = 0;
        }
        None
    }
}


pub struct RoaringBitmap {
    pub(crate) containers: BTreeMap<u16, Box<dyn Container>>
}

impl RoaringBitmap {
    pub fn new() -> RoaringBitmap {
        RoaringBitmap {
            containers: BTreeMap::new()
        }
    }

    pub fn insert(&mut self, number: u32) -> bool {
        let key = (number >> 16) as u16;
        let value = (number & 0xffff) as u16;
        match self.containers.get_mut(&key) {
            None => {
                let mut array_container = ArrayContainer::new();
                let result = array_container.add(value);
                self.containers.insert(key, Box::new(array_container));
                result
            }
            Some(container) => {
                if let Some(array_container) = container.as_any().downcast_ref::<ArrayContainer>() {
                    let mut bitmap = BitmapContainer::from_iter(array_container.iter());
                    let result = bitmap.add(value);
                    self.containers.insert(key, Box::new(bitmap));
                    result
                } else {
                    container.add(value)
                }
            }
        }
    }
}