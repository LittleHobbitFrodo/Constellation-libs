use core::num::NonZero;
use core::fmt::Debug;
use crate::Alignment;

use super::{IntegerOverflow, LayoutDescriptor};

/// Used by the `ExtentAllocator` to allocate contignous extents
#[repr(transparent)]
#[derive(Clone)]
pub struct ExtentLayout<const ALIGN: usize>(NonZero<u64>);


impl<const ALIGN: usize> ExtentLayout<ALIGN> {

    /// The generic `ALIGN` must be greater than or equal to 64 and must be a power of two
    const ALIGN_MUST_BE_POWER_OF_TWO: () = assert!(ALIGN.is_power_of_two());

    /// Constructs a new `ExtentLayout` from the given `pages` and `align`
    /// - Uses `1` as alignment if the `align` is not a power of two
    ///
    /// # Safety
    /// This function is not inherently unsafe, but since ExtentLayout is
    /// the default layout descriptor, improper use of this function could
    /// violate certain assumptions specific to the use of the allocator
    pub unsafe fn from_pages_with_align(pages: NonZero<u64>, align: NonZero<u64>) -> Result<Self, IntegerOverflow> {
        let _ = Self::ALIGN_MUST_BE_POWER_OF_TWO;

        if pages.get() <= Self::MAX_PAGE_COUNT {
            unsafe { Ok(Self::from_pages_with_align_unchecked(pages, align)) }
        } else {
            Err(IntegerOverflow)
        }
    }


    /// Constructs a new `ExtentLayout` from the given `pages` and `align`
    /// - Uses `1` as alignment if the `align` is not a power of two
    ///
    /// # Safety
    /// It is up to the caller to guarantee that the
    pub unsafe fn from_pages_with_align_unchecked(pages: NonZero<u64>, align: NonZero<u64>) -> Self {
        let _ = Self::ALIGN_MUST_BE_POWER_OF_TWO;
        unsafe {
            let bits = (pages.get() << Self::SIZE_SHIFT) | align.lowest_one() as u64;
            Self(NonZero::new_unchecked(bits))
        }
    }

    /// Mask of the size field
    const MASK_SIZE: u64 = 0xFFFFFFFFFFFFFFC0;

    /// Mask of the align field
    const MASK_ALIGN: u64 = 0x3F;

    const SIZE_SHIFT: u64 = 6;
}

impl<const ALIGN: usize> LayoutDescriptor<ALIGN> for ExtentLayout<ALIGN> {

    const MAX_PAGE_COUNT: u64 = Self::MASK_SIZE >> Self::SIZE_SHIFT;

    type Err = IntegerOverflow;

    #[inline(always)]
    fn size(&self) -> NonZero<u64> {
        unsafe {
            NonZero::new_unchecked(self.0.get() >> Self::SIZE_SHIFT)
        }
    }

    /// The alignment is set to `1` by default
    #[inline(always)]
    fn from_pages(count: NonZero<u64>) -> Result<Self, Self::Err> {
        let _ = Self::ALIGN_MUST_BE_POWER_OF_TWO;

        if count.get() <= Self::MAX_PAGE_COUNT {
            unsafe {
                Ok(Self::from_pages_unchecked(count))
            }
        } else {
            Err(IntegerOverflow)
        }
    }

    /// The alignment is set to `1` by default
    #[inline(always)]
    unsafe fn from_pages_unchecked(count: NonZero<u64>) -> Self {
        let _ = Self::ALIGN_MUST_BE_POWER_OF_TWO;
        unsafe {
            Self(NonZero::new_unchecked(count.get() << Self::SIZE_SHIFT))
        }
    }

    #[inline(always)]
    fn align(&self) -> NonZero<u64> {
        unsafe {
            NonZero::new_unchecked(1 << (self.0.get() & Self::MASK_ALIGN))
        }
    }

    /*fn split(&mut self) -> Option<Self> {
        let half = NonZero::new(self.0.get().saturating_div(2))?;

        *self = unsafe {
            //  safety:
            let count = NonZero::new_unchecked(self.0.get() - half.get());
            Self::from_pages_unchecked(count)
        };

        Some(Self(half))
    }*/

}


impl<const ALIGN: usize> Debug for ExtentLayout<ALIGN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ExtentLayout {{ size: {}, align: {} }}", self.size(), self.align())
    }
}



#[cfg(test)]
use libtest::TestRng;


#[test]
fn from_pages() {
    let mut rand = TestRng::new();

    for _ in 0..10_000 {

        let size = NonZero::new(rand.next_range(1..) as u64).unwrap();

        let layout = ExtentLayout::<1024>::from_pages(size);

        if size.get() <= ExtentLayout::<1024>::MAX_PAGE_COUNT {
            let layout = layout.expect("constructor failed when it should succeed");
            assert!(layout.size() == size);
            assert!(layout.align().get() == 1);
        } else {
            assert!(matches!(layout, Err(IntegerOverflow)));
        }
    }
}

#[test]
fn from_pages_unchecked() {

    let mut rand = TestRng::new();

    for _ in 0..1000 {

        let size = NonZero::new(rand.next_range(1..ExtentLayout::<1024>::MAX_PAGE_COUNT as usize) as u64).unwrap();

        let layout = unsafe {
            ExtentLayout::<1024>::from_pages_unchecked(size)
        };

        assert!(layout.size() == size);
        assert!(layout.align().get() == 1);
    }
}


/// Also tests `from_pages_with_align_unchecked()`
#[test]
fn from_pages_with_align() {
    let mut rand = TestRng::new();

    for _ in 0..1000 {

        let size = NonZero::new(rand.next_range(1..) as u64).unwrap();

        let align = NonZero::new((1u64 << rand.next_range(..=63)) + rand.next_range(..=1) as u64).unwrap();

        let layout = unsafe {
            ExtentLayout::<1024>::from_pages_with_align(size, align)
        };

        if size.get() <= ExtentLayout::<1024>::MAX_PAGE_COUNT {
            let layout = layout.expect("constructor failed when it should succeed");

            assert!(layout.size() == size);
            if align.get() % 2 == 0 {
                assert!(layout.align() == align);
            } else {
                //  align is not a power of two => align = 1
                assert!(layout.align().get() == 1);
            }
        } else {
            assert!(matches!(layout, Err(IntegerOverflow)));
        }


    }
}
