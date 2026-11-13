use core::{cell::LazyCell, num::NonZero};

use liblock::Mutex;
use libtest::TestRng;

use super::LayoutDescriptor;

pub(crate) static LAYOUT_RNG: Mutex<LazyCell<TestRng>> = Mutex::new(LazyCell::new(|| TestRng::new() ));


/// A layout used to test the extent allocator, it generates random alignment when constructed
#[derive(Clone)]
#[repr(transparent)]
pub struct TestLayout<const ALIGN: usize>(NonZero<u64>);

impl<const ALIGN: usize> TestLayout<ALIGN> {

    const ALIGN_BITS: u64 = 0b1111;

    pub const ALIGN_MASK: u64 = Self::ALIGN_BITS << Self::ALIGN_SHIFT;
    pub const ALIGN_SHIFT: usize = 63-5;

    pub const SIZE_MASK: u64 = u64::MAX & !Self::ALIGN_MASK;
}


impl<const ALIGN: usize> LayoutDescriptor<ALIGN> for TestLayout<ALIGN> {

    const MAX_PAGE_COUNT: u64 = Self::SIZE_MASK;
    const MIN_ALIGN: usize = 1;

    unsafe fn from_pages_unchecked(count: NonZero<u64>) -> Self {

        let align = LAYOUT_RNG.lock().next_range(1..=Self::ALIGN_BITS as usize) as u64;
        assert!(align > 0);

        let raw = (count.get() & Self::SIZE_MASK) | (align << Self::ALIGN_SHIFT);

        unsafe {
            Self(NonZero::new_unchecked(raw))
        }
    }


    type Err = ();

    fn from_pages(count: NonZero<u64>) -> Result<Self, Self::Err> {
        if count.get() <= Self::MAX_PAGE_COUNT {
            Ok(Self(count))
        } else {
            Err(())
        }
    }


    fn align(&self) -> NonZero<u64> {
        NonZero::new(1u64 << (self.0.get() >> Self::ALIGN_SHIFT))
            .expect("failed to get layout alignment")
    }

    fn size(&self) -> NonZero<u64> {
        NonZero::new(self.0.get() & Self::SIZE_MASK)
            .expect("failed to get layout size")
    }

    fn remove(self, other: Self) -> Option<Self> {
        let rem = self.0.get().saturating_sub(other.0.get());

        if rem > 0 {
            unsafe {
                Some(Self::from_pages_unchecked(NonZero::new_unchecked(rem)))
            }
        } else {
            None
        }
    }

}


//  TODO: REDO ALL UNIT TESTS?
//  - assert that all paths are taken
