use core::{fmt::Debug, marker::PhantomData};
use core::num::NonZero;

use crate::extent_alloc::{ExtentAlloc, extent::{AbstractExtent, BuiltinExt, Extent, /*OwnedExtent,*/ RawExtent}};




/// An abstraction over a raw extent type that makes sure that
/// the unnderlying extent does not outlive its allocator
#[allow(private_bounds)]
#[derive(Clone)]
#[repr(C)]
pub struct ScopedExtent<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>> {
    ext: Ext,
    _alloc: PhantomData<&'alloc Alloc>
}

//  mark extent defined within libmem
impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>>
BuiltinExt for ScopedExtent<'alloc, ALIGN, Ext, Alloc> {}

//  mark abstraction over raw extent
impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>>
AbstractExtent<'alloc, ALIGN, Ext, Alloc> for ScopedExtent<'alloc, ALIGN, Ext, Alloc> {

    /*#[inline(always)]
    fn from_ext_with_alloc(ext: Ext, _: &'alloc Alloc) -> Self {
        Self { ext, _alloc: PhantomData }
    }*/
}


impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>>
core::ops::Deref for ScopedExtent<'alloc, ALIGN, Ext, Alloc> {
    type Target = Ext;
    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.ext }
}


impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>> ScopedExtent<'alloc, ALIGN, Ext, Alloc> {

    /// Constructs an `ScopedExtent` from the given raw extent
    pub unsafe fn from_extent(ext: Ext) -> Self {
        Self { ext, _alloc: PhantomData }
    }

    /// Sets the inner raw extent free
    ///
    /// # Safety
    /// It is up to the caller to guarantee that the
    /// returned extent will not outlive its allocator
    pub unsafe fn into_extent(self) -> Ext {
        let Self { ext, _alloc } = self;
        ext
    }

    /*/// Converts the `ScopedExtent` into an `OwnedExtent`
    ///
    /// # Safety
    /// This function takes a reference to an allocator, it is up to the caller to
    /// guarantee that the passed allocator is the one that has allocated the `ScopedExtent`
    pub(crate) unsafe fn into_owned(self, alloc: &'alloc Alloc) -> OwnedExtent<'alloc, ALIGN, Ext, Alloc> {
        unsafe {
            OwnedExtent::from_extent_and_alloc(self.into_extent(), alloc)
        }
    }*/
}

impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>>
Extent<ALIGN> for ScopedExtent<'alloc, ALIGN, Ext, Alloc> {

    /// Indicates whether `self` and `other` are right next to each
    /// other and are then "touching" each other
    /// - Returns `false` if the extents are overlapping
    #[inline(always)]
    fn is_touching(&self, other: &'_ Self) -> bool { self.ext.is_touching(&other.ext) }

    /// Returns the starting address of the extent
    #[inline(always)]
    fn address(&self) -> crate::AlignedNonNull<u64, ALIGN> {
        self.ext.address()
    }

    /// Calculates the ending address of the frame
    #[inline(always)]
    fn end_address(&self) -> crate::AlignedNonNull<u64, ALIGN> {
        self.ext.end_address()
    }

    /// Returns the size in pages
    #[inline(always)]
    fn size(&self) -> NonZero<u64> { self.ext.size() }

    /// Indicates whether `self` fits into `other`
    /// - Uses starting and ending addresses of
    /// the extents to determine the result
    /// - Returns `true` if the extents are equivalent
    ///   - Therefore `assert!(extent.fits_into(extent.clone()))` will pass
    #[inline(always)]
    fn fits_into(&self, other: &'_ Self) -> bool {
        self.ext.fits_into(&other.ext)
    }

    /// Indicates whether the two extents are overlapping
    #[inline(always)]
    fn is_overlapping(&self, other: &'_ Self) -> bool {
        self.ext.is_overlapping(&other.ext)
    }
}


impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>>
Debug for ScopedExtent<'alloc, ALIGN, Ext, Alloc> where Ext: Debug {

    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "ScopedExtent {{ {:#?} }}", self.ext)
        } else {
            write!(f, "ScopedExtent {{ {:?} }}", self.ext)
        }
    }
}
