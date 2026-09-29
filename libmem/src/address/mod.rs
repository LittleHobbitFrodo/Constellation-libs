use core::fmt::Pointer;

use crate::{PAGE_SIZE, PageAlignment};

#[cfg(test)]
use crate::Alignment;

mod non_null;
pub use non_null::*;

/// Marks any address type and provides functions to work with its alignment
#[allow(private_bounds)]
pub trait Address where Self: AddressMarker {}


impl Address for usize {}

impl Address for u64 {}

impl<T> Address for *const T {}
impl<T> Address for *mut T {}







/// An address that is guaranteed to be aligned to a specific boundary
///
/// This type typically stores `*mut T`, but it can also store
/// `usize` (or `u64`) to represent a physical address
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct AlignedAddress<Addr: Address, const ALIGN: usize = PAGE_SIZE>(Addr);

#[allow(private_bounds)]
impl<Addr: Address, const ALIGN: usize> AlignedAddress<Addr, ALIGN> {

    /// The generic `ALIGN` must be a power of two
    const ALIGN_MUST_BE_A_POWER_OF_TWO: () = assert!(ALIGN.is_power_of_two());

    /// Constructs a new `AlignedAddress` if the given address is aligned to a certain boundary
    #[inline]
    pub fn new(address: Addr) -> Option<Self> {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;

        if address.is_aligned_to(ALIGN) {
            Some(Self(address))
        } else {
            None
        }
    }

    /// Constructs a new `AlignedAddress` without checking proper alignment
    ///
    /// # Safety
    /// It is up to the caller to guarantee that the `address` is properly
    /// aligned to a page boundary. If this rule is violated, the system
    /// can fall into undefined behaviour
    pub const unsafe fn new_unchecked(address: Addr) -> Self {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;
        Self(address)
    }

    /// Creates a new `AlignedAddress` by aligning the given `address` down to a page boundary
    #[inline]
    pub fn new_down(address: Addr) -> Self {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;
        Self(address.align_down(ALIGN))
    }

    /// Creates a new `AlignedAddress` by aligning the given `address` up to a page boundary
    #[inline]
    pub fn new_up(address: Addr) -> Self {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;
        Self(address.align_up(ALIGN))
    }

    /// Offsets the address by a specified page count
    ///
    ///
    /// # Behaviour
    /// **The result is saturated on overflow and the aligned down**.
    /// The only implication of this behaviour (also the reason for it)
    /// is that when the addition would overflow it sets all bits, leading
    /// to misaligned result and broken invariant.
    #[inline(always)]
    pub fn aligned_add(self, pages: usize) -> Self { Self(self.0.aligned_add::<ALIGN>(pages).align_down(ALIGN)) }

    /// Adds the page count to `self`
    #[inline(always)]
    pub fn aligned_offset(self, pages: isize) -> Self { Self(self.0.aligned_offset::<ALIGN>(pages)) }

}

impl<Addr: Address, const ALIGN: usize> core::fmt::Pointer for AlignedAddress<Addr, ALIGN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_ptr(f)
    }
}

impl<Addr: Address, const ALIGN: usize> core::ops::Deref for AlignedAddress<Addr, ALIGN> {
    type Target = Addr;
    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}


impl<Addr: Address, const ALIGN: usize> From<Addr> for AlignedAddress<Addr, ALIGN> {
    fn from(value: Addr) -> Self {
        if value.is_aligned_to(ALIGN) {
            Self(value)
        } else {
            panic!("unaligned address")
        }
    }
}



/// Marker trait that is internally used to indicate any address type
pub(crate) trait AddressMarker
where Self: PageAlignment + Sized + Copy + Default {
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result;

    /// Adds the page count to itself
    /// - The result is saturated on overflow
    fn aligned_add<const ALIGN: usize>(self, pages: usize) -> Self;

    ///  `self` by the given page count
    /// - The result is saturated on overflow
    fn aligned_offset<const ALIGN: usize>(self, pages: isize) -> Self;
}

impl AddressMarker for usize {
    fn fmt_ptr(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:X}", self)
    }

    #[inline(always)]
    fn aligned_add<const ALIGN: usize>(self, pages: usize) -> Self {
        self.saturating_add(pages.saturating_mul(ALIGN))
    }

    #[inline(always)]
    fn aligned_offset<const ALIGN: usize>(self, pages: isize) -> Self {
        self.saturating_add_signed(pages.saturating_mul(ALIGN as isize))
    }

}
impl AddressMarker for u64 {
    #[inline(always)]
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "0x{:X}", self)
    }

    #[inline(always)]
    fn aligned_add<const ALIGN: usize>(self, pages: usize) -> Self {
        self.saturating_add((pages as u64).saturating_mul(ALIGN as u64))
    }

    #[inline(always)]
    fn aligned_offset<const ALIGN: usize>(self, pages: isize) -> Self {
        self.saturating_add_signed(pages.saturating_mul(ALIGN as isize) as i64)
    }

}
impl<T> AddressMarker for *const T {
    #[inline(always)]
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result {
        Pointer::fmt(&self, f)
    }

    #[inline(always)]
    fn aligned_add<const ALIGN: usize>(self, pages: usize) -> Self {
        (self as usize).saturating_add(pages.saturating_mul(ALIGN)) as Self
    }

    #[inline(always)]
    fn aligned_offset<const ALIGN: usize>(self, pages: isize) -> Self {
        (self as usize).saturating_add_signed(pages.saturating_mul(ALIGN as isize)) as Self
    }

}
impl<T> AddressMarker for *mut T {
    #[inline(always)]
    fn fmt_ptr(&self, f: &'_ mut core::fmt::Formatter) -> core::fmt::Result {
        Pointer::fmt(&self, f)
    }

    #[inline(always)]
    fn aligned_add<const ALIGN: usize>(self, pages: usize) -> Self {
        (self as usize).saturating_add(pages.saturating_mul(ALIGN)) as Self
    }

    #[inline(always)]
    fn aligned_offset<const ALIGN: usize>(self, pages: isize) -> Self {
        (self as usize).saturating_add_signed(pages.saturating_mul(ALIGN as isize)) as Self
    }
}





/// Since all types shares one implementation, only one test per function is required
#[test]
fn new() {
    type AlignedAddr = AlignedAddress<usize, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {
        let ptr = rand.next_range(..usize::MAX);

        if ptr.is_aligned_to(1024) {
            let addr = AlignedAddr::new(ptr)
                .expect("new() returned None instead of Some()");

            assert!(*addr == ptr);
        } else {
            assert!(matches!(AlignedAddr::new(ptr), None));
        }
    }
}


/// Since all types shares one implementation, only one test per function is required
#[test]
fn new_up_and_down() {
    type AlignedAddr = AlignedAddress<usize, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {

        let ptr = rand.next_range(..usize::MAX);

        let up = AlignedAddr::new_up(ptr);
        assert!(*up == ptr.align_up(1024));

        let down = AlignedAddr::new_down(ptr);
        assert!(*down == ptr.align_down(1024));
    }
}


/// Since all types shares one implementation, only one test per function is required
#[test]
fn aligned_add() {

    type AlignedAddr = AlignedAddress<usize, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {

        let ptr = (rand.next_range(..usize::MAX/2)).align_up(1024);
        let added = rand.next_range(..usize::MAX/2);

        let addr = AlignedAddr::new(ptr)
            .expect("failed to construct AlignedAddr");

        let result = addr.aligned_add(added as usize);
        let expected = ptr.saturating_add(added.saturating_mul(1024))
            .align_down(1024);

        assert!(*result == expected);
        assert!(result.is_aligned_to(1024));
    }
}
