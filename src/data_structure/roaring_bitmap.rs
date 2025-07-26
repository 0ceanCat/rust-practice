use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Range};
use crate::data_structure::roaring_bitmap::Container::{Array, Bitmap};

const ARRAY_MAX_SIZE: usize = 4096;
const BITMAP_SIZE: usize = 1024;
const U64_BITS: usize = 64;
const U16_BITS: usize = 16;

enum Container {
    Array(ArrayContainer),
    Bitmap(BitmapContainer)
}

impl Container {
    fn add(&mut self, value: u16) -> bool {
        match self {
            Array(arrayContainer) => {
                let added = arrayContainer.add(value);
                if arrayContainer.should_upgrade() {
                    *self = Bitmap(BitmapContainer::from_iter(arrayContainer.iter()))
                }
                added
            }
            Bitmap(bitmap) => {
                bitmap.add(value)
            }
        }
    }

    fn remove(&mut self, value: u16) -> bool {
        match self {
            Array(arrayContainer) => {
                arrayContainer.remove(value)
            }
            Bitmap(bitmap) => {
                bitmap.remove(value)
            }
        }
    }

    fn remove_values(&mut self, values: Vec<u16>) -> usize {
        match self {
            Array(arrayContainer) => {
                arrayContainer.remove_values(values)
            }
            Bitmap(bitmap) => {
                bitmap.remove_values(values)
            }
        }
    }

    fn cardinality(&self) -> usize {
        match self {
            Array(arrayContainer) => {
                arrayContainer.cardinality()
            }
            Bitmap(bitmap) => {
                bitmap.cardinality()
            }
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item=u16> + '_> {
        match self {
            Array(arrayContainer) => {
                arrayContainer.iter()
            }
            Bitmap(bitmap) => {
                bitmap.iter()
            }
        }
    }

    fn contains(&self, value: u16) -> bool {
        match self {
            Array(arrayContainer) => {
                arrayContainer.contains(value)
            }
            Bitmap(bitmap) => {
                bitmap.contains(value)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        match self {
            Array(arrayContainer) => {
                arrayContainer.as_any()
            }
            Bitmap(bitmap) => {
                bitmap.as_any()
            }
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Array(arrayContainer) => {
                arrayContainer.is_empty()
            }
            Bitmap(bitmap) => {
                bitmap.is_empty()
            }
        }
    }

    fn minimum(&self) -> Option<u16> {
        match self {
            Array(arrayContainer) => {
                arrayContainer.minimum()
            }
            Bitmap(bitmap) => {
                bitmap.minimum()
            }
        }
    }

    fn maximum(&self) -> Option<u16> {
        match self {
            Array(arrayContainer) => {
                arrayContainer.maximum()
            }
            Bitmap(bitmap) => {
                bitmap.maximum()
            }
        }
    }
}

pub struct ArrayContainer {
    array: Vec<u16>
}

impl ArrayContainer {
    pub fn new() -> ArrayContainer {
        ArrayContainer {
            array: Vec::new()
        }
    }

    fn should_upgrade(&self) -> bool {
        self.cardinality() >= ARRAY_MAX_SIZE
    }

    fn fast_remove(&mut self, mut to_be_removed: Vec<usize>) {
        if !to_be_removed.is_empty() {
            to_be_removed.sort_unstable_by(|a, b| b.cmp(a));

            let mut set_ref = &mut to_be_removed;
            let start = set_ref.pop().unwrap();
            let mut offset = 1;
            let mut idx = start;
            while idx + offset < self.array.len() {
                if let Some(next_idx_to_be_removed) = set_ref.last() {
                    if *next_idx_to_be_removed - offset == idx {
                        offset += 1;
                        set_ref.pop();
                        continue
                    }
                }
                self.array[idx] = self.array[idx + offset];
                idx += 1;
            }
            self.array.truncate(idx);
        }
    }

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

    fn remove(&mut self, value: u16) -> bool {
        self.array.binary_search(&value)
                  .map_or(false, |idx| {
                      self.array.remove(idx);
                      true
                  })
    }

    fn remove_values(&mut self, values: Vec<u16>) -> usize{
        let mut to_be_removed: Vec<usize> = Vec::new();
        values.iter()
              .map(|v| self.array.binary_search(&v))
              .filter(|p| p.is_ok())
              .for_each(|p| to_be_removed.push(p.unwrap()));
        let removed = to_be_removed.len();
        self.fast_remove(to_be_removed);
        removed
    }

    fn cardinality (&self) -> usize {
        self.array.len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = u16> + '_> {
        Box::new(self.array.iter().copied())
    }

    fn contains(&self, value: u16) -> bool {
        self.array.binary_search(&value).is_ok()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }

    fn minimum(&self) -> Option<u16> {
        self.array.first().copied()
    }

    fn maximum(&self) -> Option<u16> {
        self.array.last().copied()
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

    fn is_one_at_position(&self, bucket: usize, idx_inside_bucket: usize) -> bool {
        self.bitmap[bucket] & (1 << idx_inside_bucket) != 0
    }

    fn add(&mut self, value: u16) -> bool {
        let (bucket, idx_inside_bucket) = Self::find_position_in_bitmap(value);
        let result = self.bitmap[bucket] & (1 << idx_inside_bucket) == 0;
        self.bitmap[bucket] |= 1 << idx_inside_bucket;
        if result {
            self.cardinality += 1;
        }
        result
    }

    fn remove(&mut self, value: u16) -> bool {
        let (bucket, idx_inside_bucket) = Self::find_position_in_bitmap(value);
        if self.is_one_at_position(bucket, idx_inside_bucket) {
            self.bitmap[bucket] &= !(1 << idx_inside_bucket);
            return true
        }
        false
    }

    fn remove_values(&mut self, values: Vec<u16>) -> usize {
        values.into_iter().filter(|v| self.remove(*v)).count()
    }

    fn cardinality(&self) -> usize {
        self.cardinality
    }

    fn iter(&self) -> Box<dyn Iterator<Item=u16> + '_> {
        Box::new(BitmapIterator::new(&self.bitmap))
    }

    fn contains(&self, key: u16) -> bool {
        let (bucket, idx_inside_bucket) = Self::find_position_in_bitmap(key);
        self.is_one_at_position(bucket, idx_inside_bucket)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_empty(&self) -> bool {
        self.cardinality == 0
    }

    fn minimum(&self) -> Option<u16> {
        self.iter().next()
    }

    fn maximum(&self) -> Option<u16> {
        let mut back_bucket_idx = BITMAP_SIZE - 1;
        let mut back_bit_idx = U64_BITS - 1;
        while back_bucket_idx >= 0 {
            let bucket = self.bitmap[back_bucket_idx];
            while back_bit_idx >= 0 {
                let bit = back_bit_idx;
                back_bit_idx -= 1;
                if (bucket & (1u64 << bit)) != 0 {
                    return Some((back_bucket_idx * U64_BITS + bit) as u16);
                }
            }
            back_bucket_idx -= 1;
            back_bit_idx = U64_BITS - 1;
        }
        None
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
            bit_idx: 0
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
    containers: BTreeMap<u16, Container>,
    cardinality: usize
}

impl RoaringBitmap {
    pub fn new() -> RoaringBitmap {
        RoaringBitmap {
            containers: BTreeMap::new(),
            cardinality: 0
        }
    }

    pub fn add(&mut self, number: u32) -> bool {
        let (key, value) = Self::split_into_key_value(number);

        let mut added = false;
        match self.containers.get_mut(&key) {
            None => {
                let mut array_container = ArrayContainer::new();
                added = array_container.add(value);
                self.containers.insert(key, Array(array_container));
            }
            Some(container) => {
                added = container.add(value)
            }
        };

        if added {
            self.cardinality += 1;
        }
        added
    }

    pub fn remove(&mut self, number: u32) -> bool {
        let (key, value) = Self::split_into_key_value(number);
        if self.containers.get_mut(&key)
                       .map_or(false, |container| {container.remove(value)}) {
            self.cardinality -= 1;
            return true
        }
        false
    }

    pub fn remove_range(&mut self, range: Range<u32>) {
        let mut key_values_map: HashMap<u16, Vec<u16>> = HashMap::new();
        range.map(Self::split_into_key_value)
             .for_each(|(key, value)| key_values_map.entry(key).or_default().push(value));

        for key_values in key_values_map {
            match self.containers.get_mut(&key_values.0) {
                None => {}
                Some(container) => {
                    self.cardinality -= container.remove_values(key_values.1);
                }
            }
        }
    }

    pub fn cardinality(&self) -> usize {
        self.cardinality
    }

    pub fn contains(&self, number: u32) -> bool {
        let (key, value) = Self::split_into_key_value(number);
        self.containers.get(&key)
            .map_or(false, |container| container.contains(value))

    }

    pub fn from_range(range: Range<u32>) -> RoaringBitmap {
        let mut roaring_bitmap = RoaringBitmap::new();

        for x in range {
            roaring_bitmap.add(x);
        }

        roaring_bitmap
    }

    pub fn minimum(&self) -> Option<u32> {
        self.containers.first_key_value()
                       .map_or(None, |(key, container)|
                                        {
                                            container.minimum()
                                                     .map_or(None, |minimum| Some(((*key as u32) << 16) + minimum as u32))
                                        })
    }

    pub fn maximum(&self) -> Option<u32> {
        self.containers.last_key_value()
            .map_or(None, |(key, container)|
                {
                    container.maximum()
                             .map_or(None, |maximum| Some(((*key as u32) << 16) + maximum as u32))
                })
    }

    fn split_into_key_value(number: u32) -> (u16, u16) {
        let key = (number >> U16_BITS) as u16;
        let value = (number & 0xffff) as u16;
        (key, value)
    }
}

impl FromIterator<u32> for RoaringBitmap {
    fn from_iter<T: IntoIterator<Item=u32>>(mut iter: T) -> Self {
        let mut vec: Vec<u32> = iter.into_iter().collect();
        vec.sort();

        let mut roaring_bitmap = RoaringBitmap::new();

        for v in vec {
            roaring_bitmap.add(v);
        }

        roaring_bitmap
    }
}