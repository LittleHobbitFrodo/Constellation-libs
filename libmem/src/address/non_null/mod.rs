use core::{cmp::Ordering, fmt::{Debug, Pointer}, num::{NonZero, NonZeroUsize}, ptr::NonNull};

use crate::{Address, Alignment, props::NonNullProps};

pub(crate) mod props;


/// An address that is guaranteed to be both aligned and also not null
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct AlignedNonNull<Addr: Address, const ALIGN: usize>(Addr::NonNullable);


impl<Addr: Address, const ALIGN: usize> AlignedNonNull<Addr, ALIGN> {

    const ALIGN_ASSERTION: () = assert!(ALIGN.is_power_of_two());
    const SIZE_ASSERTION: () = assert!(size_of::<Addr>() == size_of::<Addr::NonNullable>());


    /// Upgrades the non-null value into a `AlignedNonNull` if the value is properly aligned
    #[allow(private_interfaces)]
    pub fn new(non_null: Addr::NonNullable) -> Option<Self> {

        let _ = Self::ALIGN_ASSERTION;
        let _ = Self::SIZE_ASSERTION;

        if non_null.as_nullable().is_aligned_to(ALIGN) {
            Some(Self(non_null))
        } else {
            None
        }
    }

    /// Upgrades the non-null value into a `AlignedNonNull` without checking if it is aligned
    ///
    /// # Safety
    /// It is up to the caller to guarantee that the given value is properly aligned
    #[allow(private_interfaces)]
    pub const unsafe fn new_unchecked(non_null: Addr::NonNullable) -> Self {
        let _ = Self::ALIGN_ASSERTION;
        let _ = Self::SIZE_ASSERTION;

        Self(non_null)
    }

    /// Upgrades the non-null value into a `AlignedNonNull` by aligning it up to the `ALIGN` boundary
    #[allow(private_interfaces)]
    pub fn new_up(non_null: Addr::NonNullable) -> Self {
        let _ = Self::ALIGN_ASSERTION;
        let _ = Self::SIZE_ASSERTION;

        let aligned = non_null.as_nullable().align_up(ALIGN);

        unsafe {
            Self(Addr::NonNullable::new_unchecked(aligned))
        }
    }

    /// Upgrades the non-null value into a `AlignedNonNull` by aligning it down to the `ALIGN` boundary
    /// - `None` is returned if the alignment operation results in null
    #[allow(private_interfaces)]
    pub fn new_down(non_null: Addr::NonNullable) -> Option<Self> {
        let _ = Self::ALIGN_ASSERTION;
        let _ = Self::SIZE_ASSERTION;

        let aligned = non_null.as_nullable().align_down(ALIGN);

        Some(Self(Addr::NonNullable::new_transmuted(aligned)?))
    }

    /// Adds the given amount of pages to the address
    /// - The resulted value is saturated and aligned down on a overflow
    #[allow(private_interfaces)]
    #[inline(always)]
    pub fn aligned_add_saturate(self, pages: Addr::Unum) -> Self {
        Self(self.0.aligned_add_saturate::<ALIGN>(pages))
    }


    /// Adds the given amount of pages to the address without checking for overflow
    ///
    /// # Safety
    /// It is up to the caller to guarantee that integer overflow will never
    /// occur. Violating this rule may introduce undefined behaviour
    #[allow(private_interfaces)]
    #[inline(always)]
    pub unsafe fn aligned_add_unchecked(self, pages: Addr::Unum) -> Self {
        unsafe {
            Self(self.0.aligned_add_unchecked::<ALIGN>(pages))
        }
    }


    /// Offsets the pointer by the given amount of pages
    /// - The resulted value is saturated and aligned down on a overflow
    #[allow(private_interfaces)]
    #[inline(always)]
    pub fn aligned_offset_saturate(self, pages: Addr::Snum) -> Option<Self> {
        let raw = self.0.aligned_offset_saturate::<ALIGN>(pages)?;
        Some(Self(raw))
    }


    /// Returns the current alignment of the address
    /// - The returned number is guaranteed to be at least as big as `ALIGN`
    #[allow(private_interfaces)]
    #[inline(always)]
    pub fn alignment(&self) -> Addr::NonNullUnum { self.0.get_alignment() }

    /// Returns the underlying `NonZero` or `NonNull`
    #[allow(private_interfaces)]
    #[inline(always)]
    pub fn as_non_null(&self) -> Addr::NonNullable { self.0 }

    /// Converts the aligned pointer into a number (or regular a pointer)
    #[inline(always)]
    pub fn as_nullable(&self) -> Addr { self.0.inner() }

}


impl<Addr: Address, const ALIGN: usize> Pointer for AlignedNonNull<Addr, ALIGN> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.pointer_fmt(f)
    }
}


impl<Addr: Address, const ALIGN: usize> Debug for AlignedNonNull<Addr, ALIGN> {

    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&self, f)
    }
}



impl<Addr: Address, const ALIGN: usize> PartialEq<Addr> for AlignedNonNull<Addr, ALIGN> {

    #[inline(always)]
    fn eq(&self, other: &Addr) -> bool { self.0.inner().eq(other) }

    #[inline(always)]
    fn ne(&self, other: &Addr) -> bool { self.0.inner().ne(other) }
}


impl<Addr: Address, const ALIGN: usize> PartialEq for AlignedNonNull<Addr, ALIGN> {

    #[inline(always)]
    fn eq(&self, other: &Self) -> bool { self.0.eq(&other.0) }


    fn ne(&self, other: &Self) -> bool { self.0.ne(&other.0) }

}

impl<Addr: Address, const ALIGN: usize> Eq for AlignedNonNull<Addr, ALIGN> {}






impl<Addr: Address, const ALIGN: usize> PartialOrd<Addr> for AlignedNonNull<Addr, ALIGN> {

    #[inline(always)]
    fn ge(&self, other: &Addr) -> bool { self.0.inner().ge(other) }

    #[inline(always)]
    fn gt(&self, other: &Addr) -> bool { self.0.inner().gt(other) }

    #[inline(always)]
    fn le(&self, other: &Addr) -> bool { self.0.inner().le(other) }

    #[inline(always)]
    fn lt(&self, other: &Addr) -> bool { self.0.inner().lt(other) }

    #[inline(always)]
    fn partial_cmp(&self, other: &Addr) -> Option<Ordering> { self.0.inner().partial_cmp(other) }
}

impl<Addr: Address, const ALIGN: usize> PartialOrd for AlignedNonNull<Addr, ALIGN> {

    #[inline(always)]
    fn ge(&self, other: &Self) -> bool { self.0.ge(&other.0) }

    #[inline(always)]
    fn gt(&self, other: &Self) -> bool { self.0.gt(&other.0) }

    #[inline(always)]
    fn le(&self, other: &Self) -> bool { self.0.le(&other.0) }

    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.0.partial_cmp(&other.0) }
}


impl<Addr: Address, const ALIGN: usize> Ord for AlignedNonNull<Addr, ALIGN> {

    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering { self.0.cmp(&other.0) }
}






#[cfg(test)]
use libtest::{TestRng, println};

#[cfg(test)]
const ALIGN_TEST: usize = 1024;



/// Generates code for a part of the `AlignedNonNull::new()` test
/// - The generated block yields
macro_rules! generate_new_test_block_for {
    ($type:ty where NZ = $nz_type:ty | $addr:ident, $paths_taken_shift:literal) => {

        {
            type NZ = $nz_type;
            type ANN = AlignedNonNull<$type, ALIGN_TEST>;

            let addr = NZ::new($addr.inner() as $type).unwrap();

            //  checks if the address is aligned
            match ANN::new(addr) {
                Some(aligned) => {

                    //  is aligned => assert invariants

                    assert!(addr.inner().is_aligned_to(ALIGN_TEST));

                    assert!(aligned.as_non_null() == addr);
                    assert!(aligned.as_non_null().inner().is_aligned_to(ALIGN_TEST));

                    0b01 << $paths_taken_shift
                },
                None => {
                    //  is not aligned => assert it

                    assert!(!addr.inner().is_aligned_to(ALIGN_TEST));

                    0b10 << $paths_taken_shift
                },
            }
        }

    };
}


#[test]
fn new() {

    let mut rand = TestRng::new();


    let mut paths_taken: usize = 0;
    let mut i: u16 = 0;


    loop {

        let addr = NonZero::<usize>::new(rand.next_range(1..))
            .expect("rng generated NULL value");

        paths_taken |= generate_new_test_block_for!(usize where NZ = NonZero<usize> | addr, 0);
            //  paths: 0b000011

        paths_taken |= generate_new_test_block_for!(u64 where NZ = NonZero<u64> | addr, 2);
            //  paths: 0b001100

        paths_taken |= generate_new_test_block_for!(*mut u8 where NZ = NonNull<u8> | addr, 4);
            //  paths: 0b110000


        if i > 1000 && paths_taken == 0b111111 {
            break;
        }

        i = i.checked_add(1).expect("not all paths in the test were taken");
    }
}






macro_rules! generate_new_up_test_block_for {
    ($type:ty where NZ = $nz_type:ty | $addr:ident) => {

        {

            type NZ = $nz_type;
            type ANN = AlignedNonNull<$type, ALIGN_TEST>;

            let addr = NZ::new($addr.inner() as $type).unwrap();


            let address = ANN::new_up(addr);

            assert!(address.as_nullable() == addr.inner().align_up(ALIGN_TEST));

        }


    };
}

#[test]
fn new_up() {


    let mut rand = TestRng::new();

    for _ in 0..1000 {

        let addr = NonZero::<usize>::new(rand.next_range(1..))
            .expect("rng generated NULL value");

        generate_new_up_test_block_for!(usize where NZ = NonZero<usize> | addr);
        generate_new_up_test_block_for!(u64 where NZ = NonZero<u64> | addr);
        generate_new_up_test_block_for!(*mut u8 where NZ = NonNull<u8> | addr);

    }

}





macro_rules! generate_new_down_test_block_for {
    ($type:ty where NZ = $nz_type:ty | $addr:ident, $paths_taken_shift:literal) => {

        {
            type NZ = $nz_type;
            type ANN = AlignedNonNull<$type, ALIGN_TEST>;

            let addr = NZ::new($addr.inner() as $type).unwrap();


            match ANN::new_down(addr) {
                Some(aligned) => {

                    assert!(aligned.as_nullable().is_aligned_to(ALIGN_TEST));
                    assert!(aligned.as_nullable() == addr.inner().align_down(ALIGN_TEST));

                    0b01 << $paths_taken_shift
                },
                None => {

                    assert!(addr.inner() < ALIGN_TEST as $type);
                    0b10 << $paths_taken_shift
                },
            }
        }




    };
}

#[test]
fn new_down() {


    let mut rand = TestRng::new();

    let mut paths_taken: usize = 0;
    let mut i: u16 = 0;

    loop {

        let mut addr = if rand.next() & 0b111 == 0b111 {
            //  generate smaller than ALIGN_TEST
            NonZero::<usize>::new(rand.next_range(1..ALIGN_TEST)).unwrap()
        } else {
            NonZero::<usize>::new(rand.next_range(1..usize::MAX))
                .expect("rng generated NULL value")
        };

        if addr.get() & 1 != 0 {
            addr = NonZero::new(addr.get().align_up(ALIGN_TEST)).unwrap();
        }

        paths_taken |= generate_new_down_test_block_for!(usize where NZ = NonZero<usize> | addr, 0);
        paths_taken |= generate_new_down_test_block_for!(u64 where NZ = NonZero<u64> | addr, 2);
        paths_taken |= generate_new_down_test_block_for!(*mut u8 where NZ = NonNull<u8> | addr, 4);



        if i >= 2000 && paths_taken == 0b111111 {
            break;
        }

        i = i.checked_add(1).expect("not all paths in the test were taken");

    }

}







#[test]
fn aligned_add_saturated() {

    let mut rand = TestRng::new();

    todo!();


}



/*impl<Addr: Address, const ALIGN: usize> PartialOrd for AlignedNonNull<Addr, ALIGN> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some()
    }
}*/



/*use core::{fmt::Pointer, num::NonZero, ptr::NonNull};

use crate::{Address, Alignment};

#[derive(Copy, Clone /*, PartialEq, Eq, PartialOrd, Ord*/)]
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

        if address.get_inner().is_aligned_to(ALIGN) {
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
        Some(Self(Addr::from_inner(address.get_inner().align_down(ALIGN))?))
    }

    /// Constructs a new `AlignedNonNull` by aligning the `address` up to the generic `ALIGN` boundary
    /// - This operation will never overflow
    #[inline]
    pub fn new_up(address: Addr) -> Self {
        let _ = Self::ALIGN_MUST_BE_A_POWER_OF_TWO;
        unsafe { Self(Addr::from_inner_unchecked(address.get_inner().align_up(ALIGN))) }
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
    pub fn inner(&self) -> Addr { self.0.clone() }

    /// Calculates the alignment of this address
    /// - In other words: Isolates the least significant bit that is set
    #[inline(always)]
    pub fn alignment(&self) -> Addr::InnerIntNZ {
        self.0.alignment()
    }

    /// Calculates the alignment of this address
    /// - In other words: Isolates the most significant bit that is set
    #[inline(always)]
    pub fn max_alignment(&self) -> Addr::InnerIntNZ {
        self.0.max_alignment()
    }
}

/*impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> core::ops::Deref for AlignedNonNull<Addr, ALIGN> {
    type Target = Addr;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}*/

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> Pointer for AlignedNonNull<Addr, ALIGN> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.inner().fmt_ptr(f)
    }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> PartialEq<Self>
    for AlignedNonNull<Addr, ALIGN>
{
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> Eq for AlignedNonNull<Addr, ALIGN> {}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> PartialOrd<Self>
    for AlignedNonNull<Addr, ALIGN>
{
    #[inline(always)]
    fn lt(&self, other: &Self) -> bool {
        self.0.lt(&other.0)
    }

    #[inline(always)]
    fn le(&self, other: &Self) -> bool {
        self.0.le(&other.0)
    }

    #[inline(always)]
    fn gt(&self, other: &Self) -> bool {
        self.0.gt(&other.0)
    }

    #[inline(always)]
    fn ge(&self, other: &Self) -> bool {
        self.0.ge(&other.0)
    }

    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> Ord for AlignedNonNull<Addr, ALIGN> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

trait NonNullMarker {}

#[allow(private_bounds)]
pub trait NonNullAddress<const ALIGN: usize>
where
    Self: Sized + Clone + NonNullMarker + PartialEq<Self> + Eq + PartialOrd<Self> + Ord,
{
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

    //// Returns the inner type
    fn get_inner(&self) -> Self::Inner;

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
    fn get_inner(&self) -> Self::Inner { self.get() }

    #[inline(always)]
    fn from_inner(ptr: Self::Inner) -> Option<Self> {
        unsafe { core::mem::transmute(ptr) }
    }

    unsafe fn from_inner_unchecked(ptr: usize) -> Self {
        unsafe { Self::new_unchecked(ptr) }
    }

    #[inline(always)]
    fn align_add(self, pages: usize) -> Self {
        self.saturating_add(pages.saturating_mul(ALIGN))
    }

    #[inline]
    fn align_offset(self, pages: isize) -> Option<Self> {
        unsafe {
            core::mem::transmute(
                self.get()
                    .saturating_add_signed(pages.saturating_mul(ALIGN as isize)),
            )
        }
    }

    #[inline(always)]
    fn alignment(&self) -> NonZero<usize> {
        self.isolate_lowest_one()
    }

    #[inline(always)]
    fn max_alignment(&self) -> Self::InnerIntNZ {
        self.isolate_highest_one()
    }
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
    fn get_inner(&self) -> Self::Inner { self.get() }

    #[inline(always)]
    fn from_inner(ptr: Self::Inner) -> Option<Self> {
        unsafe { core::mem::transmute(ptr) }
    }

    #[inline(always)]
    unsafe fn from_inner_unchecked(ptr: u64) -> Self {
        unsafe { Self::new_unchecked(ptr) }
    }

    #[inline(always)]
    fn align_add(self, pages: usize) -> Self {
        unsafe {
            Self::new_unchecked(
                self.get()
                    .saturating_add((pages as u64).saturating_mul(ALIGN as u64)),
            )
        }
    }

    #[inline]
    fn align_offset(self, pages: isize) -> Option<Self> {
        unsafe {
            let ptr = self
                .get()
                .saturating_add_signed((pages as i64).saturating_add_unsigned(ALIGN as u64));
            core::mem::transmute(ptr)
        }
    }

    #[inline(always)]
    fn alignment(&self) -> NonZero<u64> {
        self.isolate_lowest_one()
    }

    #[inline(always)]
    fn max_alignment(&self) -> Self::InnerIntNZ {
        self.isolate_highest_one()
    }
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

    fn get_inner(&self) -> Self::Inner {
        self.as_ptr()
    }

    #[inline(always)]
    fn from_inner(ptr: Self::Inner) -> Option<Self> {
        unsafe { core::mem::transmute(ptr) }
    }

    #[inline(always)]
    unsafe fn from_inner_unchecked(ptr: *mut T) -> Self {
        unsafe { NonNull::new_unchecked(ptr) }
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
            let ptr = (self.as_ptr() as usize)
                .saturating_add_signed(pages.saturating_mul(ALIGN as isize));
            core::mem::transmute(ptr)
        }
    }

    #[inline(always)]
    fn alignment(&self) -> Self::InnerIntNZ {
        unsafe { NonZero::new_unchecked((self.as_ptr() as usize).isolate_lowest_one()) }
    }

    #[inline(always)]
    fn max_alignment(&self) -> NonZero<usize> {
        unsafe { NonZero::new_unchecked((self.as_ptr() as usize).isolate_highest_one()) }
    }
}

impl<Addr: NonNullAddress<ALIGN>, const ALIGN: usize> core::fmt::Debug
    for AlignedNonNull<Addr, ALIGN>
{
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

    let mut paths_taken: usize = 0;
    let mut i: u32 = 0;

    loop {
        let ptr = unsafe { NonZero::new_unchecked(rand.next().saturating_add(1)) };

        if ptr.get().is_aligned_to(1024) {
            paths_taken |= 0b01;

            let addr = AlignedNN::new(ptr).expect("new() returned None instead of Some()");

            assert!(addr.inner() == ptr);
        } else {
            paths_taken |= 0b10;
            assert!(matches!(AlignedNN::new(ptr), None));
        }

        if i >= 5000 && paths_taken == 0b11 {
            break;
        }

        i = i
            .checked_add(1)
            .expect("no all paths in the test were taken");
    }
}

#[test]
fn new_up() {
    type AlignedNN = AlignedNonNull<NonZero<usize>, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {
        let ptr = unsafe { NonZero::new_unchecked(rand.next().saturating_add(1)) };

        let addr = AlignedNN::new_up(ptr);
        assert!(addr.inner().get() == ptr.get().align_up(1024));
    }
}

#[test]
fn new_down() {
    type AlignedNN = AlignedNonNull<NonZero<usize>, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    let mut paths_taken: usize = 0;
    let mut i: u16 = 0;

    loop {
        let ptr = if rand.next() & 1 != 0 {
            NonZero::new(rand.next().align_up(1024)).expect("align_up() produced zero")
        } else {
            NonZero::new(rand.next().saturating_add(1)).unwrap()
        };

        match AlignedNN::new_down(ptr) {
            Some(a) => {
                paths_taken |= 0b01;
                assert!(a.inner().get() == ptr.get().align_down(1024) as usize);
            }
            None => {
                paths_taken |= 0b10;
                assert!(ptr.get() < 1024);
            }
        }

        if i >= 5000 && paths_taken == 0b11 {
            break;
        }

        i = i
            .checked_add(1)
            .expect("no all paths in the test were taken");
    }
}

#[test]
fn aligned_add() {
    type AlignedNN = AlignedNonNull<NonZero<usize>, 1024>;

    use libtest::TestRng;
    let mut rand = TestRng::new();

    for _ in 0..5000 {
        let ptr =
            unsafe { NonZero::new_unchecked(rand.next_range(1..usize::MAX / 2).align_up(1024)) };

        let added = rand.next_range(..usize::MAX / 2);

        let addr = AlignedNN::new(ptr).expect("failed to construct AlignedNonNull");

        let result = addr.aligned_add(added);
        let expected = ptr.saturating_add(added.saturating_mul(1024));
        assert!(result.inner() == expected);
    }
}*/
