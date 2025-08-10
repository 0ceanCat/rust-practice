use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::{BitAnd, BitOr, Range, Sub};
use crate::data_structure::roaring_bitmap::Container::{Array, Bitmap};

const ARRAY_MAX_SIZE: usize = 4096;
const BITMAP_SIZE: usize = 1024;
const U64_BITS: usize = 64;
const U64_BYTES: usize = 8;
const U16_BITS: usize = 16;
const U16_BYTES: usize = 2;
const LOW_16_BITS: u32 = 0xffff;

macro_rules! compute_u32 {
    ($key:expr, $value:expr) => {
        {
            let a: u16 = $key;
            let b: u16 = $value;
            ((a as u32) << 16)  + (b as u32)
        }
    };
}

#[derive(Clone, PartialOrd, PartialEq)]
enum Container {
    Array(ArrayContainer),
    Bitmap(BitmapContainer)
}

fn has_overlap(container_a: &Container, container_b: &Container) -> bool {
    let self_min = container_a.minimum().unwrap();
    let self_max = container_a.maximum().unwrap();
    let other_min = container_b.minimum().unwrap();
    let other_max = container_b.maximum().unwrap();

    max(self_min, other_min) <= min(self_max, other_max)
}


impl Container {
    fn add(&mut self, value: u16) -> bool {
        match self {
            Array(array_container) => {
                let added = array_container.add(value);
                if array_container.should_upgrade() {
                    let new_container = std::mem::take(array_container).upgrade();
                    *self = Bitmap(new_container);
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

    fn intersect(&self, other: &Container) -> Container {
        match self {
            Array(array_container) => {
                if has_overlap(self, other) {
                    return array_container.intersect(other)
                }
            }
            Bitmap(bitmap_container) => {
                if has_overlap(self, other) {
                    return bitmap_container.intersect(other)
                }
            }
        }
        Array(ArrayContainer::new())
    }

    fn difference(&self, other: &Container) -> Container {
        match self {
            Array(array_container) => {
                array_container.difference(other)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.difference(other)
            }
        }
    }

    fn symmetric_difference(&self, other: &Container) -> Container {
        match self {
            Array(array_container) => {
                array_container.symmetric_difference(other)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.symmetric_difference(other)
            }
        }
    }

    fn intersects(&self, other: &Container) -> bool	{
        match self {
            Array(array_container) => {
                if has_overlap(self, other) {
                    return array_container.intersects(other)
                }
            }
            Bitmap(bitmap_container) => {
                if has_overlap(self, other) {
                    return bitmap_container.intersects(other)
                }
            }
        }
        false
    }

    fn is_subset(&self, other: &Container) -> bool {
        match self {
            Array(array_container) => {
                array_container.is_subset(other)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.is_subset(other)
            }
        }
    }

    fn select(&self, idx: usize) -> Option<u16> {
        match self {
            Array(array_container) => {
                array_container.select(idx)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.select(idx)
            }
        }
    }

    fn rank(&self, value: u16) -> usize {
        match self {
            Array(array_container) => {
                array_container.rank(value)
            }
            Bitmap(bitmap_container) => {
                bitmap_container.rank(value)
            }
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd, Default)]
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

    fn upgrade(self) -> BitmapContainer {
        BitmapContainer::from_iter(self.iter())
    }

    fn batch_remove(&mut self, mut to_be_removed: Vec<usize>) {
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
        self.batch_remove(to_be_removed);
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
    
    fn intersect(&self, other: &Container) -> Container {
        match other {
            Array(other_array_container) => {
                let set: HashSet<u16> = HashSet::from_iter(self.array.iter().copied());
                let intersection: Vec<u16> = other_array_container.array
                                                                  .iter()
                                                                  .copied()
                                                                  .filter(|v| set.contains(v))
                                                                  .collect();
                Array(ArrayContainer { array: intersection })
            }
            Bitmap(other_bitmap_container) => {
                other_bitmap_container.intersect_with_array_container(&self)
            }
        }
    }

    fn difference(&self, container: &Container) -> Container {
        match container {
            Array(array_container) => {
                let set: HashSet<u16> = HashSet::from_iter(array_container.array.iter().copied());
                let difference: Vec<u16> = self.array.iter()
                                                     .copied()
                                                     .filter(|v| !set.contains(v))
                                                     .collect();
                Array(ArrayContainer { array: difference })
            }
            Bitmap(bitmap_container) => {
                let difference: Vec<u16> = self.array.iter()
                                               .copied()
                                               .filter(|v| !bitmap_container.contains(*v))
                                               .collect();
                Array(ArrayContainer { array: difference })
            }
        }
    }

    fn to_best_container(self) -> Container {
        if self.should_upgrade() {
            Bitmap(self.upgrade())
        } else {
            Array(self)
        }
    }

    fn intersects(&self, other: &Container) -> bool	{
        match other {
            Array(array_container) => {
                let max_v = min(self.minimum().unwrap(), array_container.minimum().unwrap());

                let mut self_idx = self.array.binary_search(&max_v).unwrap_or_else(|i| i);
                let mut other_idx = array_container.array.binary_search(&max_v).unwrap_or_else(|i| i);

                loop {
                    if self_idx >= self.array.len() || other_idx >= array_container.array.len() {
                        return false
                    }
                    let self_v = self.array[self_idx];
                    let other_v = array_container.array[other_idx];

                    if self_v == other_v {
                        return true
                    }

                    if self_v < other_v {
                        self_idx += 1;
                    } else {
                        other_idx += 1;
                    }

                }
            }
            Bitmap(bitmap_container) => {
                bitmap_container.intersects_with_array_container(&self)
            }
        }
    }

    fn select(&self, idx: usize) -> Option<u16> {
        self.array.get(idx).copied()
    }

    fn is_subset(&self, other: &Container) -> bool {
        match other {
            Array(array_container) => {
                if self.cardinality() > other.cardinality() {
                    return false;
                }

                let set: HashSet<u16> = array_container.array.iter().copied().collect();
                self.array.iter().copied().all(|v| set.contains(&v))
            }
            Bitmap(bitmap_container) => {
                self.array.iter().copied().all(|v| bitmap_container.contains(v))
            }
        }
    }

    fn symmetric_difference(&self, other: &Container) -> Container {
        match other {
            Array(array_container) => {
                let set_a: HashSet<u16> = HashSet::from_iter(array_container.iter());
                let set_b: HashSet<u16> = HashSet::from_iter(self.iter());
                let mut sym_diff = Vec::from_iter(set_a.symmetric_difference(&set_b).into_iter().copied());
                sym_diff.sort_unstable();

                let container = ArrayContainer {
                    array: sym_diff
                };
                container.to_best_container()
            }
            Bitmap(bitmap_container) => {
                bitmap_container.symmetric_difference_with_array_container(self)
            }
        }
    }

    fn rank(&self, value: u16) -> usize {
        self.array.binary_search(&value).unwrap_or_else(|i| i)
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct BitmapContainer {
    bitmap: Vec<u64>,
    cardinality: usize
}

impl BitmapContainer {
    pub fn new() -> BitmapContainer {
        BitmapContainer {
            bitmap: vec![0; BITMAP_SIZE],
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
        let not_exist = !self.is_one_at_position(bucket, idx_inside_bucket);
        self.bitmap[bucket] |= 1 << idx_inside_bucket;
        if not_exist {
            self.cardinality += 1;
        }
        not_exist
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

    fn is_empty(&self) -> bool {
        self.cardinality == 0
    }

    fn minimum(&self) -> Option<u16> {
        self.iter().next()
    }

    fn maximum(&self) -> Option<u16> {
        for (bucket_idx, &bucket) in self.bitmap.iter().enumerate().rev() {
            if bucket != 0 {
                let bit = (U64_BITS - 1) - bucket.leading_zeros() as usize;
                let idx = bucket_idx * U64_BITS + bit;
                return Some(idx as u16);
            }
        }
        None
    }

    fn union(&self, other: &Container) -> Container {
        match other {
            Array(other_array_container) => {
                self.union_with_array_container(other_array_container)
            }
            Bitmap(other_bitmap_container) => {
                let mut union_bitmap = vec![0; BITMAP_SIZE];
                let mut cardinality = 0;

                for i in 0..BITMAP_SIZE {
                    union_bitmap[i] = self.bitmap[i] | other_bitmap_container.bitmap[i];
                    cardinality += union_bitmap[i].count_ones();
                }

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
        array_container.array.iter()
                             .copied()
                             .for_each(|v| {union_bitmap.add(v);});
        Bitmap(union_bitmap)
    }

    fn intersect(&self, other: &Container) -> Container {
        match other {
            Array(array_container) => {
                self.intersect_with_array_container(array_container)
            }
            Bitmap(bitmap_container) => {
                let mut bitmap = Vec::with_capacity(BITMAP_SIZE);
                let mut cardinality = 0;

                for (a, b) in self.bitmap.iter().zip(&bitmap_container.bitmap) {
                    let word = a & b;
                    cardinality += word.count_ones() as usize;
                    bitmap.push(word);
                }
                let bitmap_container = BitmapContainer {
                    bitmap,
                    cardinality
                };
                if bitmap_container.should_downgrade() {
                    Array(bitmap_container.downgrade())
                } else {
                    Bitmap(bitmap_container)
                }
            }
        }
    }
    
    fn intersect_with_array_container(&self, array_container: &ArrayContainer) -> Container {
        let mut intersection = Array(ArrayContainer::new());
        for v in array_container.array.iter().copied() {
            if self.contains(v) {
                intersection.add(v);
            }
        }
        intersection
    }

    fn should_downgrade(&self) -> bool {
        self.cardinality < ARRAY_MAX_SIZE
    }

    fn downgrade(self) -> ArrayContainer {
        ArrayContainer { array: self.iter().into_iter().collect() }
    }

    fn difference(&self, other: &Container) -> Container {
        match other {
            Array(array_container) => {
                let difference: Vec<u16> = self.iter().filter(|v| !array_container.contains(*v)).collect();
                ArrayContainer { array: difference }.to_best_container()
            }
            Bitmap(bitmap_container) => {
                let mut cardinality = 0;
                let mut difference_bitmap = Vec::with_capacity(BITMAP_SIZE);
                for i in 0..BITMAP_SIZE {
                    let word = self.bitmap[i] & (!bitmap_container.bitmap[i]);
                    difference_bitmap.push(word);
                    cardinality += word.count_ones();
                }
                let bitmap_container = BitmapContainer {
                    bitmap: difference_bitmap,
                    cardinality: cardinality as usize
                };
                bitmap_container.to_best_container()
            }
        }
    }

    fn to_best_container(self) -> Container {
        if self.should_downgrade() {
            Array(self.downgrade())
        } else {
            Bitmap(self)
        }
    }

    fn intersects(&self, other: &Container) -> bool {
        match other {
            Array(array_container) => {
                return self.intersects_with_array_container(array_container)
            }
            Bitmap(bitmap_container) => {
                let max_v = max(self.minimum().unwrap(), bitmap_container.minimum().unwrap());
                let (bucket, _) = Self::find_position_in_bitmap(max_v);
                for i in bucket..BITMAP_SIZE {
                    if (self.bitmap[i] & bitmap_container.bitmap[i]) != 0 {
                        return true
                    }
                }
            }
        }
        false
    }

    fn intersects_with_array_container(&self, array_container: &ArrayContainer) -> bool {
        let max_v = max(self.minimum().unwrap(), array_container.minimum().unwrap());
        let mut other_idx = array_container.array.binary_search(&max_v).unwrap_or_else(|i| i);
        for i in other_idx..array_container.array.len() {
            if self.contains(array_container.array[i]) {
                return true;
            }
        }
        false
    }

    fn select(&self, idx: usize) -> Option<u16> {
        let mut bucket_idx = 0;
        let mut bit_idx = 0;
        let mut current_idx = 0;
        let idx: u32 = idx as u32;

        while bucket_idx < BITMAP_SIZE {
            let bucket = self.bitmap[bucket_idx];
            while bit_idx < U64_BITS && bucket != 0 {
                if current_idx + bucket.count_ones() < idx + 1{
                    current_idx += bucket.count_ones();
                    break;
                } else {
                    let bit = bit_idx;
                    bit_idx += 1;
                    if (bucket & (1u64 << bit)) != 0 {
                        if current_idx == idx {
                            return Some((bucket_idx * U64_BITS + bit) as u16);
                        }
                        current_idx += 1;
                    }
                }
            }
            bucket_idx += 1;
            bit_idx = 0;
        }
        None
    }

    fn is_subset(&self, other: &Container) -> bool {
        match other {
            Array(_) => {
                false
            }
            Bitmap(bitmap_container) => {
                if self.cardinality() > other.cardinality() {
                    return false;
                }

                for i in 0..BITMAP_SIZE {
                    let bucket = self.bitmap[i];
                    if bucket != bucket & bitmap_container.bitmap[i] {
                        return false
                    }
                }
                true
            }
        }
    }

    fn symmetric_difference(&self, other: &Container) -> Container {
        match other {
            Array(array_container) => {
                self.symmetric_difference_with_array_container(array_container)
            }
            Bitmap(bitmap_container) => {
                let mut bitmap = vec![0u64; BITMAP_SIZE];
                let mut cardinality = 0;
                for i in 0..BITMAP_SIZE {
                    bitmap[i] = self.bitmap[i] ^ bitmap_container.bitmap[i];
                    cardinality += bitmap[i].count_ones();
                }

                BitmapContainer {
                    bitmap,
                    cardinality: cardinality as usize
                }.to_best_container()
            }
        }
    }

    fn symmetric_difference_with_array_container(&self, other: &ArrayContainer) -> Container {
        let set_a: HashSet<u16> = HashSet::from_iter(other.iter());
        let mut sym_diff: Vec<u16> = self.iter()
                                         .filter(|v| !set_a.contains(v))
                                         .collect();

        set_a.into_iter()
             .filter(|v| self.contains(*v))
             .for_each(|v| sym_diff.push(v));

        sym_diff.sort_unstable();

        let container = ArrayContainer {
            array: sym_diff
        };

        container.to_best_container()
    }

    fn rank(&self, value: u16) -> usize {
        let mut smaller = 0;
        let value = value as usize;

        for (i, bucket) in self.bitmap.iter().enumerate() {
            let ones = bucket.count_ones();
            if (i + 1) * U64_BITS < value {
                smaller += ones;
            } else{
                for j in 0..(value - i * U64_BITS) {
                    if (bucket & (1 << j)) != 0  {
                        smaller += 1;
                    }
                }
                break
            }
        }
        smaller as usize
    }
}

pub struct BitmapIterator<'a> {
    bitmap: &'a Vec<u64>,
    bucket_idx: usize,
    bit_idx: usize,
}

impl<'a> BitmapIterator<'a> {
    pub fn new(bitmap: &'a Vec<u64>) -> BitmapIterator {
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
            while self.bit_idx < U64_BITS && bucket != 0 {
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
                                                     .map_or(None, |minimum| Some(compute_u32!(*key, minimum)))
                                        })
    }

    pub fn maximum(&self) -> Option<u32> {
        self.containers.last_key_value()
            .map_or(None, |(key, container)|
                {
                    container.maximum()
                             .map_or(None, |maximum| Some(compute_u32!(*key, maximum)))
                })
    }

    pub fn union(&self, other: &RoaringBitmap) -> RoaringBitmap {
        let mut union_roaring_bitmap = RoaringBitmap::new();
        self.containers.iter()
            .filter(|(key, container)| !other.containers.contains_key(key))
            .for_each(|(key, container)| {
                union_roaring_bitmap.cardinality += container.cardinality();
                union_roaring_bitmap.containers.insert(*key, container.clone());
            });

        for (key, container) in &other.containers {
            if self.containers.contains_key(key) {
                let union_container = self.containers[key].union(container);
                union_roaring_bitmap.cardinality += union_container.cardinality();
                union_roaring_bitmap.containers.insert(*key, union_container);
            } else {
                union_roaring_bitmap.cardinality += container.cardinality();
                union_roaring_bitmap.containers.insert(*key, container.clone());
            }
        }
        union_roaring_bitmap
    }

    pub fn intersection(&self, other: &RoaringBitmap) -> RoaringBitmap {
        let mut intersection_bitmap = RoaringBitmap::new();
        let keys1: HashSet<u16> = self.containers.keys().cloned().collect();
        let keys2: HashSet<u16> = other.containers.keys().cloned().collect();

        let intersection_keys: HashSet<u16> = keys1.intersection(&keys2).copied().collect();
        for v in intersection_keys {
            let intersect_container = self.containers[&v].intersect(&other.containers[&v]);
            intersection_bitmap.cardinality += intersect_container.cardinality();
            intersection_bitmap.containers.insert(v, intersect_container);
        }
        intersection_bitmap
    }

    fn split_into_key_value(number: u32) -> (u16, u16) {
        let key = (number >> U16_BITS) as u16;
        let value = (number & LOW_16_BITS) as u16;
        (key, value)
    }

    pub fn difference(&self, other: &RoaringBitmap) -> RoaringBitmap {
        let mut difference_bitmap = RoaringBitmap::new();
        for (key, container) in &self.containers {
            if !other.containers.contains_key(key) {
                difference_bitmap.containers.insert(*key, container.clone());
                difference_bitmap.cardinality += container.cardinality();
            } else {
                let difference_container = container.difference(&other.containers[key]);
                difference_bitmap.cardinality += difference_container.cardinality();
                difference_bitmap.containers.insert(*key, difference_container);
            }
        }
        difference_bitmap
    }

    pub fn iter(&self) -> RoaringBitmapIter {
        RoaringBitmapIter::new(self)
    }

    pub fn to_array(&self) -> Vec<u32> {
        let mut array: Vec<u32> = Vec::with_capacity(self.cardinality);
        for (key, container) in &self.containers {
            container.iter()
                .for_each(|v| {
                    array.push(compute_u32!(*key, v));
                });
        }
        array
    }

    pub fn intersects(&self, other: &RoaringBitmap) -> bool {
        other.containers.keys()
            .filter(|k| self.containers.contains_key(k))
            .any(|k| other.containers[k].intersects(&self.containers[k]))
    }

    pub fn select(&self, idx: usize) -> Option<u32> {
        let mut idx = idx;
        for (key, container) in &self.containers {
            if idx < container.cardinality() {
                match container.select(idx) {
                    None => {
                        return None;
                    }
                    Some(v) => {
                        return Some(compute_u32!(*key, v));
                    }
                }
            } else {
                idx -= container.cardinality();
            }
        }
        None
    }

    pub fn is_subset(&self, other: &RoaringBitmap) -> bool {
        for key in self.containers.keys() {
            if !other.containers.contains_key(&key) {
                return false;
            }
            if !self.containers[key].is_subset(&other.containers[&key]) {
                return false;
            }
        }
        true
    }

    pub fn symmetric_difference(&self, other: &RoaringBitmap) -> RoaringBitmap {
        let mut difference_bitmap = RoaringBitmap::new();
        for key in self.containers.keys() {
            if !other.containers.contains_key(&key) {
                difference_bitmap.containers.insert(*key, other.containers[&key].clone());
            } else {
                let diff = self.containers[key].symmetric_difference(&other.containers[&key]);
                difference_bitmap.containers.insert(*key, diff);
            }
        }

        for key in other.containers.keys() {
            if !self.containers.contains_key(&key) {
                difference_bitmap.containers.insert(*key, other.containers[&key].clone());
            }
        }

        difference_bitmap
    }

    pub fn rank(&self, value: u32) -> usize {
        let (target_key, value) = Self::split_into_key_value(value);
        let mut smaller = 0;

        for (key, container) in &self.containers {
            if *key < target_key {
                smaller += container.cardinality();
            } else {
                smaller += container.rank(value);
                break
            }
        }
        smaller
    }

    pub fn describe(&self) {
        let mut array_containers = 0;
        let mut bitmap_containers = 0;
        let mut space_occupied = self.containers.keys().len() * U16_BYTES;

        for container in self.containers.values() {
            match container {
                Array(array_container) => {
                    array_containers += 1;
                    space_occupied += array_container.cardinality() * U16_BYTES;
                }
                Bitmap(_) => {
                    bitmap_containers += 1;
                    space_occupied += BITMAP_SIZE * U64_BYTES;
                }
            }
        }

        println!("cardinality: {}\narray containers: {}\nbitmap containers: {}\nmin: {:?}\nmax: {:?}\nspace: {:?}", self.cardinality(), array_containers, bitmap_containers, self.minimum(), self.maximum(), space_occupied);
    }
}

pub struct RoaringBitmapIter<'a> {
    outer_iter: std::collections::btree_map::Iter<'a, u16, Container>,
    current_inner: Option<(u16, Box<dyn Iterator<Item=u16> + 'a>)>,
}

impl<'a> RoaringBitmapIter<'a> {
    pub fn new(bitmap: &'a RoaringBitmap) -> Self {
        let mut outer_iter = bitmap.containers.iter();
        let current_inner = outer_iter.next().map(|(&high_16, container)| {
            (high_16, container.iter())
        });

        RoaringBitmapIter {
            outer_iter,
            current_inner,
        }
    }
}

impl<'a> Iterator for RoaringBitmapIter<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((high_16, ref mut inner)) = self.current_inner {
                if let Some(low_16) = inner.next() {
                    return Some(compute_u32!(high_16, low_16));
                }
            }

            match self.outer_iter.next() {
                Some((&hi, container)) => {
                    self.current_inner = Some((hi, container.iter()));
                }
                None => return None
            }
        }
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

impl BitAnd for &RoaringBitmap {
    type Output = RoaringBitmap;

    fn bitand(self, rhs: &RoaringBitmap) -> RoaringBitmap {
        self.intersection(rhs)
    }
}

impl BitOr for &RoaringBitmap {
    type Output = RoaringBitmap;

    fn bitor(self, rhs: &RoaringBitmap) -> RoaringBitmap {
        self.union(rhs)
    }
}

impl Sub for &RoaringBitmap {
    type Output = RoaringBitmap;

    fn sub(self, rhs: &RoaringBitmap) -> Self::Output {
        self.difference(rhs)
    }
}