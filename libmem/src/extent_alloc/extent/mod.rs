use crate::{AlignedNonNull, extent_alloc::ExtentAllocMarker};
use core::num::NonZero;


mod internal;
pub use internal::InternalExtent;

mod scoped;
pub use scoped::ScopedExtent;

mod batch;
pub use batch::Batch;

mod mutable;
pub use mutable::{MutableExtent, RemainingExtents};

/// An extent describes a chunk of memory by its starting address and its size
///
/// > Consider implementing the `ConstructibleExtent` which
/// may be required by some extent allocator functions
///
/// > NOTE: Functions that are automatically implemented
/// by this trait are free to use `debug_assert!`ions
#[allow(private_bounds)]
pub trait Extent<const ALIGN: usize>
where Self: Sized {

    /// Indicates whether `self` and `other` are right next to each
    /// other and are then "touching" each other
    /// - Returns `false` if the extents are overlapping
    #[inline]
    fn is_touching(&self, other: &'_ Self) -> bool {
        //  NOTE: Extents cannot be overlapping
        self.end_address() == other.address()     //  |--SELF--||--OTHER--|
        || self.address() == other.end_address()  //  |--OTHER--||--SELF--|
    }

    /// Returns the starting address of the extent
    fn address(&self) -> AlignedNonNull<u64, ALIGN>;

    /// Calculates the ending address of the frame
    #[inline]
    fn end_address(&self) -> AlignedNonNull<u64, ALIGN> {
        self.address().aligned_add_saturate(self.size().get())
    }

    /// Returns the size in pages
    fn size(&self) -> NonZero<u64>;

    /// Indicates whether `self` fits into `other`
    /// - Uses starting and ending addresses of
    /// the extents to determine the result
    /// - Returns `true` if the extents are equivalent
    ///   - Therefore `assert!(extent.fits_into(extent.clone()))` will pass
    #[inline]
    fn fits_into(&self, other: &'_ Self) -> bool {
        self.address() >= other.address() && self.end_address() <= other.end_address()
    }

    /// Indicates whether the two extents are overlapping
    #[inline]
    fn is_overlapping(&self, other: &'_ Self) -> bool {
        !(self.end_address() <= other.address() || self.address() >= other.end_address())
    }
}


/// Marks raw extents that can be used internally by the allocator
pub trait RawExtent<const ALIGN: usize> {
    fn new(address: AlignedNonNull<u64, ALIGN>, pages: NonZero<u64>) -> Self;
}

/// Marks extent types that are defined within the `libmem` crate
pub(crate) trait BuiltinExt {}

/// Marks abstraction types
/// - Raw extents are unmarked by this trait
pub(crate) trait AbstractExtent<'alloc, const ALIGN: usize, Ext: Extent<ALIGN>, Alloc: ExtentAllocMarker>
where Self: BuiltinExt {
    //fn from_ext_with_alloc(ext: Ext, alloc: &'alloc Alloc) -> Self;
}



/// Inidicates whether the extent is used or free
#[derive(Clone)]
pub enum TakenExtent<const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>> {
    /// An extent that is not used and/or allocated
    Free(Ext),
    /// An extent that is used and/or allocated
    Used(Ext)
}
