use std::any::Any;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::{Range};
use crate::data_structure::roaring_bitmap::Container::{Array, Bitmap};

const ARRAY_MAX_SIZE: usize = 4096;
const BITMAP_SIZE: usize = 1024;
const U64_BITS: usize = 64;
const U16_BITS: usize = 16;
const LOW_16_BITS: u32 = 0xffff;

#[derive(Clone, PartialOrd, PartialEq)]
enum Container {
    Array(ArrayContainer),
    Bitmap(BitmapContainer)
}

impl Container {
    fn add(&mut self, value: u16) -> bool {
        match self {
            Array(array_container) => {
                let added = array_container.add(value);
                if array_container.should_upgrade() {
                    *self = Bitmap(BitmapContainer::from_iter(array_container.iter()))
                }
                added
            }
            Bitmap(bitmap_container) => {
                bitmap_container.add(value)
            }
        }
    }

    fn remove(&mut self, value: u16) -> bool {
        match self {
            Array(array_container) => {
                array_container.remove(value)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.remove(value)
            }
        }
    }

    fn remove_values(&mut self, values: Vec<u16>) -> usize {
        match self {
            Array(array_container) => {
                array_container.remove_values(values)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.remove_values(values)
            }
        }
    }

    fn cardinality(&self) -> usize {
        match self {
            Array(array_container) => {
                array_container.cardinality()
            }
            Bitmap(bitmap_container) => {
                bitmap_container.cardinality()
            }
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item=u16> + '_> {
        match self {
            Array(array_container) => {
                array_container.iter()
            }
            Bitmap(bitmap_container) => {
                bitmap_container.iter()
            }
        }
    }

    fn contains(&self, value: u16) -> bool {
        match self {
            Array(array_container) => {
                array_container.contains(value)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.contains(value)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        match self {
            Array(array_container) => {
                array_container.as_any()
            }
            Bitmap(bitmap_container) => {
                bitmap_container.as_any()
            }
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Array(array_container) => {
                array_container.is_empty()
            }
            Bitmap(bitmap_container) => {
                bitmap_container.is_empty()
            }
        }
    }

    fn minimum(&self) -> Option<u16> {
        match self {
            Array(array_container) => {
                array_container.minimum()
            }
            Bitmap(bitmap_container) => {
                bitmap_container.minimum()
            }
        }
    }

    fn maximum(&self) -> Option<u16> {
        match self {
            Array(array_container) => {
                array_container.maximum()
            }
            Bitmap(bitmap_container) => {
                bitmap_container.maximum()
            }
        }
    }

    fn union(&self, other: &Container) -> Container {
        match self {
            Array(array_container) => {
                array_container.union(other)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.union(other)
            }
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
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

    fn union(&self, other: &Container) -> Container {
        match other {
            Array(other_array_container) => {
                let mut union_set: BTreeSet<u16> = BTreeSet::from_iter(self.array.iter().copied());
                union_set.extend(other_array_container.array.iter().copied());
                if union_set.len() >= ARRAY_MAX_SIZE {
                    Bitmap(BitmapContainer::from_iter(union_set.into_iter()))
                } else {
                    Array(ArrayContainer { array: Vec::from_iter(union_set.into_iter()) })
                }
            }
            Bitmap(other_bitmap_container) => {
                other_bitmap_container.union_with_array_container(&self)
            }
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
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

    fn union(&self, other: &Container) -> Container {
        match other {
            Array(other_array_container) => {
                self.union_with_array_container(other_array_container)
            }
            Bitmap(other_bitmap_container) => {
                let mut union_bitmap: [u64; BITMAP_SIZE] = [0; BITMAP_SIZE];
                for i in 0..BITMAP_SIZE {
                    union_bitmap[i] = self.bitmap[i] | other_bitmap_container.bitmap[i];
                }
                let cardinality: u32 = union_bitmap.iter().map(|l| l.count_ones()).sum();
                Bitmap(BitmapContainer {
                    bitmap: union_bitmap,
                    cardinality: cardinality as usize
                })
            }
        }
    }

    fn union_with_array_container(&self, array_container: &ArrayContainer) -> Container {
        let mut union_bitmap = BitmapContainer {
            bitmap: self.bitmap.clone(),
            cardinality: self.cardinality()
        };
        array_container.array.iter().for_each(|v| {union_bitmap.add(*v);});
        Bitmap(union_bitmap)
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

    pub fn union(&self, other: &RoaringBitmap) -> RoaringBitmap {
        let mut union_bitmap = RoaringBitmap::new();
        for (key, container) in &self.containers {
            let container1 = container.clone();
            union_bitmap.containers.insert(*key, container1);
        }
        for (key, container) in &other.containers {
            union_bitmap.containers.entry(*key)
                .and_modify(|other_container| {*other_container = other_container.union(&container);})
                .or_insert(container.clone());
        }
        union_bitmap.cardinality = union_bitmap.containers.iter()
                                                          .map(|(_, container)| container.cardinality())
                                                          .sum();
        union_bitmap
    }

    fn split_into_key_value(number: u32) -> (u16, u16) {
        let key = (number >> U16_BITS) as u16;
        let value = (number & LOW_16_BITS) as u16;
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