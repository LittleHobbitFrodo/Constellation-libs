use alloc::vec::Vec;

use super::{AddressMap, SizeMap};
use crate::{AlignedNonNull, Alignment, cold_panic, assume, extent_alloc::raw_alloc::PartDeallocError};

use crate::extent_alloc::extent::{Batch, RawExtent, RemainingExtents, ScopedExtent};

use core::ops::RangeBounds;
use core::ops::RangeInclusive;
use core::{hint::cold_path, num::NonZero};

use core::cmp::Ordering;

use crate::extent_alloc::{
    extent::{Extent, MutableExtent}, layout::LayoutDescriptor};


#[cfg(test)]
pub(crate) mod tests;

/// Invokes `cold_panic!()` with custom panic message
macro_rules! cold_recovery_panic {
    ($msg:literal) => {{
        cold_panic!("ExtentAllocator error recovery failed: {}", $msg)
    }};
}


#[derive(Debug)]
/// A self of either free or used extents
pub(crate) struct AllocMap<const ALIGN: usize> {
    /// Resolves addresses to extent sizes
    pub(crate) map: AddressMap<ALIGN>,

    /// Resolves extent size -> align -> addresses
    pub(crate) size_index: SizeMap<ALIGN>,
}

impl<const ALIGN: usize> AllocMap<ALIGN> {
    pub const fn uninit() -> Self {
        Self { map: AddressMap::new(), size_index: SizeMap::new() }
    }


    /*/// Finds a suitable extent that satisfies the given layout
    ///
    /// This function achieves this as follows:
    /// 1. Restricts `size_index` based on the given range
    /// 2. Creates a range based on the passed `align_range`
    ///     - `Less` => `..=layout.align()`
    ///     - `Equal` => `layout.align()..=layout.align()`
    ///     - `Greater` => `layout.align()..`
    /// 3. It searches the `AlignMaps` based on the created range
    pub fn find_in_range<Lay: LayoutDescriptor<ALIGN>>(&self, layout: &Lay, size_range: impl RangeBounds<NonZero<u64>>, align_range: Ordering) -> Option<SuitableExtent<ALIGN>> {

        let align_range = make_range::<ALIGN, Lay>(&layout, align_range);


        for (size, by_max_align) in self.size_index.range(size_range) {

            for (_max_align, addresses) in by_max_align.range(align_range.clone()) {

                //  max_align contains the maximum alignment that any address within the extent can have
                for address in addresses.iter() {
                    if let Some(suitable) = SuitableExtent::from_suitable(MutableExtent::new(*address, *size), layout) {
                        return Some(suitable)
                    }
                }
            }
        }

        return None;    //  no suitable extent found


        fn make_range<const ALIGN: usize, Lay: LayoutDescriptor<ALIGN>>(layout: &Lay, order: Ordering) -> RangeInclusive<NonZero<u64>> {

            const SMALLEST: NonZero<u64> = unsafe { NonZero::new_unchecked(1) };
            const BIGGEST: NonZero<u64> = unsafe { NonZero::new_unchecked(u64::MAX) };

            match order {
                Ordering::Less => (SMALLEST..=layout.align()).into(),
                Ordering::Equal => {
                    let align = layout.align();
                    align..=align
                },
                Ordering::Greater => layout.align()..=BIGGEST,
            }
        }

    }*/


    /// Finds a suitable extent to satisfy the given `layout`
    ///
    /// Returns a pair of extents, deemed `suitable` and `allocated`. The `suitable`
    /// extent is the one found by this function. It is guaranteed to satisfy
    /// both the alignment and size contraints. The `allocated` extent marks the
    /// part of the `suitable` extent that does exactly fulfill the given layout.
    /// It is guaranteed that the `allocated` extent fits into the `suitable` extent
    ///
    /// This function searches the `layout.size()..` range
    #[inline(always)]
    pub fn find_suitable(&self, layout: &impl LayoutDescriptor<ALIGN>) ->  Option<SuitableExtent<ALIGN>> {

        for (size, by_max_align) in self.size_index.range(layout.size()..) {

            for (_max_align, addresses) in by_max_align.range(layout.align()..) {

                //  max_align contains the maximum alignment that any address within the extent can have

                for address in addresses.iter() {
                    if let Some(suitable) = SuitableExtent::from_suitable(MutableExtent::new(*address, *size), layout) {
                        return Some(suitable)
                    }
                }
            }
        }

        None    //  no suitable extent found
    }


    /// Finds a suitable extent that satisfies the specified `layout`.
    /// If no such extent can be found, this function returns the extent
    /// that is closest to the given `layout` (i.e., the one that meets
    /// the alignment requirement and is closest in size to the requested size).
    /// In this case, the returned extent is always smaller than requested.
    ///
    /// To verify that the returned extent is fully suitable, simply compare
    /// the allocated extent size with the `layout` size (the allocated extent always meets the
    /// alignment requirement). If the allocated extent size is greater than or equal
    /// to the requested size, the extent is considered suitable.
    ///
    /// This function searches the `..layout.size()` range
    #[inline(never)]
    pub fn find_closest_to(&self, layout: &impl LayoutDescriptor<ALIGN>) -> Option<SuitableExtent<ALIGN>> {

        //  iterate sizes greater than or equal to `layout.size()`
        if let Some(suitable) = self.find_suitable(layout) {
            return Some(suitable)
        }


        //  reverse iterate sizes smaller than `layout.size()`
        for (size, align_map) in self.size_index.range(..layout.size()).rev() {

            for (_, addresses) in align_map.range(layout.align()..) {

                if let Some(suitable) = SuitableExtent::from_suitable(MutableExtent::new(*addresses.first()?, *size), layout) {
                    return Some(suitable)
                }

            }
        }

        None

    }


    /// Removes the extent from the map
    ///
    /// Returned `Err` indicates that the extent was not found
    pub fn extract_exact(&mut self, extent: MutableExtent<ALIGN>) -> Result<(), ()> {

        let by_max_align = match self.size_index.get_mut(&extent.size()) {
            Some(by_size) => by_size,
            None => return Err(()),
        };

        //  remove the address from size_index
        //  - addresses are unique, so this would eventually have to happen anyway
                //match by_align.get_mut(&(&extent.end_address().max_alignment())) {
        match by_max_align.get_mut(&extent.find_max_alignment()) {
            Some(addresses) => {
                if addresses.remove(&extent.address()) {

                    if addresses.is_empty() {
                        _ = by_max_align.remove(&extent.size());
                    }

                } else {
                    cold_path();
                    //  address not found
                    return Err(())
                }
            },
            None => {   //  alignment not found
                return Err(())
            },
        };

        //  remove the extent from the map
        //  - addresses are unique, so this would eventually have to happen anyway
        match self.map.remove(&extent.address()) {
            Some(size) => {
                //  helps with debugging
                debug_assert!(size == extent.size(), "map contains corrupted record: unexpected extent size");
                Ok(())
            },
            None => {
                cold_panic!("extent is not registered in the map, but was found in the size_index, this state is considered corrupted")
            },
        }
    }



    /// Same as `extract_exact()`, but this splits the extent based on
    /// the `layout` requirements and inserts the remaining space into the map again
    ///
    /// An returned `Err`or indicates that the extent was not found
    ///
    /// # Parameters
    /// - `extent` is the extent found in the passed `map` that satisfies `layout`
    ///   - It is thus needed to be properly aligned and at least the same size as dictated by the `layout`
    pub(crate) fn extract_and_split(&mut self, mut extent: MutableExtent<ALIGN>, layout: impl LayoutDescriptor<ALIGN>) -> Result<MutableExtent<ALIGN>, ()> {

        //  used in recovery
        let original = extent.clone();

        if let Err(_) = self.extract_exact(extent.clone()) {
            cold_path();
            return Err(())
        }

        debug_assert!(extent.size() >= layout.size());

        match unsafe { extent.split_unchecked(layout.size()) } {
            Some(put_back) => {

                //  create new entries in size_index and map
                //  DEfragmentation is undesired

                if let Err(_) = self.insert_fragmented(&put_back) {
                    //  recovery: re-insert original
                    cold_path();
                    match self.insert_fragmented(&original) {
                        Ok(_) => return Err(()),
                        Err(_) => cold_recovery_panic!("failed to re-insert the original extent"),
                    }
                }
            },
            None => {   //  OK
                //  layout.size() == extent.size() => no entry to put back
                //  - entries from both size_index and map are already removed
            },
        }

        Ok(extent)

    }


    /// Extracts the `base` extent from the map, removes the `part` from it and inserts the remaining space back into the map
    ///
    /// This function is made to simplify `allocate_exact()`, `allocate_closest()` and simillar functions
    ///
    /// This function is tested by testing the `RawExtentAlloc::allocate_exact()` function
    pub(crate) fn extract_and_remove_from(&mut self, base: MutableExtent<ALIGN>, part: &'_ MutableExtent<ALIGN>) -> Result<(), ()> {
        //  remove the suitable extent from the free map
        if let Err(_) = self.extract_exact(base.clone()) {
            cold_path();
            return Err(())
        }

        //todo!("unit test");


        //  remove allocated from suitable
        let RemainingExtents { left, right } = unsafe { part.clone().remove_from_unchecked(base.clone()) };



        {   //  re-insert left and right into the free map

            if let Some(left) = &left {
                if let Err(_) = self.insert_fragmented(left) {

                    //  recovery: re-insert suitable to the free map
                    cold_path();

                    if let Err(_) = self.insert_fragmented(&base) {
                        cold_panic!("could not re-insert suitable")
                    }
                }
            }

            if let Some(right) = right {

                if let Err(_) = self.insert_fragmented(&right) {

                    //  recovery: remove left and insert the suitable into the free map
                    cold_path();

                    if let Some(left) = left {
                        if let Err(_) = self.extract_exact(left) {
                            cold_panic!("ExtentAllocator recovery failed: failed to remove left extent")
                        }
                    }

                    if let Err(_) = self.insert_fragmented(&base) {
                        cold_panic!("ExtentAllocator recovery failed: could not re-insert suitable")
                    }

                    return Err(())
                }
            }
        }

        Ok(())
    }


    /// Inserts the extent into the `map` and `size_index`
    /// - Does not take care of fragmentation
    pub(crate) fn insert_fragmented(&mut self, ext: &MutableExtent<ALIGN>) -> Result<(), ()> {

        //  insert into the map
        //      addresses are unique => fail if alread in there
        if let Some(_) = self.map.insert(ext.address(), ext.size()) {
            //panic!("conflicting stuff");
            return Err(())
        }

        let by_mut_align = self.size_index.entry(ext.size()).or_default();

        //let addresses = by_mut_align.entry(ext.end_address().max_alignment()).or_default();
        let addresses = by_mut_align.entry(ext.find_max_alignment()).or_default();
        if !addresses.insert(ext.address()) {
            //  insertion failed, but the extent is already in the map
            //  - Remove the extent from the map to keep the system sound
            if let None = self.map.remove(&ext.address()) {
                cold_recovery_panic!("failed to remove the extent from the map")
            }
                //.expect("recovery failed: failed to remove the extent from the map");
            return Err(())
        }

        Ok(())
    }


    /// Inserts the extent into the given map and performs defragmentation routine if needed
    ///
    /// # Defrag. routine
    /// The defragmentation routine works this way:
    /// 1. Checks whether the inserted extent would touch any neighboring extent (if there are any)
    /// 2. Extracts the neighboring extents that are touching the inserted extent
    /// 3. Merge the extracted extents into the inserted one
    /// 4. Inserts the merged extent
    pub(crate) fn insert_defragment(&mut self, mut ext: MutableExtent<ALIGN>) -> Result<(), ()> {

        let ext_end_address = ext.end_address();

        //  check for the neighbor with higher address
        if let Some(right_size) = self.map.remove(&ext_end_address) {
            //  `right` is the right neighbor of ext
            //      => merge `ext` and `right`
            //
            //  NOTE: the `right` extent is already removed from the map

            let right = MutableExtent::new(ext_end_address, right_size);

            assume!(ext.address() < right.address());
            assume!(ext.is_touching(&right));

            //  remove ONLY FROM `size_index`
            if let Err(_) = remove_sized_index(&mut self.size_index, right.clone()) {
                cold_path();

                //  recovery: reinsert to the map
                if let Some(_) = self.map.insert(ext_end_address, right_size) {
                    cold_recovery_panic!("failed to reinsert the extent");
                }

            }

            unsafe {
                ext.merge_unchecked(right);
            }
        }


        //  check for the neighbor with lower address
        match self.map.range(..ext_end_address).next_back().map(|(a, s)| (*a, *s) ) {
            Some((left_addr, left_size)) if ext.address() == left_addr.aligned_add_saturate(left_size.get()) => {
                //  ext is touching the left neighbor

                if let Err(_) = self.extract_exact(MutableExtent::new(left_addr, left_size)) {
                    cold_panic!("failed to remove extent");
                }

                ext = MutableExtent::new(left_addr, ext.size().saturating_add(left_size.get()));
            },
            _ => {
                //  any of the extents with lower addresses do not touch `ext`
                //      => do nothing
            }
        }


        //  merge the modified `ext`
        return self.insert_fragmented(&ext);




        /// Removes the extent from the given `SizeMap`, panics if the extraction fails
        ///
        /// Error recovery: this function will return `Err` if the extent cannot be found,
        /// all the user is to do in this case is to re-insert the extent into the `map`
        fn remove_sized_index<const ALIGN: usize>(size_index: &mut SizeMap<ALIGN>, ext: MutableExtent<ALIGN>) -> Result<(), ()> {
            //  manually remove the neighbor from size_index
            //      - neighbor is already removed from the map
            //
            //      - ext_end_address = neighbor's starting address

            let by_max_align = match size_index.get_mut(&ext.size()) {
                Some(by_align) => by_align,
                None => return Err(())
            };

            let addresses = match by_max_align.get_mut(/*&neighbor*/&ext.find_max_alignment()) {
                Some(a) => a,
                None => return Err(())
            };

            if !addresses.remove(&ext.end_address()) {
                return Err(())
            }

            if addresses.is_empty() {
                cold_path();

                if let None = by_max_align.remove(&ext.find_max_alignment()) {
                    cold_path();
                    return Err(())
                }
            }

            Ok(())
        }

    }
}




/// Returned by the `extract_extent_part()` function
/// - Convertable into `PartDeallocError`
#[repr(u32, align(4))]
pub(crate) enum PartExtractionError {
    /// The given extent was not found in allocator
    UnregisteredExtent = 0,
    /// The part extent does not fit into the bigger extent
    PartDoesNotFit = 1,
    /// Something went wrong, but the system was able to recover
    InternalError = 2,
}


impl PartExtractionError {
    pub(crate) fn into_dealloc_error(self) -> PartDeallocError {
        unsafe {
            core::mem::transmute(self)
        }
    }
}


/// Returned by extent allocator's `find_suitable()` function.
/// Contains the extent that is deemed suitable and its part to be allocated
///
///
pub struct SuitableExtent<const ALIGN: usize> {
    /// The extent that the `allocated` fits into
    pub suitable: MutableExtent<ALIGN>,
    /// The part of the `suitable` extent that is meant to be allocated
    /// - Is guaranteed to fit into `suitable`
    pub allocated: MutableExtent<ALIGN>,
}


impl<const ALIGN: usize> SuitableExtent<ALIGN> {

    /// Converts the `suitable` extent into `SuitableExtent` if `suitable` can satisfy `layout`'s requirements
    pub(crate) fn from_suitable(suitable: MutableExtent<ALIGN>, layout: &impl LayoutDescriptor<ALIGN>) -> Option<Self> {

        let allocated = {
            let addr = unsafe {
                //  safety: since `address` cannot be NULL, align_up cannot produce NULL output
                NonZero::new_unchecked(suitable.address().as_nullable().align_up(layout.align().get() as usize))
            };

            assume!(addr.get().is_aligned_to(layout.align().get() as usize));

            let addr = AlignedNonNull::new(addr).unwrap();

            MutableExtent::new(addr, layout.size())
        };

        if allocated.fits_into(&suitable) {
            Some(SuitableExtent { suitable, allocated })
        } else {
            None
        }

    }

}
