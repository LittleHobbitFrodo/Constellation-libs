use alloc::collections::{BTreeMap, BTreeSet};
use core::{num::NonZero, ops::{Deref, DerefMut}};
use crate::{Address, AddressMarker, AlignedNonNull, extent_alloc::extent::{Extent, MutableExtent}};



pub(crate) mod alloc_map;
pub(crate) use alloc_map::*;



/// A map used internally by the allocator that uses addresses as keys and extent sizes as values
#[repr(transparent)]
#[derive(Default, Clone, Debug)]
pub(crate) struct AddressMap<const ALIGN: usize>(BTreeMap<AlignedNonNull<u64, ALIGN>, NonZero<u64>>);

impl<const ALIGN: usize> Deref for AddressMap<ALIGN> {
    type Target = BTreeMap<AlignedNonNull<u64, ALIGN>, NonZero<u64>>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<const ALIGN: usize> DerefMut for AddressMap<ALIGN> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<const ALIGN: usize> AddressMap<ALIGN> {
    pub const fn new() -> Self { Self(BTreeMap::new()) }
}




/// A map used internally by the allocator that uses maximum address alignment as keys and a map of addresses as values
/// - You get the index using `address.max_alignment()`
#[repr(transparent)]
#[derive(Default, Clone, Debug)]
pub(crate) struct AlignMap<const ALIGN: usize>(BTreeMap<NonZero<u64>, BTreeSet<AlignedNonNull<u64, ALIGN>>>);


impl<const ALIGN: usize> Deref for AlignMap<ALIGN> {
    type Target = BTreeMap<NonZero<u64>, BTreeSet<AlignedNonNull<u64, ALIGN>>>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<const ALIGN: usize> DerefMut for AlignMap<ALIGN> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<const ALIGN: usize> AlignMap<ALIGN> {
    pub const fn new() -> Self { Self(BTreeMap::new()) }

}



/// A map used internally by the allocator that uses extent sizes as keys and `AlignMap`s as values
#[repr(transparent)]
#[derive(Default, Clone, Debug)]
pub(crate) struct SizeMap<const ALIGN: usize>(BTreeMap<NonZero<u64>, AlignMap<ALIGN>>);


impl<const ALIGN: usize> Deref for SizeMap<ALIGN> {
    type Target = BTreeMap<NonZero<u64>, AlignMap<ALIGN>>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<const ALIGN: usize> DerefMut for SizeMap<ALIGN> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<const ALIGN: usize> SizeMap<ALIGN> {
    pub const fn new() -> Self { Self(BTreeMap::new()) }

    /// Insterst the extent into the map, does NOT defragment
    #[must_use]
    pub fn insert(&mut self, extent: MutableExtent<ALIGN>) -> Result<(), ()> {
        let by_align = self.entry(extent.size()).or_default();
        let addresses = by_align.entry(extent.find_max_alignment()).or_default();
        if addresses.insert(extent.address()) {
           Ok(())
        } else {
            Err(())
        }
    }

}
