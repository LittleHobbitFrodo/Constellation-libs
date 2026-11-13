

use core::num::NonZero;

use libtest::TestRng;

use crate::extent_alloc::{extent::InternalExtent, helpers::{alloc_map::tests::random_extents, tests::distinguish_free_and_used}, layout::{ExtentLayout, LayoutDescriptor}, raw_alloc::RawExtentAlloc};

use libtest::println;


const EXT_COUNT: core::ops::Range<usize> = 256usize..512;
const MAX_EXT_SIZE: usize = 2048;


#[ignore = "TODO"]
#[test]
fn allocate() {

    todo!();

    /*let mut rand = TestRng::new();

    let mut exts = random_extents::<MAX_EXT_SIZE>(rand.next_range(EXT_COUNT), &mut rand);


    let mut allocator = RawExtentAlloc::uninit();

    let init_result = unsafe {
        allocator.initialize(exts.iter().cloned())
    };

    assert!(matches!(init_result, Ok(())));

    let (mut free, mut used) = distinguish_free_and_used(exts);


    let mut fail_count = 0;

    for _ in 0..100 {

        let layout = {

            let size = NonZero::new(rand.next_range((MAX_EXT_SIZE/2)..(MAX_EXT_SIZE * 2)) as u64).unwrap();
            ExtentLayout::from_pages(size).expect("failed to construct layout")
        };

        match allocator.allocate::<InternalExtent<1024>, ExtentLayout<1024>, RawExtentAlloc<1024>>(layout) {
            Some(batch) => {

                panic!("batch: {:?}", batch.as_slice());

                todo!("assert invariants");
            },
            None => fail_count += 1,//todo!("assert failure"),
        }

    }

    assert!(fail_count != 100);*/


}
