use core::{ops::Deref, slice};

use crate::extent_alloc::{ExtentAlloc, extent::{Extent, RawExtent, ScopedExtent}};

use alloc::{boxed::Box, vec::Vec};
use alloc::vec;



/// A batch of extents allocated by one allocation request
///
/// Extent ordering is not guaranteed
#[repr(transparent)]
pub struct Batch<'alloc, const ALIGN: usize, Ext, Alloc>(BatchInner<'alloc, ALIGN, Ext, Alloc>)
where Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>;



impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>>
Deref for Batch<'alloc, ALIGN, Ext, Alloc>  {

    type Target = [ScopedExtent<'alloc, ALIGN, Ext, Alloc>];

    fn deref(&self) -> &Self::Target {
        match &self.0 {
            BatchInner::Single(ext) => slice::from_ref(&ext),
            BatchInner::Slice(boxed) => boxed.as_ref(),
            //BatchInner::Vec(vec) => vec.as_slice(),
        }
    }
}

impl<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>>
Batch<'alloc, ALIGN, Ext, Alloc> {

    /// Constructs a new `Batch` from the given `ScopedExtent`
    pub(crate) const fn from_single(ext: ScopedExtent<'alloc, ALIGN, Ext, Alloc>) -> Self {
        Self(BatchInner::Single(ext))
    }

    /// Construct a new `Batch` from the given vector of `ScopedExtent`s
    pub(crate) const fn from_box(boxed: Box<[ScopedExtent<'alloc, ALIGN, Ext, Alloc>]>) -> Self {
        Self(BatchInner::Slice(boxed))
    }

    /// Returns a slice of the contained extents
    pub fn as_slice(&self) -> &[ScopedExtent<'alloc, ALIGN, Ext, Alloc>] {
        match &self.0 {
            BatchInner::Single(ext) => core::slice::from_ref(ext),
            BatchInner::Slice(boxed) => boxed.as_ref(),
            //BatchInner::Vec(vec) => vec.as_slice(),
        }
    }

    /// Converts the `Batch` into a vector of extents
    pub fn into_slice(self) -> Vec<ScopedExtent<'alloc, ALIGN, Ext, Alloc>> {
        match self.0 {
            BatchInner::Single(ext) => vec![ext],
            BatchInner::Slice(boxed) => boxed.into_vec(),
            //BatchInner::Vec(vec) => vec,
        }
    }


}

enum BatchInner<'alloc, const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Alloc: ExtentAlloc<ALIGN>> {
    Single(ScopedExtent<'alloc, ALIGN, Ext, Alloc>),
    Slice(Box<[ScopedExtent<'alloc, ALIGN, Ext, Alloc>]>)
    //Vec(Vec<ScopedExtent<'alloc, ALIGN, Ext, Alloc>>),
}
