use crate::data_structure::roaring_bitmap::{RoaringBitmap};

#[test]
fn test1_add_one_by_one() {
    let mut rb = RoaringBitmap::new();

    for x in  0..=1 << 16 {
        rb.add(x);
    }

    assert_eq!((1 << 16) + 1, rb.cardinality());
}

#[test]
fn test2_add_by_range() {
    let rb = RoaringBitmap::from_iter((0..=1 << 16).into_iter());
    assert_eq!((1 << 16) + 1, rb.cardinality());
}

#[test]
fn test3_add_and_remove() {
    let mut rb = RoaringBitmap::new();

    assert_eq!(true, rb.add(10));
    assert_eq!(true, rb.add(70000));
    assert_eq!(false, rb.add(70000));


    assert_eq!(true, rb.contains(10));
    assert_eq!(true, rb.contains(70000));
    assert_eq!(false, rb.contains(70001));
    assert_eq!(2, rb.cardinality());

    assert_eq!(true, rb.remove(10));
    assert_eq!(true, rb.remove(70000));
    assert_eq!(false, rb.remove(70000));
    assert_eq!(false, rb.remove(70001));
    assert_eq!(0, rb.cardinality());
}

#[test]
fn test4_add_and_remove_range() {
    let mut rb = RoaringBitmap::from_range(0.. 1 << 17);

    assert_eq!(true, rb.contains(10));
    assert_eq!(true, rb.contains(1 << 16));
    assert_eq!(false, rb.contains(1 << 17));
    assert_eq!(1 << 17, rb.cardinality());

    rb.remove_range(0..1<<16);
    assert_eq!((1 << 17) - (1 << 16), rb.cardinality());

    assert_eq!(false, rb.contains(10));
    assert_eq!(false, rb.contains((1 << 16) - 1));
}

#[test]
fn test5_get_minimum_maximum() {
    let rb = RoaringBitmap::new();
    assert_eq!(None, rb.minimum());
    assert_eq!(None, rb.maximum());


    let rb = RoaringBitmap::from_iter([1]);
    assert_eq!(Some(1), rb.minimum());
    assert_eq!(Some(1), rb.maximum());

    let rb = RoaringBitmap::from_range(2.. 1 << 17);
    assert_eq!(Some(2), rb.minimum());
    assert_eq!(Some((1 << 17) - 1), rb.maximum());
}

#[test]
fn test6_union() {
    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter(2..1<<17);
    let rb3 = &rb1 | &rb2;
    assert_eq!(1 << 17, rb3.cardinality());
    assert_eq!(Some(0), rb3.minimum());
    assert_eq!(Some((1 << 17) - 1), rb3.maximum());


    let rb1 = RoaringBitmap::from_iter(1..1<<17);
    let rb2 = RoaringBitmap::from_iter(2..1<<17);
    let rb3 = &rb1 | &rb2;
    assert_eq!((1 << 17) - 1, rb3.cardinality());
    assert_eq!(Some(1), rb3.minimum());
    assert_eq!(Some((1 << 17) - 1), rb3.maximum());
}

#[test]
fn test7_intersection() {
    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter([1,2,3]);
    let rb3 = &rb1 & &rb2;
    assert_eq!(3, rb3.cardinality());
    assert_eq!(Some(1), rb3.minimum());
    assert_eq!(Some(3), rb3.maximum());

    let rb1 = RoaringBitmap::from_iter(1..1<<17);
    let rb2 = RoaringBitmap::from_iter(2..1<<17);
    let rb3 = &rb1 & &rb2;
    assert_eq!(rb2.cardinality(), rb3.cardinality());
    assert_eq!(Some(2), rb3.minimum());
    assert_eq!(Some((1 << 17) - 1), rb3.maximum());

    let rb1 = RoaringBitmap::from_iter([1,2,3]);
    let rb2 = RoaringBitmap::from_iter(2..1<<17);
    let rb3 = &rb1 & &rb2;
    assert_eq!(2, rb3.cardinality());
    assert_eq!(Some(2), rb3.minimum());
    assert_eq!(Some(3), rb3.maximum());
}

#[test]
fn test8_difference() {
    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter([1,2,3]);
    let rb3 = &rb1 - &rb2;
    assert_eq!(vec![0, 4, 5, 6, 7], rb3.to_array());
    assert_eq!(5, rb3.cardinality());
    assert_eq!(Some(0), rb3.minimum());
    assert_eq!(Some(7), rb3.maximum());


    let rb1 = RoaringBitmap::from_iter(0..1 << 17);
    let rb2 = RoaringBitmap::from_iter(1..(1 << 17) - 1);
    let rb3 = &rb1 - &rb2;
    assert_eq!(vec![0, (1 << 17) - 1], rb3.to_array());
    assert_eq!(2, rb3.cardinality());
    assert_eq!(Some(0), rb3.minimum());
    assert_eq!(Some((1 << 17) - 1), rb3.maximum());
}

#[test]
fn test9_iter() {
    let rb = RoaringBitmap::from_iter(0..8);
    for (idx, v) in rb.iter().enumerate() {
        assert_eq!(idx as u32, v);
    }
}

#[test]
fn test10_intersects() {
    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter([1,2,3]);
    assert_eq!(true, rb1.intersects(&rb2));

    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter([9,10,11]);
    assert_eq!(false, rb1.intersects(&rb2));

    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter(10..1<<18);
    assert_eq!(false, rb1.intersects(&rb2));

    let rb1 = RoaringBitmap::from_iter(1<<19..1<<20);
    let rb2 = RoaringBitmap::from_iter(10..1<<18);
    assert_eq!(false, rb1.intersects(&rb2));

    let rb1 = RoaringBitmap::from_iter(0..1<<17);
    let rb2 = RoaringBitmap::from_iter(10..1<<18);
    assert_eq!(true, rb1.intersects(&rb2));
}

#[test]
fn test11_select() {
    let rb = RoaringBitmap::from_iter(0..8);
    assert_eq!(Some(0), rb.select(0));
    assert_eq!(Some(4), rb.select(4));
    assert_eq!(None, rb.select(8));

    let rb = RoaringBitmap::from_iter(1..1 << 17);
    assert_eq!(Some(1), rb.select(0));
    assert_eq!(Some(2), rb.select(1));
    assert_eq!(Some(70001), rb.select(70000));
    assert_eq!(None, rb.select(1 << 17));
}

#[test]
fn test12_is_subset() {
    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter(1..9);
    assert_eq!(false, rb1.is_subset(&rb2));
    assert_eq!(false, rb2.is_subset(&rb1));

    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter(2..7);
    assert_eq!(false, rb1.is_subset(&rb2));
    assert_eq!(true, rb2.is_subset(&rb1));

    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter(0..1<<17);
    assert_eq!(true, rb1.is_subset(&rb2));
    assert_eq!(false, rb2.is_subset(&rb1));

    let rb1 = RoaringBitmap::from_iter(1..1<<18);
    let rb2 = RoaringBitmap::from_iter(0..1<<17);
    assert_eq!(false, rb1.is_subset(&rb2));
    assert_eq!(false, rb2.is_subset(&rb1));

    let rb1 = RoaringBitmap::from_iter(2..1<<17);
    let rb2 = RoaringBitmap::from_iter(0..1<<18);
    assert_eq!(true, rb1.is_subset(&rb2));
    assert_eq!(false, rb2.is_subset(&rb1));
}

#[test]
fn test13_symmetric_diff() {
    let rb1 = RoaringBitmap::from_iter(0..8);
    let rb2 = RoaringBitmap::from_iter(1..9);
    let rb3 = rb1.symmetric_difference(&rb2);

    assert_eq!(vec![0,1,2,3,4,5,6,7], rb1.to_array());
    assert_eq!(vec![0, 8], rb3.to_array());


    let rb1 = RoaringBitmap::from_iter(0..1<<17);
    let rb2 = RoaringBitmap::from_iter(1..1<<17);
    let rb3 = rb1.symmetric_difference(&rb2);
    assert_eq!(vec![0], rb3.to_array());
}

#[test]
fn test14_rank() {
    let rb = RoaringBitmap::from_iter(0..8);
    assert_eq!(8, rb.rank(8));
    assert_eq!(1, rb.rank(1));
    assert_eq!(5, rb.rank(5));
    assert_eq!(8, rb.rank(100));


    let rb = RoaringBitmap::from_iter(0..1<<17);
    assert_eq!(8, rb.rank(8));
    assert_eq!(1, rb.rank(1));
    assert_eq!(5, rb.rank(5));
    assert_eq!(100, rb.rank(100));
    assert_eq!(70000, rb.rank(70000));
}