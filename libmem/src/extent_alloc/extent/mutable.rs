

use core::{cmp::Ordering::{Equal, Greater, Less}, mem::ManuallyDrop, num::NonZero, ops::BitOrAssign};

use crate::{AlignedAddress, AlignedNonNull, Alignment, NonNullAddress, cold_panic, extent_alloc::extent::{Extent, InternalExtent, RawExtent}};




#[derive(Clone)]
#[repr(C)]
pub struct MutableExtent<const ALIGN: usize> {
    start: AlignedNonNull<NonZero<u64>, ALIGN>,
    size: NonZero<u64>
}


impl<const ALIGN: usize> Extent<ALIGN> for MutableExtent<ALIGN> {
    #[inline(always)]
    fn address(&self) -> AlignedNonNull<NonZero<u64>, ALIGN> { self.start }

    fn size(&self) -> NonZero<u64> { self.size }
}

impl<const ALIGN: usize> MutableExtent<ALIGN> {

    /// Constructs a new `MutableExtent`
    #[inline(always)]
    pub const fn new(address: AlignedNonNull<NonZero<u64>, ALIGN>, pages: NonZero<u64>) -> Self {
        Self { size: pages, start: address }
    }

    /// Splits the `MutableExtent` at the given size, returning
    /// the new extent if there is any space left
    /// - Returns `Err` if `size` is greater than the size of `self`
    pub fn split(&mut self, size: NonZero<u64>) -> Result<Option<Self>, ()> {
        match self.size().cmp(&size) {
            Less => Err(()),
            Equal => Ok(None),
            Greater => {

                let new_addr = self.address().aligned_add(size.get() as usize);

                let new_size = unsafe {
                    let new_size = self.size().get() - size.get();

                    debug_assert!(new_size > 0);
                    NonZero::new_unchecked(new_size)
                };

                self.size = size;

                Ok(Some(Self::new(new_addr, new_size)))
            }
        }
    }


    /// Finds the maximum alignment that any of the addresses contained in the extent can have
    ///
    /// By that definition, if you take starting address of the extent and align it
    /// up to the returned alignment, it will always fit into the extent
    /// - Tha same work for the ending address when aligned down
    ///
    /// ```rust
    /// //  pseudo code
    /// let alignment = mut_ext.find_max_alignment();
    ///
    /// let aligned_down = mut_ext.end_address().align_down(alignment);
    /// let aligned_up = mut_ext.address().align_up(alignment);
    ///
    /// //  aligned addresses fits into the extent
    /// assert!(mut_ext.address() <= aligned_down && mut_ext.end_address() >= aligned_down);
    /// assert!(mut_ext.address() <= aligned_up && mut_ext.end_address() >= aligned_up);
    /// ```
    pub fn find_max_alignment(&self) -> NonZero<u64> {
        let addr = self.end_address();

        let mut align = ALIGN as u64;

        loop {
            let aligned_down = addr.get().align_down(align as usize);

            if aligned_down < self.address().get() {
                //  the address does not fit into the extent's address space
                align  = align >> 1;
                break
            }
            align = align << 1;
        }

        debug_assert!(align > 0);

        unsafe {
            //  safety: the algorithm above will never
            // produce number smaller than `ALIGN`
            NonZero::new_unchecked(align)
        }
    }

    /// Same as `MutableExtent::split_at()`, but does not check whether `size` fits into `self`
    /// - Returns `None` if `self.size()` is equal to `size`
    ///
    /// # Safety
    /// It is up to the caller to guarantee that `size` is less than or equal to `self.size()`. Violating this rule may introduce undefined behaviour
    pub unsafe fn split_unchecked(&mut self, size: NonZero<u64>) -> Option<Self> {
        if self.size() == size {
            None
        } else {
            let new_addr = self.address().aligned_add(size.get() as usize);

            let new_size = unsafe {
                let new_size = self.size().get() - size.get();

                debug_assert!(new_size > 0);
                NonZero::new_unchecked(new_size)
            };

            self.size = size;

            Some(Self::new(new_addr, new_size))
        }
    }


    /// Merges the `neighbor` into `self` if the two extents are touching
    ///
    /// Returns `Err(neighbor)` if the two extents are not touching
    pub fn merge/*<Ext: Extent<ALIGN>>*/(&mut self, neighbor: Self) -> Result<(), Self> {
        if self.is_touching(&neighbor) {
            unsafe { self.merge_unchecked(neighbor) }
            Ok(())
        } else {
            Err(neighbor)
        }
    }

    /// Merges the `neighbor` extent into `self` without checking
    /// whether the two extents are touching each other
    ///
    /// # Safety
    /// It is up to the caller to guarantee that `self` touches the `neighbor` extent.
    /// Otherwise this function may result in undefined behaviour
    pub unsafe fn merge_unchecked(&mut self, neighbor: impl Extent<ALIGN>) {
        let size = self.size().saturating_add(neighbor.size().get());
        if self.address() < neighbor.address() {
            self.size = size;
        } else {
            *self = Self { start: neighbor.address(), size }
        }
        core::mem::forget(neighbor);
    }


    /// Consumes the `MutableExtent` and converts it into a regular extent
    #[inline]
    pub fn into_regular<Ext: Extent<ALIGN> + RawExtent<ALIGN>>(self) -> Ext {
        let Self { size, start } = self;
        Ext::new(start, size)
    }


    /// Converts the given extent into `MutableExtent`
    /// - The extent will not be dropped
    #[inline]
    pub fn from_regular<Ext: Extent<ALIGN> + RawExtent<ALIGN>>(ext: Ext) -> Self {
        let ext = ManuallyDrop::new(ext);
        Self::new(ext.address(), ext.size())
    }



    /// Removes the given extent from `self` without checking
    /// whether the extent fits into `self`
    /// - Returns the extents created by removing `self`
    ///   - `self` is consumed, but not returned
    ///
    /// # Safety
    /// It is up to the caller to guarantee that the extent fits into `self`,
    /// otherwise this function will introduce undefined behaviour
    pub unsafe fn remove_from_unchecked(self, ext: Self) -> RemainingExtents<ALIGN> {

        //  prevent double-free
        let ext = ManuallyDrop::new(ext);


        let front = {
            let size = self.address().get().saturating_sub(ext.address().get()) / ALIGN as u64;

            match NonZero::new(size) {
                Some(size) => Some(MutableExtent::new(ext.address(), size)),
                None => None
            }
        };

        let remainder = {

            let end_addr = self.end_address();

            let size = ext.end_address().get().saturating_sub(end_addr.get()) / ALIGN as u64;

            match NonZero::new(size) {
                Some(size) => Some(MutableExtent::new(self.end_address(), size)),
                None => None,
            }
        };

        RemainingExtents { left: front, right: remainder }
    }

    /*/// Returns the starting address of the extent
    #[inline(always)]
    pub fn address(&self) -> AlignedNonNull<NonZero<u64>, ALIGN> { self.start }

    /// Calculates the ending address of the frame
    #[inline(always)]
    pub fn end_address(&self) -> AlignedNonNull<NonZero<u64>, ALIGN> {
        self.start.aligned_add(self.size().get() as usize)
    }

    /// Returns the size in pages
    #[inline(always)]
    pub fn size(&self) -> NonZero<u64> { self.size }

    /// Indicates whether `self` and `other` are right next to each
    /// other and are then "touching" each other
    /// - Returns `false` if the extents are overlapping
    #[inline]
    pub fn is_touching<Ext: Extent<ALIGN>>(&self, other: &'_ Ext) -> bool {
        //  NOTE: Extents cannot be overlapping
        *self.end_address() == *other.address()     //  |--SELF--||--OTHER--|
        || *self.address() == *other.end_address()  //  |--OTHER--||--SELF--|
    }


    /// Indicates whether `self` fits into `other`
    /// - Uses starting and ending addresses of
    /// the extents to determine the result
    /// - Returns `true` if the extents are equivalent
    ///   - Therefore `assert!(extent.fits_into(extent.clone()))` will pass
    #[inline]
    pub fn fits_into<Ext: Extent<ALIGN>>(&self, other: &'_ Ext) -> bool {
        *self.address() >= *other.address() && *self.end_address() <= *other.end_address()
    }

    /// Indicates whether the two extents are overlapping
    #[inline]
    pub fn is_overlapping<Ext: Extent<ALIGN>>(&self, other: &'_ Ext) -> bool {
        !(self.end_address() <= other.address() || self.address() >= other.end_address())
    }*/


    /// Removes the given extent from `self`, returns the extents created by removing `self`
    /// - `self` is consumed, but not returned
    ///
    /// If the given extent does not fit into `self`, the function returns ownership of `(self, ext)`
    pub fn remove_from(self, ext: Self) -> Result<RemainingExtents<ALIGN>, (Self, Self)> {
        if self.fits_into(&ext) {
            unsafe {
                Ok(self.remove_from_unchecked(ext))
            }
        } else {
            Err((self, ext))
        }
    }

}

/// Returned by `MutableExtent::remove_subext()`
pub struct RemainingExtents<const ALIGN: usize> {
    /// The space in front of the removed extent
    pub left: Option<MutableExtent<ALIGN>>,
    /// The remaining space
    pub right: Option<MutableExtent<ALIGN>>,
}


#[test]
fn remove_from_unchecked() {

    use libtest::{println, TestRng};


    const BIGGER_ADDR: u64 = 1024;
    const BIGGER_SIZE: u64 = 1024 * 4;

    let mut rand = TestRng::new();

    for _ in 0..30_000 {
        let big: MutableExtent<1024> = {
            let addr = AlignedNonNull::new(NonZero::new(BIGGER_ADDR).unwrap()).unwrap();
            MutableExtent::new(addr, NonZero::new(BIGGER_SIZE).unwrap())
        };

        let small = {
            let addr = rand.next_range(BIGGER_ADDR as usize..(BIGGER_ADDR + BIGGER_SIZE) as usize - 1) as u64;
            let addr = AlignedNonNull::new_down(NonZero::new(addr).unwrap()).unwrap();

            let size = rand.next_range(1..(big.end_address().get().strict_sub(addr.get()) / 1024) as usize) as u64;
            let size = NonZero::new(size).unwrap();

            MutableExtent::new(addr, size)
        };

        {
            assert!(small.fits_into(&big));
        }

        let RemainingExtents { left: front, right: remainder } = unsafe { small.clone().remove_from_unchecked(big.clone()) };


        if small.address() == big.address() {
            //  no front
            assert!(matches!(front, None));

            if small.size() == big.size() {
                //  also no rem
                assert!(matches!(remainder, None));
            } else {
                //  rem exists
                let rem = remainder.expect("remainder is None instead of Some()");

                assert!(rem.address() == small.end_address());

                let expected_size = big.end_address().get().strict_sub(small.end_address().get()) / 1024;
                let expected_size = NonZero::new(expected_size).unwrap();

                assert!(rem.size() == expected_size);
            }

        } else if small.end_address() == big.end_address() {
            //  case where small.size == big.size is handled above
            //      => small.size < big.size

            assert!(matches!(remainder, None));

            let front = front.expect("front is None instead of Some()");

            assert!(front.address() == big.address());

            let expected_size = small.address().get().strict_sub(big.address().get()) / 1024;
            let expected_size = NonZero::new(expected_size).unwrap();

            assert!(front.size() == expected_size);
        } else {
            //  both front and rem are Some

            let front = front.expect("front is None instead of Some");
            let rem = remainder.expect("remainder is None instead of Some");

            {   //  validate front
                assert!(front.address() == big.address());

                let expected_size = small.address().get().strict_sub(big.address().get()) / 1024;
                let expected_size = NonZero::new(expected_size).unwrap();

                assert!(front.size() == expected_size);
            }

            {   //  validate remainder
                assert!(rem.end_address() == big.end_address());

                let expected_size = big.end_address().get().strict_sub(small.end_address().get()) / 1024;
                let expected_size = NonZero::new(expected_size).unwrap();

                assert!(rem.size() == expected_size);
            }
        }

    }


}


#[test]
fn split_unchecked() {

    use libtest::TestRng;

    let mut rand = TestRng::new();

    for _ in 0..200_000 {

        let addr = NonZero::new(rand.next_range(1..10240) as u64).unwrap();

        let mut ext: MutableExtent<1024> = MutableExtent::new(
            AlignedNonNull::new_up(addr),
            NonZero::new(rand.next_range(2..10240) as u64).unwrap());
        let original = ext.clone();

        let split_size = NonZero::new(rand.next_range(1..ext.size().get() as usize) as u64).unwrap();

        let splitted = unsafe {
            ext.split_unchecked(split_size)
                .expect("split failed")
        };

        assert!(ext.address() == original.address());
        assert!(ext.size() == split_size);
        assert!(*splitted.address() == *ext.address().aligned_add(split_size.get() as usize));
        assert!(splitted.size().get() == original.size().get() - split_size.get());
    }
}



#[test]
fn split() {

    use libtest::TestRng;

    let mut rand = TestRng::new();

    for _ in 0..2000 {

        let mut ext: MutableExtent<1024> = MutableExtent::new(
            AlignedNonNull::new_up(NonZero::new(rand.next_range(1..10240) as u64).unwrap()),
            NonZero::new(rand.next_range(1..10240) as u64).unwrap());
        let original = ext.clone();

        let split_size = NonZero::new(rand.next_range(1..ext.size().get() as usize * 2) as u64)
            .unwrap();

        let splitted = match split_size.cmp(&ext.size()) {
            Less => match ext.split(split_size) {
                Ok(Some(ext)) => ext,
                Ok(None) => panic!("split produced Ok(None) instead of Ok(Some())"),
                Err(_) => panic!("split failed when it should have succeeded"),
            },
            Equal => match ext.split(split_size) {
                Ok(None) => continue,   //  expected
                Ok(Some(_)) => panic!("split produced Ok(Some()) instead of Ok(None)"),
                Err(_) => panic!("split failed when it should have succeeded"),
            },
            Greater => {
                assert!(matches!(ext.split(split_size), Err(())));
                continue
            },
        };

        assert!(ext.address() == original.address());
        assert!(ext.size() == split_size);
        assert!(*splitted.address() == *ext.address().aligned_add(split_size.get() as usize));
        assert!(splitted.size().get() == original.size().get() - split_size.get());
    }

}


#[test]
fn find_max_alignment() {

    use libtest::{TestRng, println};

    let mut rand = TestRng::new();

    for _ in 0..20_000 {

        let ext: MutableExtent<1024> = MutableExtent::new(
            AlignedNonNull::new_up(NonZero::new(rand.next_range(1..20240) as u64).unwrap()),
            NonZero::new(rand.next_range(1..20240) as u64).unwrap());

        let alignment = ext.find_max_alignment();

        let aligned_down = ext.end_address().get().align_down(alignment.get() as usize);
        let aligned_up = ext.address().get().align_up(alignment.get() as usize);


        assert!(ext.address().get() <= aligned_down && ext.end_address().get() >= aligned_down);
        assert!(ext.address().get() <= aligned_up && ext.end_address().get() >= aligned_up);
        assert!(alignment.is_power_of_two());

        let next_alignment = ext.end_address().get().align_down((alignment.get() << 1) as usize);

        assert!(next_alignment < ext.address().get());
    }
}
