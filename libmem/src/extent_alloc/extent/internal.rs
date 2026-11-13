use core::{mem::ManuallyDrop, num::NonZero};

use crate::{AlignedNonNull, extent_alloc::extent::{BuiltinExt, RawExtent, Extent}};



/// An extent type used by the extent allocator under the hood
#[derive(Clone)]
#[repr(C)]
pub struct InternalExtent<const ALIGN: usize> {
    start: AlignedNonNull<u64, ALIGN>,
    size: NonZero<u64>,
}


//  mark extent defined within libmem
impl<const ALIGN: usize> BuiltinExt for InternalExtent<ALIGN> {}

//  mark constructible extent
impl<const ALIGN: usize> RawExtent<ALIGN> for InternalExtent<ALIGN> {
    #[inline(always)]
    fn new(address: AlignedNonNull<u64, ALIGN>, pages: NonZero<u64>) -> Self {
        Self { start: address, size: pages }
    }
}

impl<const ALIGN: usize> InternalExtent<ALIGN> {

    /// Converts the extent into `InternalExtent`
    /// - The given extent will not be dropped
    #[inline(always)]
    pub fn from_extent<Ext: Extent<ALIGN>>(ext: Ext) -> Self {
        let ext = ManuallyDrop::new(ext);
        Self { start: ext.address(), size: ext.size() }
    }

}


impl<const ALIGN: usize> Extent<ALIGN> for InternalExtent<ALIGN> {
    #[inline(always)]
    fn address(&self) -> AlignedNonNull<u64, ALIGN> {
        self.start
    }

    #[inline(always)]
    fn size(&self) -> NonZero<u64> { self.size }
}


impl<const ALIGN: usize> core::fmt::Debug for InternalExtent<ALIGN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            write!(f, "IntExt {{ {:p} -> {:p} }}", self.address(), self.end_address())
        } else {
            write!(f, "IntExt {{ {:p}, {} }}", self.address(), self.size())
        }
    }
}
