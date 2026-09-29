use core::{hint::cold_path, marker::PhantomData};

use alloc::vec::{Vec};

use crate::{cold_panic, extent_alloc::{
    ExtentAlloc, ExtentAllocMarker, extent::{self, Batch, Extent, InternalExtent, MutableExtent, RawExtent, ScopedExtent, TakenExtent}, layout::{ExtentLayout, LayoutDescriptor}, raw_alloc::{AlreadyInitialized, RawExtentAlloc},
    raw_alloc::PartDeallocError,
}};



/// An implementation of an extent allocator that operates with
/// a predetermined page size, various sizes of allocated blocks,
/// and any number of memory blocks.
///
/// `ExtentAllocator` does not use any kind of cache. If you want
/// to use any kind of allocation cache, use `CachedExtentAllocator`
///
/// > **IMPORTANT**: `ExtentAllocator` uses the heap,
/// therefore the heap must be initialized first
///
/// This implementation is optimized only for allocation and deallocation.
/// - Deallocation of parts of allocated frames is also supported!
///
/// > **SAFETY**: Due to implementation limitations, the generic constant
/// ALIGN must be a power of two and must be greater than or equal
/// to 64. Violating this rule will result in undefined behavior.
pub struct ExtentAllocator<const ALIGN: usize,
    Ext: Extent<ALIGN> + RawExtent<ALIGN> = InternalExtent<ALIGN>,
    Lay: LayoutDescriptor<ALIGN> = ExtentLayout<ALIGN>>
{
    alloc: RawExtentAlloc<ALIGN>,
    _ext: PhantomData<Ext>,
    _lay: PhantomData<Lay>,
}


//  mark extent allocator
impl<const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Lay: LayoutDescriptor<ALIGN>>
ExtentAllocMarker for ExtentAllocator<ALIGN, Ext, Lay> {}

//  trait for use in generics
impl<const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Lay: LayoutDescriptor<ALIGN>>
ExtentAlloc<ALIGN> for ExtentAllocator<ALIGN, Ext, Lay> {}

impl<const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Lay: LayoutDescriptor<ALIGN>>
ExtentAllocator<ALIGN, Ext, Lay> {

    /// Constructs an uninitialized `CachedExtentAllocator`
    pub const fn uninit() -> Self {
        Self {
            alloc: RawExtentAlloc::uninit(),
            _ext: PhantomData,
            _lay: PhantomData
        }
    }

    pub(crate) fn inner(&self) -> &RawExtentAlloc<ALIGN> { &self.alloc }

    pub(crate) unsafe fn inner_mut(&mut self) -> &mut RawExtentAlloc<ALIGN> { &mut self.alloc }


    /// Initializes the `ExtentAllocator` by consuming an iterator that yields `TakenExtent`
    /// - `TakenExtent` is thin abstraction over `RawExtent` that
    /// indicates whether the extent is already used or still free
    ///
    /// # Safety
    /// This function itself is not unsafe, but if the iterator passed to it provides
    /// invalid data, the system may fall into undefined behavior. For this reason,
    /// the caller must guarantee that the passed extents are valid at least for the lifetime
    /// of the allocator and that they do not overlap with one another.
    ///
    /// The `AlreadyInitialized` error is returned only if this function has already been called in this instance
    #[cold]
    #[inline(never)]
    #[must_use]
    pub(crate) unsafe fn initialize<I>(&mut self, iter: I) -> Result<(), AlreadyInitialized>
    where I: IntoIterator<Item = TakenExtent<ALIGN, InternalExtent<ALIGN>>> {
        unsafe { self.alloc.initialize(iter) }
    }

    /// Allocates an extent as described by the given `layout`
    ///
    /// The returned extent must be deallocated by calling the
    /// `deallocate()` function or transformed into a `OwnedExtent`
    #[inline]
    #[must_use]
    pub fn alloc_contignous<'me>(&'me mut self, layout: Lay) -> Option<ScopedExtent<'me, ALIGN, Ext, Self>> {
        self.alloc.allocate_exact(layout).map(|ext| unsafe {
            ScopedExtent::<'me, ALIGN, Ext, Self>::from_extent(Ext::new(ext.address(), ext.size()))
        } )
    }

    /// Allocates any amount of extents to fulfill the given `layout`
    ///
    /// This function tries to allocate the extent as described by the `layout`.
    /// If the allocation fails, it then proceeds to split the extent and
    /// allocate the newly created layouts. This process repeats until the
    /// layout is fulfilled or the system run out of memory
    #[inline(never)]
    #[must_use]
    pub fn alloc<'me>(&'me mut self, mut layout: Lay) -> Option<Batch<'me, ALIGN, Ext, Self>> {

        todo!("AllocMap::find_closest_to() needs to be implemented");

        /*//  try to allocate, split the layout if fails
        let mut lays = match self.alloc.allocate_exact(layout.clone()) {
            Some(ext) => {
                let ext = unsafe { ScopedExtent::from_extent(Ext::new(ext.address(), ext.size())) };
                return Some(Batch::from_single(ext))
            },
            None => match layout.split() {
                Some(lay) => Vec::from([layout, lay]),
                None => {   //  split failed => layout cannot be allocated
                    cold_path();
                    return None
                }
            }
        };

        //  this vector will be returned on success
        let mut exts = Vec::with_capacity(2);

        //  try to allocate each of the layouts stored in `lays`
        while let Some(mut layout) = lays.pop() {

            match self.alloc.allocate_exact(layout.clone()) {
                Some(ext) => {  //  success: push into exts
                    let ext = unsafe {
                        ScopedExtent::from_extent(Ext::new(ext.address(), ext.size()))
                    };

                    exts.push(ext)
                },
                None => match layout.split() {  //  split and append to lays
                    Some(lay) => {
                        lays.push(layout);
                        lays.push(lay);
                    },
                    None => {   //  split failed => layout cannot be allocated
                        cold_path();

                        //  recovery: deallocate all allocate extents
                        for ext in exts {
                            let ext = unsafe {
                                InternalExtent::from_extent(ext.into_extent())
                            };

                            match self.alloc.deallocate(ext) {
                                Ok(_) => { /* OK */ },
                                Err(_) => cold_panic!("deallocation failed")
                            }
                        }

                        return None
                    }
                }
            }
        }

        Some(Batch::from_vec(exts))*/

    }


    /// Deallocates the extent
    #[inline(always)]
    #[must_use]
    pub fn deallocate(&mut self, ext: Ext) -> Result<(), ()> {
        self.alloc.deallocate(MutableExtent::from_regular(ext))
    }


    /// Deallocates the `part` of the extent
    #[inline(always)]
    #[must_use]
    pub fn deallocate_part(&mut self, ext: MutableExtent<ALIGN>, part: MutableExtent<ALIGN>) -> Result<(), PartDeallocError> {
        self.alloc.deallocate_part(ext, part)
    }


}
