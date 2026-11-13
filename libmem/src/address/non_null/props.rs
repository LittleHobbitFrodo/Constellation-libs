use core::{num::NonZero, ptr::NonNull};

use crate::{Address, Alignment};



/// Unifies the properties of the `NonZero` and `NonNull` types to be used in by `AlignedNonNull`
/// - Must be paired with the `Address` trait
pub(crate) trait NonNullProps: Sized + Copy + Clone + Ord + PartialOrd + Eq + PartialEq {

    /// The nullable variant of this `NonNull` type
    /// - `usize` for `NonZero<usize>`, `*mut T` for `NonNull<T>`
    type Nullable: Sized + Copy + Clone + Address<NonNullable = Self, Unum = Self::Unum, Snum = Self::Snum, NonNullUnum = Self::NonNullUnum>;

    /// Unsigned number type used to do operations
    type Unum: Sized + Copy + Clone;

    /// Signed number type used to do operations
    type Snum: Sized + Copy + Clone;

    type NonNullUnum: Sized + Copy + Clone;


    /// Converts the nullable variant into the non-nullable one
    fn new(value: Self::Nullable) -> Option<Self>;

    /// Converts the nullable variant into the non-nullable one without checking if the value is null
    ///
    /// # Safety
    /// It is up to the caller to guarantee that the given `value` is not null
    unsafe fn new_unchecked(value: Self::Nullable) -> Self;

    /// Converts the nullable value into `Option<NonNullable>` by calling `core::mem::transmute()`
    fn new_transmuted(nullable: Self::Nullable) -> Option<Self>;

    /// Returns the underlying nullable value
    fn as_nullable(&self) -> Self::Nullable;



    /// Adds the given amout of pages to the address
    /// - The calculated result is saturated and aligned down
    /// to prevent overflows and to keep the address aligned
    ///   - The logic looks like this: `self.saturating_add(pages.saturating_mul(ALIGN)).align_down(ALIGN)`
    /// - This function will panic in debug mode instead of introducing undefined behaviour
    fn aligned_add_saturate<const ALIGN: usize>(self, pages: Self::Unum) -> Self;

    /// Adds the given amount of pages to the address without checking for overflow
    /// - Introduced undefined behaviour if the calculation overflows
    ///   - The logic looks like this: `self.unchecked_add(pages.unchecked_mul(ALIGN))`
    /// - This function will panic in debug_mode instead of introducing undefined behaviour
    ///
    /// # Safety
    /// It is up to the caller to guarantee that integer overflow will never
    /// occur. Violating this rule may introduce undefined behaviour
    unsafe fn aligned_add_unchecked<const ALIGN: usize>(self, amount: Self::Unum) -> Self;

    /// Offsets the pointer by given amount of pages
    /// - The calculated result is saturated and aligned down
    /// to prevent overflows and to keep the address aligned
    ///   - The logic looks like this: `self.saturating_add(amount.saturating_add(ALIGN)).align_down(ALIGN)`
    /// - This function will panic in debug_mode instead of introducing undefined behaviour
    fn aligned_offset_saturate<const ALIGN: usize>(self, amount: Self::Snum) -> Option<Self>;


    /// Returns the alignment of the address
    /// - The returned value is guaranteed to be grater than or equal to `ALIGN`
    fn get_alignment(&self) -> Self::NonNullUnum;

    /// Formats the pointer as for the `core::fmt::Pointer` trait
    fn pointer_fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;


    /// Returns the underlying nullable value
    fn inner(&self) -> Self::Nullable;


}


impl NonNullProps for NonZero<usize> {

    type Nullable = usize;
    type Unum = usize;
    type Snum = isize;

    type NonNullUnum = NonZero<usize>;

    #[inline(always)]
    fn new(value: Self::Nullable) -> Option<Self> { Self::new(value) }

    #[inline(always)]
    unsafe fn new_unchecked(value: Self::Nullable) -> Self {
        unsafe {
            Self::new_unchecked(value)
        }
    }

    #[inline(always)]
    fn as_nullable(&self) -> Self::Nullable { self.get() }

    #[inline(always)]
    fn new_transmuted(nullable: Self::Nullable) -> Option<Self> {
        unsafe {
            core::mem::transmute(nullable)
        }
    }


    #[inline]
    fn aligned_add_saturate<const ALIGN: usize>(self, amount: Self::Unum) -> Self {
        let raw = self.get().saturating_add(amount.saturating_mul(ALIGN)).align_down(ALIGN);

        debug_assert!(raw > 0);

        unsafe { Self::new_unchecked(raw) }
    }

    #[inline]
    unsafe fn aligned_add_unchecked<const ALIGN: usize>(self, amount: Self::Unum) -> Self {
        let raw = unsafe {
            self.get().unchecked_add(amount.unchecked_mul(ALIGN))
        };

        debug_assert!(raw > 0 && raw.is_aligned_to(ALIGN));

        unsafe {
            Self::new_unchecked(raw)
        }
    }

    #[inline]
    fn aligned_offset_saturate<const ALIGN: usize>(self, amount: Self::Snum) -> Option<Self> {
        let raw = self.get().saturating_add_signed(amount.saturating_mul(ALIGN as isize)).align_down(ALIGN);

        unsafe {
            core::mem::transmute(raw)
        }
    }

    #[inline(always)]
    fn get_alignment(&self) -> Self::NonNullUnum { self.isolate_lowest_one() }


    #[inline(always)]
    fn pointer_fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&(self.get() as *const ()), f)
    }


    #[inline(always)]
    fn inner(&self) -> Self::Nullable { self.get() }

}


impl NonNullProps for NonZero<u64> {

    type Nullable = u64;
    type Unum = u64;
    type Snum = i64;

    type NonNullUnum = NonZero<u64>;

    #[inline(always)]
    fn new(value: Self::Nullable) -> Option<Self> { Self::new(value) }

    #[inline(always)]
    unsafe fn new_unchecked(value: Self::Nullable) -> Self {
        unsafe {
            Self::new_unchecked(value)
        }
    }

    #[inline(always)]
    fn as_nullable(&self) -> Self::Nullable { self.get() }

    #[inline(always)]
    fn new_transmuted(nullable: Self::Nullable) -> Option<Self> {
        unsafe {
            core::mem::transmute(nullable)
        }
    }


    #[inline]
    fn aligned_add_saturate<const ALIGN: usize>(self, amount: Self::Unum) -> Self {
        let raw = self.get().saturating_add(amount.saturating_mul(ALIGN as u64)).align_down(ALIGN);

        debug_assert!(raw > 0);

        unsafe { Self::new_unchecked(raw) }
    }

    #[inline]
    unsafe fn aligned_add_unchecked<const ALIGN: usize>(self, amount: Self::Unum) -> Self {
        let raw = unsafe {
            self.get().unchecked_add(amount.unchecked_mul(ALIGN as u64))
        };

        debug_assert!(raw > 0 && raw.is_aligned_to(ALIGN));

        unsafe {
            Self::new_unchecked(raw)
        }
    }

    #[inline]
    fn aligned_offset_saturate<const ALIGN: usize>(self, amount: Self::Snum) -> Option<Self> {
        let raw = self.get().saturating_add_signed(amount.saturating_mul(ALIGN as i64)).align_down(ALIGN);

        unsafe {
            core::mem::transmute(raw)
        }
    }


    #[inline(always)]
    fn get_alignment(&self) -> Self::NonNullUnum { self.isolate_lowest_one() }


    #[inline(always)]
    fn pointer_fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&(self.get() as *const ()), f)
    }


    #[inline(always)]
    fn inner(&self) -> Self::Nullable { self.get() }


}

impl<T> NonNullProps for NonNull<T> {

    type Nullable = *mut T;
    type Unum = usize;
    type Snum = isize;

    type NonNullUnum = NonZero<usize>;

    #[inline(always)]
    fn new(value: Self::Nullable) -> Option<Self> { Self::new(value) }

    #[inline(always)]
    unsafe fn new_unchecked(value: Self::Nullable) -> Self {
        unsafe {
            Self::new_unchecked(value)
        }
    }

    #[inline(always)]
    fn as_nullable(&self) -> Self::Nullable { self.as_ptr() }

    #[inline(always)]
    fn new_transmuted(nullable: Self::Nullable) -> Option<Self> {
        unsafe {
            core::mem::transmute(nullable)
        }
    }

    fn aligned_add_saturate<const ALIGN: usize>(self, amount: Self::Unum) -> Self {
        let ptr = self.as_ptr() as usize;
        let raw = ptr.saturating_add(amount.saturating_mul(ALIGN)).align_down(ALIGN);

        unsafe { Self::new_unchecked(raw as *mut T) }
    }

    unsafe fn aligned_add_unchecked<const ALIGN: usize>(self, amount: Self::Unum) -> Self {
        let ptr = self.as_ptr() as usize;

        let raw = unsafe {
            ptr.unchecked_add(amount.unchecked_mul(ALIGN))
        };

        debug_assert!(raw > 0 && raw.is_aligned_to(ALIGN));

        unsafe {
            Self::new_unchecked(raw as *mut T)
        }
    }


    fn aligned_offset_saturate<const ALIGN: usize>(self, amount: Self::Snum) -> Option<Self> {
        let ptr = self.as_ptr() as usize;

        let raw = ptr.saturating_add_signed(amount.saturating_mul(ALIGN as isize)).align_down(ALIGN);

        unsafe {
            core::mem::transmute(raw)
        }
    }

    #[inline(always)]
    fn get_alignment(&self) -> Self::NonNullUnum {
        let nz: NonZero<usize> = unsafe {
            core::mem::transmute(self)
        };

        nz.isolate_lowest_one()
    }


    fn pointer_fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&self.as_ptr(), f)
    }

    #[inline(always)]
    fn inner(&self) -> Self::Nullable { self.as_ptr() }

}
