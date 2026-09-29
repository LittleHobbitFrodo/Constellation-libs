use core::{hint::cold_path, marker::PhantomData, mem::ManuallyDrop, ptr::NonNull};

use crate::{assume, cold_panic, extent_alloc::{
    ExtentAlloc, ExtentAllocMarker, cache::{DefaultCache, ExtentCache, RefuelRequest, SourceAllocation}, extent::{AbstractExtent, Extent, InternalExtent, MutableExtent, RawExtent, RemainingExtents, ScopedExtent, TakenExtent}, layout::{ExtentLayout, LayoutDescriptor}, raw_alloc::{AlreadyInitialized, RawExtentAlloc},
    raw_alloc::PartDeallocError,
}};

use alloc::vec::Vec;


/// An implementation of an extent allocator that operates with
/// a predetermined page size, various sizes of allocated blocks,
/// and any number of memory blocks.
///
/// `CachedExtentAllocator` uses a cache to speed up lookup
/// for small extents. The cache can be implemented by the user,
/// so feel free to optimize it for your own usecase
///
/// This implementation is optimized only for allocation and deallocation.
/// - Deallocation of parts of allocated frames is also supported!
///
/// > **IMPORTANT**: `CachedExtentAllocator` uses the heap,
/// therefore the heap must be initialized before the allocator
///
/// > **SAFETY**: Due to implementation limitations, the generic constant
/// ALIGN must be a power of two and must be greater than or equal
/// to 64. Violating this rule will result in undefined behavior.
pub struct CachedExtentAllocator<const ALIGN: usize,
    Ext: Extent<ALIGN> + RawExtent<ALIGN> = InternalExtent<ALIGN>,
    Lay: LayoutDescriptor<ALIGN> = ExtentLayout<ALIGN>,
    Cache: ExtentCache<ALIGN, Lay> = DefaultCache<ALIGN, Lay>>
{
    alloc: RawExtentAlloc<ALIGN>,
    cache: Cache,
    _ext: PhantomData<Ext>,
    _lay: PhantomData<Lay>,
}

//  mark extent allocator
impl<const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Lay: LayoutDescriptor<ALIGN>, Cache: ExtentCache<ALIGN, Lay>>
ExtentAllocMarker for CachedExtentAllocator<ALIGN, Ext, Lay, Cache> {}

//  trait for use in generics
impl<const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Lay: LayoutDescriptor<ALIGN>, Cache: ExtentCache<ALIGN, Lay>>
ExtentAlloc<ALIGN> for CachedExtentAllocator<ALIGN, Ext, Lay, Cache> {

}


impl<const ALIGN: usize, Ext: Extent<ALIGN> + RawExtent<ALIGN>, Lay: LayoutDescriptor<ALIGN>, Cache: ExtentCache<ALIGN, Lay>>
CachedExtentAllocator<ALIGN, Ext, Lay, Cache> {

    /// Constructs an uninitialized `CachedExtentAllocator`
    pub const fn uninit() -> Self {
        Self {
            alloc: RawExtentAlloc::uninit(),
            cache: Cache::NEW,
            _ext: PhantomData,
            _lay: PhantomData
        }
    }

    pub(crate) fn inner(&self) -> &RawExtentAlloc<ALIGN> { &self.alloc }

    pub(crate) fn inner_mut(&mut self) -> &mut RawExtentAlloc<ALIGN> { &mut self.alloc }

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
    #[must_use]
    pub(crate) unsafe fn initialize<I>(&mut self, iter: I) -> Result<(), AlreadyInitialized>
    where I: IntoIterator<Item = TakenExtent<ALIGN, InternalExtent<ALIGN>>> {
        unsafe { self.alloc.initialize(iter) }?;
        self.cache.refuel(&mut self.alloc);
        Ok(())
    }

    /// Allocates an extent as described by the given `layout`
    ///
    /// The returned extent must be deallocated by calling the
    /// `deallocate()` function or transformed into a `OwnedExtent`
    #[inline(never)]
    #[must_use]
    pub fn allocate_contignous<'me>(&'me mut self, layout: Lay) -> Option<ScopedExtent<'me, ALIGN, Ext, Self>> {

        match self.cache.allocate_cached(layout.clone()) {
            Ok(Some(SourceAllocation { suitable, allocated })) => {
                //  allocated by the cache
                //
                // Now it is up to the allocator to maodify its tables to account with the
                // allocated extent as separate extent and not as a part of the cached extent

                assume!(allocated.size() == layout.size());
                assume!(allocated.address().alignment() >= layout.align());
                assume!(allocated.fits_into(&suitable));

                assume!(suitable.size() >= layout.size());
                assume!(suitable.address().alignment() >= layout.align());


                let suitable = MutableExtent::from_regular(suitable);
                let allocated = MutableExtent::from_regular(allocated);

                {   //  remove the suitable extent from used map

                    let used_map = unsafe { self.alloc.used_map_mut() };

                    if let None = used_map.remove(&suitable.address()) {
                        cold_panic!("failed to remove the suitable extent from used map")
                    }
                }

                //  remove allocated from suitable
                let RemainingExtents { left, right } = unsafe { allocated.clone().remove_from_unchecked(suitable) };

                let used_map = unsafe { self.alloc.used_map_mut() };


                //  re-insert the remaining space back into the used map

                if let Some(left) = left {
                    if let Some(_) = used_map.insert(left.address(), left.size()) {
                        //  recovery is impossible, there is no way to tell the cache
                        //  that the allocated extent has not beed really allocated
                        // so the cache would be out of sync
                        cold_panic!("failed to re-insert extent");
                    }
                }

                if let Some(right) = right {
                    if let Some(_) = used_map.insert(right.address(), right.size()) {
                        //  recovery is impossible, there is no way to tell the cache
                        //  that the allocated extent has not beed really allocated
                        // so the cache would be out of sync
                        cold_panic!("failed to re-insert extent");
                    }
                }

                let scoped = unsafe {
                    ScopedExtent::from_extent(Ext::new(allocated.address(), allocated.size()))
                };
                Some(scoped)
            }
            Ok(None) => {
                //  could not be allocated by the cache
                //      => allocate manually

                self.alloc.allocate_exact(layout).map(|ext| unsafe {
                    ScopedExtent::from_extent(Ext::new(ext.address(), ext.size()))
                } )
            },
            Err(_) => {
                //  the cache is out of memory
                //      => refuel it and allocate manually


                //  this path should not be taken often
                //  - Also the refuelling is expected to be expensive
                cold_path();

                self.cache.refuel(&mut self.alloc);

                self.alloc.allocate_exact(layout).map(|ext| unsafe {
                    ScopedExtent::from_extent(Ext::new(ext.address(), ext.size()))
                })
            }
        }
    }


    #[must_use]
    pub fn allocate<'me>(&'me mut self, layout: Lay) -> Option<Vec<ScopedExtent<'me, ALIGN, Ext, Self>>> {
        todo!("AllocMap::find_closest_to() needs to be implemented");
    }


    /// Deallocates the extent
    #[inline]
    #[must_use]
    pub fn deallocate<'me>(&mut self, ext: ScopedExtent<'me, ALIGN, Ext, Self>) -> Result<(), ()> {
        let ext = {
            let ext = ManuallyDrop::new(ext);
            MutableExtent::new(ext.address(), ext.size())
        };
        self.alloc.deallocate(ext)
    }

    /// Deallocates the `part` of the extent
    #[inline(always)]
    #[must_use]
    pub fn deallocate_part(&mut self, ext: MutableExtent<ALIGN>, part: MutableExtent<ALIGN>) -> Result<(), PartDeallocError> {
        self.alloc.deallocate_part(ext, part)
    }

}
