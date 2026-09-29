use core::{fmt::Pointer, num::NonZero, ptr::NonNull};

use crate::{Address, Alignment, PAGE_SIZE};




#[derive(Copy, Clone/*, PartialEq, Eq, PartialOrd, Ord*/)]
#[repr(transparent)]
#[allow(private_bounds)]
pub struct AlignedNonNull<Addr: NonNullAddress<ALIGN>, const ALIGN: usize>(Addr);

#[allow(private_bounds)]
impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> AlignedNonNull<Addr, ALIGN> {

    /// The generic `ALIGN` must be a power of two
    const ALIGN_MUST_BE_A_POWER_OF_TWO: () = assert!(ALIGN.is_power_of_two());

    /// Constructs a new `AlignedNonNull` if the given
    /// `address` is aligned to the generic `ALIGN` boundary
    #[inline]
    pub fn new(address: Addr) -> Option<Self> {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;

        if address.inner().is_aligned_to(ALIGN) {
            Some(Self(address))
        } else {
            None
        }
    }

    /// Constructs a new `AlignedNonNull` without checking whether the `address`
    /// is non null and aligned to the `ALIGN` boundary
    ///
    /// # Safety
    /// It is up to caller to guarantee that the given `address`
    /// is non-null and aligned to `ALIGN`
    pub const unsafe fn new_unchecked(address: Addr) -> Self {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;
        Self(address)
    }

    /// Constructs a new `AlignedNonNull` by aligning the `address` down to the generic `ALIGN` boundary
    ///
    /// Given that the alignment operation can make the
    /// address NULL, this function returns the `AlignedAddress`
    /// only if the alignment operation yields non-null result
    #[inline]
    pub fn new_down(address: Addr) -> Option<Self> {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;
        Some(Self(Addr::from_inner(address.inner().align_down(ALIGN))?))
    }

    /// Constructs a new `AlignedNonNull` by aligning the `address` up to the generic `ALIGN` boundary
    /// - This operation will never overflow
    #[inline]
    pub fn new_up(address: Addr) -> Self {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;
        unsafe {
            Self(Addr::from_inner_unchecked(address.inner().align_up(ALIGN)))
        }
    }

    /// Adds the given `pages` (chunks of `ALIGN` bytes) to the address
    #[inline(always)]
    pub fn aligned_add(self, pages: usize) -> Self {
        Self(self.0.align_add(pages))
    }

    /// Adds the given `pages` (chunks of `ALIGN` bytes) to the address
    #[inline]
    pub fn aligned_offset(self, pages: isize) -> Option<Self> {
        Some(Self(self.0.align_offset(pages)?))
    }

    /// Returns the inner address
    #[inline(always)]
    pub fn as_inner(&self) -> Addr::Inner { self.0.inner() }


    /// Calculates the alignment of this address
    /// - In other words: Isolates the least significant bit that is set
    #[inline(always)]
    pub fn alignment(&self) -> Addr::InnerIntNZ { self.0.alignment() }

    /// Calculates the alignment of this address
    /// - In other words: Isolates the most significant bit that is set
    #[inline(always)]
    pub fn max_alignment(&self) -> Addr::InnerIntNZ { self.0.max_alignment() }

}


impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> core::ops::Deref for AlignedNonNull<Addr, ALIGN> {
    type Target = Addr;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> Pointer for AlignedNonNull<Addr, ALIGN> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_ptr(f)
    }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> PartialEq<Self> for AlignedNonNull<Addr, ALIGN> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool { self.0.eq(&other.0) }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> Eq for AlignedNonNull<Addr, ALIGN> {}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> PartialOrd<Self> for AlignedNonNull<Addr, ALIGN> {
    #[inline(always)]
    fn lt(&self, other: &Self) -> bool { self.0.lt(&other.0) }

    #[inline(always)]
    fn le(&self, other: &Self) -> bool { self.0.le(&other.0) }

    #[inline(always)]
    fn gt(&self, other: &Self) -> bool { self.0.gt(&other.0) }

    #[inline(always)]
    fn ge(&self, other: &Self) -> bool { self.0.ge(&other.0) }

    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> Ord for AlignedNonNull<Addr, ALIGN> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { self.0.cmp(&other.0) }
}







trait NonNullMarker {}

#[allow(private_bounds)]
pub trait NonNullAddress<const ALIGN: usize> where Self: Sized + Clone + NonNullMarker + PartialEq<Self> + Eq + PartialOrd<Self> + Ord {

    /// The inner type (ex. `NonNull<T>`)
    ///
    /// # Explanation
    /// `Self` is a `NonZero<T>` or `NonNull<T>` type, to `Inner` is the `T`
    type Inner: Alignment + Address;

    /// The integer representation of the inner type
    ///
    /// For example `NonNull<T>::InnerInt` is `usize`
    ///
    /// and `NonZero<u64>::InnerInt` is `u64`
    type InnerInt: Alignment + Address;

    /// `NonZero<Self::InnerInt>`
    type InnerIntNZ;

    /// Formats `self` as `core::fmt::Pointer` would
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result;

    /// Returns the inner type
    fn inner(&self) -> Self::Inner;

    /// Constructs a new `Self` if `ptr` is not null
    fn from_inner(ptr: Self::Inner) -> Option<Self>;

    /// Constructs a new `Self` from `ptr` without checking whether `ptr` is null
    unsafe fn from_inner_unchecked(ptr: Self::Inner) -> Self;

    /// Adds the given pages to the address
    fn align_add(self, pages: usize) -> Self;

    /// Offsets the address by the given pages
    fn align_offset(self, pages: isize) -> Option<Self>;

    /// Calculates the alignment of this address
    /// - In other words: Isolates the least significant bit that is set
    fn alignment(&self) -> Self::InnerIntNZ;

    /// Calculates the maximum alignment of this address
    /// - In other words: Isolates the most significant bit that is set
    fn max_alignment(&self) -> Self::InnerIntNZ;
}





impl NonNullMarker for NonZero<usize> {}
impl<const ALIGN: usize> NonNullAddress<ALIGN> for NonZero<usize> {

    type Inner = usize;
    type InnerInt = usize;
    type InnerIntNZ = NonZero<usize>;

    #[inline]
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "0x{:x}", self)
    }

    #[inline(always)]
    fn inner(&self) -> Self::Inner { self.get() }

    #[inline(always)]
    fn from_inner(ptr: Self::Inner) -> Option<Self> {
        unsafe {
            core::mem::transmute(ptr)
        }
    }

    unsafe fn from_inner_unchecked(ptr: usize) -> Self {
        unsafe {
            Self::new_unchecked(ptr)
        }
    }

    #[inline(always)]
    fn align_add(self, pages: usize) -> Self {
        self.saturating_add(pages.saturating_mul(ALIGN))
    }

    #[inline]
    fn align_offset(self, pages: isize) -> Option<Self> {
        unsafe {
            core::mem::transmute(self.get().saturating_add_signed(pages.saturating_mul(ALIGN as isize)))
        }
    }


    #[inline(always)]
    fn alignment(&self) -> NonZero<usize> { self.isolate_lowest_one() }

    #[inline(always)]
    fn max_alignment(&self) -> Self::InnerIntNZ { self.isolate_highest_one() }

}




impl NonNullMarker for NonZero<u64> {}
impl<const ALIGN: usize> NonNullAddress<ALIGN> for NonZero<u64> {

    type Inner = u64;
    type InnerInt = u64;
    type InnerIntNZ = NonZero<u64>;

    #[inline]
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "0x{:x}", self)
    }

    #[inline(always)]
    fn inner(&self) -> Self::Inner { self.get() }

    #[inline(always)]
    fn from_inner(ptr: Self::Inner) -> Option<Self> {
        unsafe {
            core::mem::transmute(ptr)
        }
    }

    #[inline(always)]
    unsafe fn from_inner_unchecked(ptr: u64) -> Self {
        unsafe {
            Self::new_unchecked(ptr)
        }
    }

    #[inline(always)]
    fn align_add(self, pages: usize) -> Self {
        unsafe {
            Self::new_unchecked(self.get().saturating_add((pages as u64).saturating_mul(ALIGN as u64)))
        }
    }

    #[inline]
    fn align_offset(self, pages: isize) -> Option<Self> {
        unsafe {
            let ptr = self.get().saturating_add_signed((pages as i64).saturating_add_unsigned(ALIGN as u64));
            core::mem::transmute(ptr)
        }
    }

    #[inline(always)]
    fn alignment(&self) -> NonZero<u64> { self.isolate_lowest_one() }

    #[inline(always)]
    fn max_alignment(&self) -> Self::InnerIntNZ { self.isolate_highest_one() }

}



impl<T> NonNullMarker for NonNull<T> {}
impl<T, const ALIGN: usize> NonNullAddress<ALIGN> for NonNull<T> {
    type Inner = *mut T;
    type InnerInt = usize;
    type InnerIntNZ = NonZero<usize>;

    #[inline]
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result {
        Pointer::fmt(&self, f)
    }

    #[inline(always)]

    fn inner(&self) -> Self::Inner { self.as_ptr() }

    #[inline(always)]
    fn from_inner(ptr: Self::Inner) -> Option<Self> {
        unsafe {
            core::mem::transmute(ptr)
        }
    }

    #[inline(always)]
    unsafe fn from_inner_unchecked(ptr: *mut T) -> Self {
        unsafe {
            NonNull::new_unchecked(ptr)
        }
    }

    #[inline(always)]
    fn align_add(self, pages: usize) -> Self {
        unsafe {
            let ptr = (self.as_ptr() as usize).saturating_add(pages.saturating_mul(ALIGN));
            Self::new_unchecked(ptr as *mut T)
        }
    }

    #[inline(always)]
    fn align_offset(self, pages: isize) -> Option<Self> {
        unsafe {
            let ptr = (self.as_ptr() as usize).saturating_add_signed(pages.saturating_mul(ALIGN as isize));
            core::mem::transmute(ptr)
        }
    }

    #[inline(always)]
    fn alignment(&self) -> Self::InnerIntNZ {
        unsafe {
            NonZero::new_unchecked((self.as_ptr() as usize).isolate_lowest_one())
        }
    }

    #[inline(always)]
    fn max_alignment(&self) -> NonZero<usize> {
        unsafe {
            NonZero::new_unchecked((self.as_ptr() as usize).isolate_highest_one())
        }
    }

}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> core::fmt::Debug for AlignedNonNull<Addr, ALIGN> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Pointer::fmt(&self, f)
    }
}




#[test]
fn new() {
    type AlignedNN = AlignedNonNull<NonZero<usize>, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {

        let ptr = unsafe {
            NonZero::new_unchecked(rand.next().saturating_add(1))
        };

        if ptr.get().is_aligned_to(1024) {
            let addr = AlignedNN::new(ptr)
                .expect("new() returned None instead of Some()");

            assert!(*addr == ptr);
        } else {
            assert!(matches!(AlignedNN::new(ptr), None));
        }
    }
}

#[test]
fn new_up() {

    type AlignedNN = AlignedNonNull<NonZero<usize>, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {

        let ptr = unsafe {
            NonZero::new_unchecked(rand.next().saturating_add(1))
        };

        let addr = AlignedNN::new_up(ptr);
        assert!(addr.as_inner() == ptr.get().align_up(1024));
    }
}

#[test]
fn new_down() {

    type AlignedNN = AlignedNonNull<NonZero<usize>, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {

        let ptr = unsafe {
            NonZero::new_unchecked(rand.next().saturating_add(1))
        };

        let addr = match AlignedNN::new_down(ptr) {
            Some(a) => a,
            None => {
                assert!(ptr.get() < 1024);
                continue
            }
        };

        assert!(addr.as_inner() == ptr.get().align_down(1024));
    }
}



#[test]
fn aligned_add() {

    type AlignedNN = AlignedNonNull<NonZero<usize>, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {

        let ptr = unsafe {
            NonZero::new_unchecked(rand.next_range(1..usize::MAX/2).align_up(1024))
        };

        let added = rand.next_range(..usize::MAX/2);

        let addr = AlignedNN::new(ptr)
            .expect("failed to construct AlignedNonNull");

        let result = addr.aligned_add(added);
        let expected = ptr.saturating_add(added.saturating_mul(1024));
        assert!(*result == expected);
    }
}
