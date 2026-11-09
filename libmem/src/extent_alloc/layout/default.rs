use core::cmp::Ordering;
use core::num::NonZeroU64;
use core::{hint::cold_path, num::NonZero};
use core::fmt::Debug;
use crate::{Alignment, assume};

use super::{IntegerOverflow, LayoutDescriptor};

/// Used by the `ExtentAllocator` to allocate contignous extents
#[repr(transparent)]
#[derive(Clone)]
pub struct ExtentLayout<const ALIGN: usize>(NonZero<u64>);


impl<const ALIGN: usize> ExtentLayout<ALIGN> {

    /// The generic `ALIGN` must be greater than or equal to 64 and must be a power of two
    const ALIGN_MUST_BE_POWER_OF_TWO: () = assert!(ALIGN.is_power_of_two());

    /*/// Constructs a new `ExtentLayout` from the given `pages` and `align`
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

        let bits = (pages.get() << Self::SIZE_SHIFT) | align.lowest_one() as u64;
        unsafe {
            Self(NonZero::new_unchecked(bits))
        }
    }*/

    /// Mask of the size field
    const MASK_SIZE: u64 = 0xFFFFFFFFFFFFFFFF;

    /*/// Mask of the align field
    const MASK_ALIGN: u64 = 0x3F;

    const SIZE_SHIFT: u64 = 6;*/
}

impl<const ALIGN: usize> LayoutDescriptor<ALIGN> for ExtentLayout<ALIGN> {

    const MAX_PAGE_COUNT: u64 = Self::MASK_SIZE;// >> Self::SIZE_SHIFT;

    type Err = IntegerOverflow;

    #[inline(always)]
    fn size(&self) -> NonZero<u64> {

        let size = self.0.get() & Self::MASK_SIZE;// >> Self::SIZE_SHIFT;

        assume!(size != 0);

        unsafe {
            NonZero::new_unchecked(size)
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
            Self(NonZero::new_unchecked(count.get()/* << Self::SIZE_SHIFT*/))
        }
    }

    #[inline(always)]
    fn align(&self) -> NonZero<u64> {
        unsafe {
            NonZero::new_unchecked(1)
        }
        /*unsafe {
            NonZero::new_unchecked(1 << (self.0.get() & Self::MASK_ALIGN))
        }*/
    }

    fn remove(self, other: Self) -> Option<Self> {
        //  alignment is always 1

        let new_size = NonZero::new(self.size().get().saturating_sub(other.size().get()))?;

        unsafe {
            Some(Self::from_pages_unchecked(new_size))
        }
    }


    /*fn split_half(&mut self) -> Option<Self> {

        let half = match NonZero::new(self.0.get() / 2) {
            Some(half) => half,
            None => {
                cold_path();
                return None
            }
        };

        let new_size = match NonZero::new(self.size().get().saturating_sub(half.get())) {
            Some(sz) => sz,
            None => {
                cold_path();
                return None
            }
        };

        *self = unsafe {
            //  safety: `new_size` is less than `self.size`, cannot overflow
            Self::from_pages_unchecked(new_size)
        };


        unsafe {
            Some(Self::from_pages_unchecked(half))
        }
    }

    fn split_at(&mut self, size: NonZero<u64>) -> Option<Self> {

        let remaining = self.size().get().saturating_sub(size.get());

        if remaining > 0 {
            *self = unsafe {
                Self::from_pages_unchecked(size)
            };

            unsafe {
                Some(Self::from_pages_unchecked(NonZero::new_unchecked(remaining)))
            }
        } else {
            None
        }

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

        let size = NonZero::new(rand.next_range(1..ExtentLayout::<1024>::MAX_PAGE_COUNT as usize) as u64)
            .unwrap();

        let layout = unsafe {
            ExtentLayout::<1024>::from_pages_unchecked(size)
        };

        assert!(layout.size() == size);
        assert!(layout.align().get() == 1);
    }
}


/*#[test]
fn split_half() {

    let mut rand = TestRng::new();

    for _ in 0..1000 {

        let size = NonZero::new(rand.next_range(1..ExtentLayout::<1024>::MAX_PAGE_COUNT as usize) as u64)
            .unwrap();

        let mut layout = ExtentLayout::<1024>::from_pages(size)
            .expect("failed to create ExtentLAyout");

        assert!(layout.size() == size);
        assert!(layout.align().get() == 1);


        loop {

            let prev_size = layout.size();

            let lay = match layout.split_half() {
                Some(lay) => lay,
                None => {
                    assert!(layout.size().get() == 1);
                    break
                },
            };

            assert!(layout.size().saturating_add(lay.size().get()) == prev_size);
            assert!(lay.align().get() == 1);
        }

    }

}


#[test]
fn split_at() {

    let mut rand = TestRng::new();

    for _ in 0..1000 {

        let size = NonZero::new(rand.next_range(1..ExtentLayout::<1024>::MAX_PAGE_COUNT as usize / 2) as u64)
            .unwrap();

        let mut layout = ExtentLayout::<1024>::from_pages(size)
            .expect("failed to create ExtentLAyout");

        assert!(layout.size() == size);
        assert!(layout.align().get() == 1);


        let new_size = NonZero::new(rand.next_range(1..ExtentLayout::<1024>::MAX_PAGE_COUNT as usize) as u64)
            .unwrap();

        match new_size.cmp(&layout.size()) {
            Ordering::Less => match layout.split_at(new_size) {
                Some(lay) => assert!(layout.size().saturating_add(lay.size().get()) == size),
                None => panic!("failed to split layout ({} to {})", size, new_size),
            },
            Ordering::Equal | Ordering::Greater => match layout.split_at(new_size) {
                Some(_) => panic!("split_at() succeeded with size greater that the layout.size()"),
                None => { /* OK */ }
            }
        }

    }

}*/
