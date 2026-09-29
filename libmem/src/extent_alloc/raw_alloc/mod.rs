use alloc::collections::BTreeMap;

use crate::{AlignedNonNull, assume, extent_alloc::{
    extent::{Extent, InternalExtent, MutableExtent, RemainingExtents, ScopedExtent, TakenExtent}, helpers::SuitableExtent, layout::LayoutDescriptor,
}};
use super::helpers::AllocMap;
use core::{fmt::Alignment::Right, hint::cold_path, num::NonZero};
use crate::cold_panic;


/// The raw extent allocator internally used by the `ExtentAllocator` and `CachedExtentAllocator`
///
/// # Safety
/// Due to implementation limitations, the generic constant ALIGN must be a power of two
/// and must be greater than or equal to 64. Violating this rule will result in undefined behavior.
#[repr(C)]
pub struct RawExtentAlloc<const ALIGN: usize> {
    free: AllocMap<ALIGN>,
    used: BTreeMap<AlignedNonNull<NonZero<u64>, ALIGN>, NonZero<u64>>
}

impl<const ALIGN: usize> RawExtentAlloc<ALIGN> {
    /// Constructs a new uninitialized `RawExtentAllocator`
    pub const fn uninit() -> Self {
        //  evaluates the assertion at comptime
        let _ = Self::ALIGN_MUST_BE_POWER_OF_TWO_AND_GREATER_THAN_63;

        Self { used: BTreeMap::new(), free: AllocMap::uninit() }
    }

    /// The generic `ALIGN` must be greater than or equal to 64 and must be a power of two
    const ALIGN_MUST_BE_POWER_OF_TWO_AND_GREATER_THAN_63: () = assert!(ALIGN >= 64 && ALIGN.is_power_of_two());
}


impl<const ALIGN: usize> RawExtentAlloc<ALIGN> {

    /// Returns a reference to the free map
    #[inline(always)]
    pub(crate) fn free_map(&self) -> &AllocMap<ALIGN> { &self.free }

    /// Returns a mutable reference to the free map
    #[inline(always)]
    pub(crate) unsafe fn free_map_mut(&mut self) -> &mut AllocMap<ALIGN> { &mut self.free }


    /// Returns a reference to the used map
    #[inline(always)]
    pub(crate) fn used_map(&self) -> &BTreeMap<AlignedNonNull<NonZero<u64>, ALIGN>, NonZero<u64>> {
        &self.used
    }


    /// Returns a mutable reference to the used map
    #[inline(always)]
    pub(crate) unsafe fn used_map_mut(&mut self) -> &mut BTreeMap<AlignedNonNull<NonZero<u64>, ALIGN>, NonZero<u64>> {
        &mut self.used
    }



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

        if !self.free.size_index.is_empty() || !self.free.map.is_empty() || !self.used.is_empty() {
            return Err(AlreadyInitialized)
        }

        for ext in iter.into_iter() {
            match ext {
                TakenExtent::Free(ext) => {
                    if let Err(_) = self.free.insert_extent_fragmented(&MutableExtent::from_regular(ext)) {
                        cold_panic!("found overlapping extents")
                    }
                },
                TakenExtent::Used(ext) => {

                    //  insert into the used map
                    if let Some(_) = self.used.insert(ext.address(), ext.size()) {
                        cold_panic!("found overlapping extents")
                    }
                },
            }
        }

        Ok(())
    }


    /// Allocates one contignous extent as described by the `layout`
    /// - The returned extent needs to be deallocated manually
    #[inline(never)]
    #[must_use]
    pub fn allocate_exact(&mut self, layout: impl LayoutDescriptor<ALIGN>) -> Option<InternalExtent<ALIGN>> {
        assume!(layout.align().is_power_of_two());

        let SuitableExtent { suitable, allocated } = self.free.find_suitable(layout)?;
        assume!(allocated.fits_into(&suitable));


        //  remove the suitable extent from the free map
        if let Err(_) = self.free.extract_exact_extent(suitable.clone()) {
            cold_path();
            return None
        }



        //  remove allocated from suitable
        let RemainingExtents { left, right } = unsafe { allocated.clone().remove_from_unchecked(suitable.clone()) };



        {   //  re-insert left and right into the free map

            if let Some(left) = &left {
                if let Err(_) = self.free.insert_extent_fragmented(left) {
                    //  recovery: re-insert suitable to the free map
                    if let Err(_) = self.free.insert_extent_fragmented(&suitable) {
                        cold_panic!("could not re-insert suitable")
                    }
                }
            }

            if let Some(right) = right {
                if let Err(_) = self.free.insert_extent_fragmented(&right) {
                    //  recovery: remove left and insert the suitable into the free map
                    cold_path();

                    if let Some(left) = left {
                        if let Err(_) = self.free.extract_exact_extent(left) {
                            cold_panic!("ExtentAllocator recovery failed: failed to remove left extent")
                        }
                    }

                    if let Err(_) = self.free.insert_extent_fragmented(&suitable) {
                        cold_panic!("ExtentAllocator recovery failed: could not re-insert suitable")
                    }

                    return None
                }
            }
        }

        //  insert allocated into used map
        if let Some(_) = self.used.insert(allocated.address(), allocated.size()) {
            cold_panic!("could not insert allocated extent into used map");
        }

        Some(allocated.into_regular())

    }


    /// Allocates one contignous extent as described by the `layout`. If there
    /// is no extent that can fit the `layout`, this function will instead
    /// choose an extent that is closest to meeting the required `layout`
    #[inline(never)]
    #[must_use]
    pub fn allocate_closest(&mut self, layout: impl LayoutDescriptor<ALIGN>) -> Option<InternalExtent<ALIGN>> {
        assume!(layout.align().is_power_of_two());

        todo!("AllocMap::find_closest_to() is unimplemented");
    }


    /// Deallocates the extent
    ///
    /// The passed extent must have been allocated by this instance
    /// and must be in the same state as it was at the time of allocation
    #[must_use]
    pub fn deallocate<'me>(&mut self, ext: MutableExtent<ALIGN>) -> Result<(), ()> {

        //  remove from the used map
        match self.used.remove(&ext.address()) {
            Some(removed_ext_size) => debug_assert!(removed_ext_size == ext.size()),
            None => {
                //  this path is unexpected
                cold_path();
                return Err(())
            }
        }

        //  insert the extent into the free map
        if let Err(_) = self.free.insert_extent(ext.clone()) {

            //  recovery: re-insert ext into the used map
            if let Some(_) = self.used.insert(ext.address(), ext.size()) {
                cold_panic!("ExtentAllocator recovery failed: failed to re-insert the deallocated extent")
            }

            return Err(())
        }

        Ok(())
    }


    /// Deallocates the `part` of the given extent
    #[inline(never)]
    #[must_use]
    pub fn deallocate_part(&mut self, ext: MutableExtent<ALIGN>, part: MutableExtent<ALIGN>) -> Result<(), PartDeallocError> {

        if !part.fits_into(&ext) { return Err(PartDeallocError::PartDoesNotFit) }

        //  remove the extent from the used map
        match self.used.remove(&ext.address()) {
            Some(removed_ext_size) => debug_assert!(removed_ext_size == ext.size()),
            None => {
                //  this path is unexpected
                cold_path();
                return Err(PartDeallocError::UnregisteredExtent)
            }
        }

        //  remove part from the extent
        let RemainingExtents { left, right } = unsafe {
            //  SAFETY: it is asserted above that part fots into the bigger extent
            part.clone().remove_from_unchecked(ext.clone())
        };


        {   //  re-insert remains into the used map
            if let Some(left) = &left {
                if let Some(_) = self.used.insert(left.address(), left.size()) {
                    //  recovery: re-insert ext into the used map
                    cold_path();

                    if let Some(_) = self.used.insert(ext.address(), ext.size()) {
                        cold_panic!("ExtentAllocator recovery failed: could not re-insert ext into the used map")
                    }

                    return Err(PartDeallocError::InternalError)
                }
            }

            if let Some(right) = right {
                if let Some(_) = self.used.insert(right.address(), right.size()) {
                    //  recovery: remove left and insert ext into the used map
                    cold_path();

                    if let Some(left) = left {
                        if let None = self.used.remove(&left.address()) {
                            cold_panic!("ExtentAllocator recovery failed: could not remove left extent")
                        }
                    }

                    if let Some(_) = self.used.insert(ext.address(), ext.size()) {
                        cold_panic!("ExtentAllocator recovery failed: failed to insert extent");
                    }

                    return Err(PartDeallocError::InternalError)
                }
            }
        }

        //  insert part into the free map
        if let Err(_) = self.free.insert_extent_fragmented(&part) {
            cold_panic!("failed to insert the deallocated part into the free map")
        }

        Ok(())
    }


}


/// Indicates that the extent allocator has already been initialized
pub struct AlreadyInitialized;


/// Returned by the `deallocate_part()` function
#[repr(u32, align(4))]
//  NOTE: this enum is directly corresponding to `crate::extent_alloc::helpers::PartExtractionError`
pub enum PartDeallocError {
    /// The given extent was not found in allocator
    UnregisteredExtent = 0,
    /// The part extent does not fit into the bigger extent
    PartDoesNotFit = 1,
    /// Something went wrong, but the system was able to recover
    InternalError = 2,
}
