use alloc::collections::BTreeSet;

use super::{AddressMap, SizeMap};
use crate::{AlignedNonNull, Alignment, NonNullAddress, cold_panic, assume, extent_alloc::{extent::RemainingExtents, raw_alloc::PartDeallocError}};

use core::{hint::cold_path, mem::ManuallyDrop, num::{NonZero, NonZeroIsize}};

use crate::extent_alloc::{
    extent::{Extent, InternalExtent, MutableExtent, RawExtent}, layout::LayoutDescriptor};


#[cfg(test)]
mod tests;

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


    /// Finds a suitable extent to satisfy the given `layout`
    ///
    /// The returned extent is the one considered suitable (i.e., one that meets the alignment and size requirements)
    /// - The size of the returned extent may not exactly match the requested size,
    /// but the extent is guaranteed to be at least as large as specified by the `layout`
    pub(crate) fn find_suitable(&self, layout: impl LayoutDescriptor<ALIGN>) ->  Option<SuitableExtent<ALIGN>> {

        for (size, by_max_align) in self.size_index.range(layout.size()..) {

            for (_max_align, addresses) in by_max_align.range(layout.align()..) {

                //  max_align contains the maximum alignment that any address within the extent can have

                for address in addresses.iter() {
                    let suitable = MutableExtent::new(*address, *size);

                    let allocated = {
                        let addr = unsafe {
                            //  safety: since `address` cannot be NULL, align_up cannot produce NULL output
                            NonZero::new_unchecked(address.get().align_up(layout.align().get() as usize))
                        };

                        assume!(addr.get().is_aligned_to(layout.align().get() as usize));

                        let addr = AlignedNonNull::new(addr).unwrap();

                        MutableExtent::new(addr, layout.size())
                    };

                    if allocated.fits_into(&suitable) {
                        return Some(SuitableExtent { suitable, allocated })
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
    pub(crate) fn find_closest_to(&self, layout: impl LayoutDescriptor<ALIGN>) -> Option<SuitableExtent<ALIGN>> {

        //  contains `(size, &AlignMap)`
        let mut closest = self.size_index.iter().next()
            .map(|(size, map_ref)| (*size, map_ref) )?;

        for (size, by_max_align) in self.size_index.iter() {
            for (_max_align, addresses) in by_max_align.range(layout.align()..) {

                //  max_align contains the maximum alignment that any address within the extent can have

                for address in addresses.iter() {
                    let suitable = MutableExtent::new(*address, *size);

                    let allocated = {
                        let addr = unsafe {
                            //  safety: since `address` cannot be NULL, align_up cannot produce NULL output
                            NonZero::new_unchecked(address.get().align_up(layout.align().get() as usize))
                        };

                        assume!(addr.get().is_aligned_to(layout.align().get() as usize));
                        let addr = AlignedNonNull::new(addr).unwrap();

                        MutableExtent::new(addr, layout.size())
                    };

                    if allocated.fits_into(&suitable) {
                        return Some(SuitableExtent { suitable, allocated })
                    }
                }
                //  did not found suitable extent => set closest
                closest = (*size, by_max_align);

            }
        }


        //  suitable extent not found => find closest
        todo!("suitable not found, find closest to suitable");

    }


    /// Removes the extent from the map
    ///
    /// Returned `Err` indicates that the extent was not found
    pub(crate) fn extract_exact(&mut self, extent: MutableExtent<ALIGN>) -> Result<(), ()> {

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
    /// Since any state where this function is unable to perform its
    /// task is considered unexpected and/or undefined, it will simply panic when such state is detected
    pub(crate) fn insert(&mut self, mut ext: MutableExtent<ALIGN>) -> Result<(), ()> {

        //todo!("check");

        let ext_end_address = ext.end_address();

        //  used in recovery
        let original = ext.clone();

        //  remove from map
        match self.map.remove(&ext_end_address) {
            Some(neigh_size) => {
                //  has neighbor => defragment

                //let neigh_end_address = ext_end_address.aligned_add(neigh_size.get() as usize);
                let neighbor = MutableExtent::new(ext_end_address, neigh_size);

                {   //  manually remove the neighbor from size_index
                    //      - neighbor is already removed from the map
                    //
                    //      - ext_end_address = neighbor's starting address

                    //  panics here are intended: since the neighboring extent was
                    // found in the map, it must always be in the size index.
                    // - Otherwise the state of the allocator is considered corrupted

                    let by_max_align = match self.size_index.get_mut(&neigh_size) {
                        Some(by_align) => by_align,
                        None => cold_panic!("could not find neighboring extent in size_index:by_size"),
                    };

                    //let addresses = match by_max_align.get_mut(&neigh_end_address.max_alignment()) {
                    let addresses = match by_max_align.get_mut(&neighbor.find_max_alignment()) {
                        Some(a) => a,
                        None => cold_panic!("could not found neighboring extent in size_index:by_align"),
                    };

                    if !addresses.remove(&ext_end_address) {
                        cold_panic!("could not remove the neighboring extent from size_index:addresses");
                    }

                    if addresses.is_empty() {
                        //if let None = by_max_align.remove(&ext_end_address.max_alignment()) {
                        if let None = by_max_align.remove(&ext.find_max_alignment()) {
                            cold_panic!("failed to remove the address map from size_index");
                        }
                    }
                }

                let neighbor = MutableExtent::new(ext_end_address, neigh_size);

                //  defragment => merge ext and neighbor
                let merged = {
                    debug_assert!(ext.is_touching(&neighbor));

                    unsafe {
                        ext.merge_unchecked(neighbor.clone())
                    }
                    ext
                };

                //  defragmentation complete, now insert
                if let Err(_) = self.insert_fragmented(&merged) {
                    //  recovery: re-insert the original and neighbor

                    cold_path();
                    if let Err(_) = self.insert_fragmented(&original) {
                        cold_recovery_panic!("failed to reinsert the original extent")
                    }

                    if let Err(_) = self.insert_fragmented(&MutableExtent::from_regular(neighbor.into_regular::<InternalExtent<ALIGN>>())) {
                        cold_recovery_panic!("failed to reinsert the neighboring extent")
                    }

                    return Err(())
                }

                Ok(())
            },
            None => {   //  no neighbor => no defragmentation
                self.insert_fragmented(&ext)
            }
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


/// Contains the extent that is deemed suitable and its part to be allocated
pub struct SuitableExtent<const ALIGN: usize> {
    /// The extent that the `allocated` fits into
    pub suitable: MutableExtent<ALIGN>,
    /// The part of the `suitable` extent that is meant to be allocated
    /// - Is guaranteed to fit into `suitable`
    pub allocated: MutableExtent<ALIGN>,
}
