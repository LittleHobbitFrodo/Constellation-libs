use alloc::{borrow::ToOwned, collections::{BTreeMap, BTreeSet}};

use crate::{AlignedAddress, AlignedNonNull, Alignment, address, assume, cold_panic, extent_alloc::{
    cache, extent::{Extent, InternalExtent, MutableExtent, RawExtent, RemainingExtents}, helpers::{SizeMap, SuitableExtent}, layout::{ExtentLayout, LayoutDescriptor}, raw_alloc::RawExtentAlloc,
}};
use core::{hint::cold_path, marker::PhantomData, net::AddrParseError, num::{NonZero, NonZeroU64}, ops::Index};

use super::{ExtentCache, SourceAllocation, RefuelRequest};


#[cfg(test)]
mod tests;

/// Extent cache used by the `CachedExtentAllocator` by default
pub struct DefaultCache<const ALIGN: usize, Lay: LayoutDescriptor<ALIGN>> {
    cached: AlignmentCache<ALIGN>,
    _lay: PhantomData<Lay>
}


const ALIGN_ARRAY_LEN: usize = 5;

impl<const ALIGN: usize, Lay: LayoutDescriptor<ALIGN>> DefaultCache<ALIGN, Lay> {

    const PRE_ALLOC_EXT_SIZE: [NonZero<u64>; ALIGN_ARRAY_LEN] = unsafe {
        [
            NonZero::new_unchecked(512),    //  align: 1
            NonZero::new_unchecked(1024),   //  align: 2
            NonZero::new_unchecked(2048),   //  align: 4
            NonZero::new_unchecked(4096),   //  align: 8
            NonZero::new_unchecked(8192),   //  align: 16
        ]
    };

}

impl<const ALIGN: usize, Lay: LayoutDescriptor<ALIGN>> ExtentCache<ALIGN, Lay> for DefaultCache<ALIGN, Lay> {


    const NEW: Self = Self { cached: AlignmentCache::new(), _lay: PhantomData };


    fn allocate_cached(&mut self, layout: Lay) -> Result<Option<SourceAllocation<ALIGN>>, RefuelRequest> {

        let suitable = match self.cached.extract_suitable(&layout) {
            Some(ext) => ext,
            None => return Ok(None)
        };

        let allocated = MutableExtent::new(suitable.address(), layout.size());

        assume!(suitable.address().alignment() >= layout.align());
        assume!(allocated.fits_into(&suitable));


        let RemainingExtents { left, right } = unsafe { allocated.clone().remove_from_unchecked(suitable.clone()) };


        //  remove allocated from suitable and insert
        //  the remaining extents back to the map
        if let Some(left) = left { self.cached.insert(left); }

        if let Some(right) = right { self.cached.insert(right); }


        Ok(Some(SourceAllocation { suitable: suitable.into_regular(), allocated: allocated.into_regular() }))
    }



    fn refuel(&mut self, gas_station: &mut RawExtentAlloc<ALIGN>) {
        let cached = self.cached.inner_mut();

        debug_assert!(cached.len() == Self::PRE_ALLOC_EXT_SIZE.len());

        for (i, map) in cached.iter_mut().enumerate() {
            if map.len() > 8 { continue; }

            let size = unsafe {
                //  safety: the constant's length is equal to the size of the `cached` array
                //  - this is asserted above
                Self::PRE_ALLOC_EXT_SIZE.get_unchecked(i).clone()
            };


            //  construct layout for the allocation
            let align = unsafe { NonZero::new_unchecked(1 << i) };

            let layout = unsafe {
                ExtentLayout::from_pages_with_align_unchecked(size, align)
            };


            //  allocate the extent
            if let Some(ext) = gas_station.allocate_exact(layout) {
                //if let Some(_) = map.insert(ext.size(), ext.address()) {
                if !map.entry(ext.size()).or_default().insert(ext.address()) {
                    //  recovery: deallocate the extent

                    cold_path();
                    if let Err(_) = gas_station.deallocate(MutableExtent::from_regular(ext)) {
                        //  this path should never be taken
                        cold_panic!("cache recovery failed: deallocation failed")
                    }
                }
            } else {
                //  ignored
                cold_path();
            }

        }
    }

}

/// `size` -> `address` pairs
type SizedMap<const ALIGN: usize> = BTreeMap<NonZero<u64>, BTreeSet<AlignedNonNull<NonZero<u64>, ALIGN>>>;

/// Holds an array of maps of extents. The array is indexed by the
/// `trailing_zeroes()` of the layout alignment and each map contains
/// `size` -> `address` pairs.
struct AlignmentCache<const ALIGN: usize>([SizedMap<ALIGN>; ALIGN_ARRAY_LEN]);

impl<const ALIGN: usize> AlignmentCache<ALIGN> {

    const fn new() -> Self {
        Self([SizedMap::new(), SizedMap::new(), SizedMap::new(), SizedMap::new(), SizedMap::new()])
    }

    /// Returns a reference to the map that handles the given alignment
    #[inline(always)]
    pub fn get(&self, align: usize) -> Option<&SizedMap<ALIGN>> {
        self.0.get(align.trailing_zeros() as usize)
    }

    /// Returns a mutable reference to the map that handles the given alignment
    #[inline(always)]
    pub fn get_mut(&mut self, align: usize) -> Option<&mut SizedMap<ALIGN>> {
        self.0.get_mut(align.trailing_zeros() as usize)
    }


    /// Returns a reference to the inner array
    #[inline(always)]
    pub fn inner(&self) -> &[SizedMap<ALIGN>; ALIGN_ARRAY_LEN] { &self.0 }

    /// Returns a mutable reference to the inner array
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut [SizedMap<ALIGN>; ALIGN_ARRAY_LEN] { &mut self.0 }


    /// Tries to find suitable extent for the given `layout`
    ///
    /// If this function returns `Some`, it is guaranteed that the extent is removed from the map/cache.
    ///
    /// The returned extent is the one deemed suitable (and thus the extent found within the map).
    /// The allocated extent is obtained by shrinking or splitting the extent to the given layout size as follows:
    /// ```rust
    /// let allocated = cache.extract_suitable(&layout).unwrap()
    ///     .split(layout.size()).unwrap().unwrap();
    /// ```
    pub fn extract_suitable(&mut self, layout: &'_ impl LayoutDescriptor<ALIGN>) -> Option<MutableExtent<ALIGN>> {

        //  control flow:
        //      the `'align_iter` for loop does not iterate on its own, but only through `continue` statements
        //      on the other hand the `'find_suitable` while loop iterates only on its own
        //      if it finds a suitable extent, it is manually exitted with the `break 'find_suitable` statement
        //      otherwise it falls into the `continue 'align_iter`

        //let align_index = layout.align().trailing_zeros() as usize;

        let map = self.get_mut(layout.align().get() as usize)?;

        let (suitable, set_is_empty) = 'find_suitable: {

            for (size, addresses) in map.range_mut(layout.size()..) {

                let addr = addresses.pop_last()?;

                //  found suitable extent

                assume!(addr.alignment() >= layout.align());
                assume!(*size >= layout.size());

                break 'find_suitable (MutableExtent::new(addr, *size), addresses.is_empty())
            }

            //  No suitable extent was found
            return None
        };

        //  suitable extent found
        if set_is_empty {
            let _ = map.remove(&suitable.size());
        }


        let mut allocated = suitable.clone();
        if let Some(remainder) = unsafe { allocated.split_unchecked(layout.size()) } {
            //  insert the extent back
            if !self.insert(remainder) {
                cold_panic!("failed to insert extent");
            }
        }

        return Some(suitable)


        /*'align_iter: for map in &mut self.0[align_index..] {

            let (suitable, set_empty) = 'find_suitable: {
                while let Some((size, addresses)) = map.range_mut(layout.size()..).next() {

                    //  choose extent
                    let addr = match addresses.last() {
                        Some(a) => *a,
                        None => continue 'align_iter,
                    };

                    //  remove the chosen extent from the map
                    if !addresses.remove(&addr) {
                        cold_panic!("failed to remove the extent from the set")
                    }

                    assume!(addr.alignment() >= layout.align());
                    assume!(*size >= layout.size());


                    break 'find_suitable (MutableExtent::new(addr, *size), addresses.is_empty())
                }

                //  no suitable extent found
                continue 'align_iter

            };

            //  suitable extent found and removed from the map
            assume!(suitable.address().alignment() >= layout.align());
            assume!(suitable.size() >= layout.size());

            if set_empty {     //  remove the set from the map if empty
                cold_path();
                if let None = map.remove(&suitable.size()) {
                    cold_panic!("failed to remove set from the map");
                }
            }

            //  will be splitted
            let mut allocated = suitable.clone();
            if let Some(remainder) = unsafe { allocated.split_unchecked(layout.size()) } {
                //  insert the extent back
                if !self.insert(remainder) {
                    cold_panic!("failed to insert extent");
                }
            }

            return Some(suitable)
        }

        None*/
    }


    /// Inserts the extent into the array based on its alignment
    ///
    /// Returns `true` if the operation was successful
    pub fn insert(&mut self, ext: MutableExtent<ALIGN>) -> bool {

        let index = Self::index_from_address(ext.address()).min(ALIGN_ARRAY_LEN-1);

        assume!(index < self.0.len());

        let map = unsafe { self.0.get_unchecked_mut(index) };

        map.entry(ext.size()).or_default().insert(ext.address())
    }

    /// Calculates the index of the inner array based on the adress
    #[inline]
    pub fn index_from_address(address: AlignedNonNull<NonZero<u64>, ALIGN>) -> usize {
        (address.get() as usize / ALIGN).trailing_zeros() as usize
    }

}
