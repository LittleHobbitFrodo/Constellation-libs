use super::*;

use libtest::TestRng;

#[test]
fn insert() {

    use alloc::vec::Vec;

    use libtest::TestRng;

    let mut rand = TestRng::new();

    for _ in 0..256 {

        let mut cache = AlignmentCache::<1024>::new();

        let count = rand.next_range(512..2048);

        let mut extents = Vec::with_capacity(count);


        let mut addr = 1024;

        for _ in 0..count {

            let ext = random_extent(&mut rand, &mut addr);


            //  insert the extent into the comparison vector and the cache
            extents.push(ext.clone());

            if !cache.insert(MutableExtent::from_regular(ext.clone())) {
                panic!("failed to insert into the cache");
            }

            //  index within the cache's inner array
            let index = AlignmentCache::index_from_address(ext.address()).min(ALIGN_ARRAY_LEN-1);

            assume!(index < cache.0.len());


            //  make sure that the extent is in the right place
            let map = &cache.inner()[index];
            let addr = map.get(&ext.size()).expect("map does not contain the size")
                .get(&ext.address()).expect("map does not contain the address");

            assert!(*addr == ext.address());




            validate(cache.inner(), &extents);


        }

    }


    fn validate(cache: &[SizedMap<1024>; ALIGN_ARRAY_LEN], cmp: &Vec<InternalExtent<1024>>) {

        {   //  assert that the count of
            let count: usize = cache.iter().map(|map| {
                map.values().map(|addresses| addresses.len() ).sum::<usize>()
            }).sum();

            assert!(count == cmp.len());
        }

        for extent in cmp {

            let index = AlignmentCache::index_from_address(extent.address()).min(ALIGN_ARRAY_LEN-1);

            assume!(index < cache.len());

            cache.get(index).unwrap().get(&extent.size()).expect("cache does not contain the size")
                .get(&extent.address()).expect("cache does not contain the address");
        }
    }


}

#[test]
fn extract_suitable() {

    use libtest::{TestRng, println};
    use alloc::vec::Vec;


    let mut rand = TestRng::new();

    for _ in 0..1000 {

        let mut cache = AlignmentCache::<1024>::new();

        let count = rand.next_range(256..512);

        //  construct comparison data and cache's map
        let mut cmp = {
            let mut vec = Vec::with_capacity(count);
            let mut addr = 1024;

            for _ in 0..count {
                let ext = random_extent(&mut rand, &mut addr);
                vec.push(ext.clone());

                if !cache.insert(MutableExtent::from_regular(ext)) {
                    panic!("failed to insert into the map");
                }

            }

            vec
        };

        for _ in 0..count.min(200) {
            let index = rand.next_range(..cmp.len());
            let extent = cmp.get(index).unwrap().clone();

            let layout = {
                let size = NonZero::new(rand.next_range(1..700) as u64).unwrap();
                let align = NonZero::new(1 << rand.next_range(..6)).unwrap();
                unsafe {
                    ExtentLayout::from_pages_with_align_unchecked(size, align)
                }
            };


            match cache.extract_suitable(&layout) {
                Some(ext) => {
                    //  extract from the comparison data
                    //  and assert that the extent exists

                    cmp.remove(index);

                    //todo!();
                },
                None => {
                    //  assert that there is no suitable extent
                    let found = cmp.iter().find(|ext| {
                        ext.size() >= layout.size() && AlignmentCache::index_from_address(ext.address()) >= layout.align().trailing_zeros() as usize
                    }).is_some();
                    assert!(!found, "suitable extent was not found in the cache but it was found in the cmp data");
                }
            }
        }


    }

}



#[cfg(test)]
fn random_extent(rand: &mut TestRng, addr: &mut u64) -> InternalExtent<1024> {

    let address = AlignedNonNull::new_up(NonZero::new(*addr).unwrap());

    let size = NonZero::new(rand.next_range(1..512) as u64).unwrap();

    *addr += size.get() * 1024;

    InternalExtent::new(address, size)
}
