

//#[cfg(target_arch = "x86_64")]
mod x86_64;
//#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

/// One kilobyte (1024 bytes)
pub const KB: usize = 1024;

/// One megabyte (1024 kilobytes)
pub const MB: usize = 1024 * KB;

/// One gigabyte (1024 megabytes)
pub const GB: usize = 1024 * MB;

/// One terabyte (1024 gigabytes)
pub const TB: usize = 1024 * GB;


/// Marker trait marking types that are allowed to implement `Alignment` and `PageAlignment`
trait AlignmentMarker where Self: Sized + Copy + Clone {}

/// Trait used to manipulate address or integer alignment
#[allow(private_bounds)]
pub trait Alignment where Self: AlignmentMarker {
    /// Aligns the address up to a specific boundary
    /// - This operation will never overflow the integer boundary
    ///
    /// The given `align` is required to be a power of two
    fn align_up(self, align: usize) -> Self;
    /// Aligns the address down to a specific boundary
    /// - The given `align` is required to be a power of two
    fn align_down(self, align: usize) -> Self;
    /// Indicates if the address is aligned to a specific boundary
    /// - The given `align` is required to be a power of two
    fn is_aligned_to(&self, align: usize) -> bool;
    /// Returns the alignment of `self`
    /// - In other words: the least significant bit that is set
    fn alignment(&self) -> Self;
    /// Returns the maximum alignment of `self`
    /// - In other words: the most significant bit that is set
    fn max_alignment(&self) -> Self;
}


/// Trait used to align addresses to a architecture-specific page boundary
pub trait PageAlignment where Self: Alignment {
    /// Aligns the address up to a page boundary
    /// - This operation will never overflow the integer boundary
    fn page_align_up(self) -> Self;
    /// Aligns the address down to a page boundary
    fn page_align_down(self) -> Self;
    /// Indicates whether an address is aligned to a page boundary
    fn is_page_aligned(&self) -> bool;
}


macro_rules! impl_alignment {
    ($type:ty) => {
        impl AlignmentMarker for $type {}
        impl Alignment for $type {
            #[inline(always)]
            fn align_up(self, align: usize) -> Self { (self.saturating_add(((align as $type)-1))) & !((align as $type)-1) }

            #[inline(always)]
            fn align_down(self, align: usize) -> Self { self & !((align as $type)-1) }

            #[inline(always)]
            fn is_aligned_to(&self, align: usize) -> bool { (self & !((align as $type)-1)) == *self }

            #[inline(always)]
            fn alignment(&self) -> Self { self.isolate_lowest_one() }

            #[inline(always)]
            fn max_alignment(&self) -> Self { self.isolate_highest_one() }
        }
    };
    (T, $type:ty) => {
        impl<T> AlignmentMarker for $type {}
        impl<T> Alignment for $type {
            #[inline(always)]
            fn align_up(self, align: usize) -> Self { (((self as usize).saturating_add((align-1))) & !(align-1)) as $type }

            #[inline(always)]
            fn align_down(self, align: usize) -> Self { (self as usize & !(align-1)) as $type }

            #[inline(always)]
            fn is_aligned_to(&self, align: usize) -> bool { (*self as usize & !(align-1)) as $type == *self }

            #[inline(always)]
            fn alignment(&self) -> Self { (*self as usize).isolate_lowest_one() as Self }

            #[inline(always)]
            fn max_alignment(&self) -> Self { (*self as usize).isolate_highest_one() as Self }
        }
    };
}



macro_rules! impl_page_alignment {
    ($type:ty) => {
        impl PageAlignment for $type {
            #[inline]
            fn page_align_up(self) -> Self { (self + ((PAGE_SIZE as $type)-1)) & !((PAGE_SIZE as $type)-1) }

            #[inline]
            fn page_align_down(self) -> Self { self & !((PAGE_SIZE as $type)-1) }

            #[inline]
            fn is_page_aligned(&self) -> bool { (self & !((PAGE_SIZE as $type)-1)) >= *self }
        }
    };
    (T, $type:ty) => {  //  raw pointers
        impl<T> PageAlignment for $type {
            #[inline]
            fn page_align_up(self) -> Self { ((self as usize + ((PAGE_SIZE as usize)-1)) & !((PAGE_SIZE as usize)-1)) as $type }

            #[inline]
            fn page_align_down(self) -> Self { (self as usize & !((PAGE_SIZE as usize)-1)) as $type }

            #[inline]
            fn is_page_aligned(&self) -> bool { ((*self as usize & !((PAGE_SIZE as usize)-1))) as $type >= *self }
        }
    }
}




impl_alignment!(usize);
impl_alignment!(u64);
impl_alignment!(T, *const T);
impl_alignment!(T, *mut T);


impl_page_alignment!(usize);
impl_page_alignment!(u64);
impl_page_alignment!(T, *const T);
impl_page_alignment!(T, *mut T);



/// Since all implementations are automatic, only one test is enough
#[test]
fn alignment() {

    use libtest::TestRng;

    let mut rand = TestRng::new();

    for _ in 0..10000 {

        let align = 1 << rand.next_range(..15);

        let value = rand.next_range(..usize::MAX-align);

        if value % align == 0 {
            assert!(value.is_aligned_to(align));
            assert!(value.align_down(align) == value);
            assert!(value.align_up(align) == value);
        } else {
            assert!(!value.is_aligned_to(align));

            let aligned_down = value - value % align;
            assert!(value.align_down(align) == aligned_down);
            assert!(value.align_up(align) == aligned_down + align);
        }
    }
}



/// Since all implementations are automatic, only one test is enough
#[test]
fn page_alignment() {
    use libtest::TestRng;

    let mut rand = TestRng::new();

    for _ in 0..100 {

        let value = rand.next_range(..usize::MAX-PAGE_SIZE);

        if value % PAGE_SIZE == 0 {
            assert!(value.is_page_aligned());
            assert!(value.page_align_down() == value);

            assert!(value.page_align_up() == value);
        } else {
            assert!(!value.is_page_aligned());

            let aligned_down = value - value % PAGE_SIZE;
            assert!(value.page_align_down() == aligned_down);
            assert!(value.page_align_up() == aligned_down + PAGE_SIZE);
        }
    }
}
