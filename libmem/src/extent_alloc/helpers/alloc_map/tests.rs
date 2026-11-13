
use libtest::{TestRng, dbg, print, println};
use crate::misc::Alignment;
use core::num::NonZero;

use crate::AlignedNonNull;

use crate::extent_alloc::extent::{Extent, MutableExtent};
use crate::extent_alloc::helpers::{AllocMap, SuitableExtent};
use crate::extent_alloc::{
    extent::{TakenExtent, InternalExtent, RawExtent},
    layout::{test_layout::TestLayout, LayoutDescriptor},
    raw_alloc::RawExtentAlloc,
};

const MIN_EXT_COUNT: usize = 128;
const MAX_EXT_COUNT: usize = 256;



use alloc::vec::Vec;


#[test]
fn find_suitable_extent() {

    let mut rand = TestRng::new();
    let mut paths_taken: usize = 0;

    for _ in 0..500 {

        let count = rand.next_range(256..512);

        let exts = random_extents::<2048>(count, &mut rand);

        let mut alloc = RawExtentAlloc::uninit();
        let init_result = unsafe { alloc.initialize(exts.iter().cloned()) };
        assert!(matches!(init_result, Ok(())));

        let (free, _) = distinguish_free_and_used(exts);




        for _ in 0..128 {

            let layout = TestLayout::from_pages(NonZero::new(rand.next_range(512..4096) as u64).unwrap()).unwrap();

            //  find any suitable extent
            match alloc.free_map().find_suitable(&layout) {
                Some(SuitableExtent { suitable, allocated }) => {

                    paths_taken |= 0b01;

                    assert!(allocated.fits_into(&suitable));

                    assert!(allocated.address().alignment() >= layout.align());
                    assert!(allocated.size() >= layout.size());

                    let was_found = free.iter().find(|ext| ext.address() == suitable.address() && ext.size() == suitable.size() );

                    assert!(matches!(was_found, Some(_)), "suitable extent was found manually but was not found by the tested function");
                },
                None => {   //  assert that any valid extent does not exist

                    paths_taken |= 0b10;

                    let was_found = free.iter().find(|suitable| {

                        //  align the address up to `layout.align()` to figure out where would the extent start
                        let aligned_up = /*unsafe*/ {
                            //  safety: align_up() always return a number larger than the input
                            NonZero::new/*_unchecked*/(suitable.address().as_nullable().align_up(layout.align().get() as usize)).unwrap()
                        };

                        let allocated = InternalExtent::new(AlignedNonNull::new(aligned_up).unwrap(), layout.size());

                        allocated.fits_into(&suitable)
                    } );

                    assert!(matches!(was_found, None));
                }
            }
        }

    }

    assert_eq!(paths_taken, 0b11);

}



#[ignore = "TODO"]
#[test]
fn insert_defragment() {


    todo!();


    /*//  free = | Ext | <space> | Ext |, ....
    //      after every registered extent there is a space
    // used takes all the empty space from free
    //
    // algo:
    //  insert into free and expect the two extents on the sides to merge into one

    let mut rand = TestRng::new();

    //  generate testing data
    //      each odd extent is missing from `free` and is in `used`
    //          - each used extent touches two free ones
    //
    //      layout:
    //      | free | used | free | used | ...
    let (mut used, mut free) = {

        let count = rand.next_range(256..512);

        let mut addr = NonZero::new(2048).unwrap();

        let mut used = Vec::with_capacity(count/3);

        let mut free = Vec::with_capacity((count/3) * 2);

        for i in 0..count {
            let vec = if i % 2 == 0 {
                &mut free
            } else {
                &mut used
            };

            vec.push(random_ext::<1024>(&mut rand, &mut addr));
        }

        (used, free)
    };

    let mut allocator = RawExtentAlloc::uninit();

    let init_result = unsafe {
        allocator.initialize(free.iter().map(|e| TakenExtent::Free(e.clone()) ))
    };

    assert!(matches!(init_result, Ok(())));


    while !used.is_empty() {

        let index = rand.next_range(..used.len());

        let ext = MutableExtent::from_regular(used.remove(index));

        let result = unsafe {
            allocator.free_map_mut()
        }.insert_defragment(ext.clone());

        assert!(matches!(result, Ok(())));

        //  remove ext's left and right neighbors extents from free map and merge them into ext

        let drain_free = match free.iter().enumerate().find(|(_, e)| e.end_address() == ext.address() ) {
            Some((i, _)) => if let Some(right) = free.get(i + 1) {
                assert!(right.address() == ext.end_address());
                i..=i+1
            } else {
                i..=i
            },
            None => panic!("-\\** /-"),
        };

        println!("drain free: {drain_free:?}");

        let merged = {

            let mut ext = ext;

            for e in free.drain(drain_free.clone()) {
                ext.merge(MutableExtent::from_regular(e)).expect("not a neighbor");
            }

            ext
        };

        free.insert(*drain_free.start(), merged.into_regular());


        verify_map(&free, allocator.free_map());

    }



    todo!("remove and merge neighboring extents from free and used");*/
}



#[test]
fn extract_exact() {

    let mut rand = TestRng::new();

    for _ in 0..500 {

        let count = rand.next_range(256..512);
        let exts = random_extents::<2048>(count, &mut rand);

        let mut allocator = RawExtentAlloc::uninit();
        let init_result = unsafe { allocator.initialize(exts.iter().cloned()) };
        assert!(matches!(init_result, Ok(())));

        let (mut free, /*mut used*/ _) = distinguish_free_and_used(exts);

        for _ in 0..150 {

            let idx = rand.next_range(..free.len());

            let extent = MutableExtent::from_regular(free.remove(idx));

            unsafe { allocator.free_map_mut() }.extract_exact(extent.clone())
                .expect("extraction failed");

            verify_map(&free, allocator.free_map());

            //  the extent is no longer present in the map
            let result = unsafe { allocator.free_map_mut() }.extract_exact(extent.clone());
            assert!(matches!(result, Err(())));

            if free.is_empty() { break }
        }

    }
}












/// Produces `(free, used)` from the `Vec<TakenExtent<..>>`
pub(crate) fn distinguish_free_and_used(exts: Vec<TakenExtent<1024, InternalExtent<1024>>>) -> (Vec<InternalExtent<1024>>, Vec<InternalExtent<1024>>) {
    let mut free = Vec::new();
    let mut used = Vec::new();

    for ext in exts {
        match ext {
            TakenExtent::Free(ext) => free.push(ext),
            TakenExtent::Used(ext) => used.push(ext),
        }
    }

    (free, used)
}

/// Verifies the map agains the vector
#[track_caller]
pub(crate) fn verify_map(vec: &Vec<InternalExtent<1024>>, map: &AllocMap<1024>) {

    let mut size_index_count: usize = 0;


    //  verify the map against the vector
    if vec.len() != map.map.len() {
        panic!("vec.len() != map.map.len() | {} != {}", vec.len(), map.map.len());
    }
    //assert!(vec.len() == map.map.len());

    for ext in vec.iter() {
        match map.map.get(&ext.address()) {
            Some(size) => assert!(ext.size() == *size),
            None => panic!("an extent is not present in the map"),
        }
    }


    //  verify the size_index against the map
    for (size, by_align) in map.size_index.iter() {
        for (_, addresses) in by_align.iter() {
            size_index_count += addresses.len();

            for address in addresses.iter() {
                match map.map.get(address) {
                    Some(map_size) => assert!(*map_size == *size),
                    None => panic!("size_index contains address that is not registered in the map"),
                }
            }
        }
    }

    assert!(size_index_count == map.map.len());
}








/// Generates random `Vec<TakenExtent<>` filled with `Free` non-overlapping entries
///
/// This function uses a counter for the address, so the extents are naturally sorted based on address
pub(crate) fn random_free_extents<const MAX_EXT_COUNT: usize>(count: usize, rand: &mut TestRng) -> Vec<TakenExtent<1024, InternalExtent<1024>>> {

    let mut vec = Vec::with_capacity(count);
    let mut address = NonZero::new(1).unwrap();

    for _ in 0..count {
        vec.push(TakenExtent::Free(random_ext::<MAX_EXT_COUNT>(rand, &mut address)));
    }

    vec
}



/// Generates a vector of random extents
///
/// This function uses a counter for the address, so the extents are naturally sorted based on address
pub(crate) fn random_extents<const MAX_EXT_SIZE: usize>(count: usize, rand: &mut TestRng) -> Vec<TakenExtent<1024, InternalExtent<1024>>> {

    let mut vec = Vec::with_capacity(count);
    let mut address = NonZero::new(1024).unwrap();

    for _ in 0..count {
        if rand.next() & 1 != 0 {
            vec.push(TakenExtent::Used(random_ext::<MAX_EXT_COUNT>(rand, &mut address)));
        } else {
            vec.push(TakenExtent::Free(random_ext::<MAX_EXT_COUNT>(rand, &mut address)));
        }
    }

    vec
}

//  Generates a random extent at the `addr`ess and increases the `addr` by its size
pub(crate) fn random_ext<const MAX_EXT_SIZE: usize>(rand: &mut TestRng, addr: &mut NonZero<u64>) -> InternalExtent<1024> {
    let pages = rand.next_range(8..MAX_EXT_SIZE) as u64;

    let ret = InternalExtent::new(
        AlignedNonNull::new_up(*addr),
        NonZero::new(pages).unwrap());

    *addr = addr.saturating_add(pages.saturating_mul(1024));

    ret
}
