use core::{marker::PhantomData, num::NonZero};

use crate::extent_alloc::{extent::{Extent, InternalExtent, MutableExtent}, layout::LayoutDescriptor, raw_alloc::RawExtentAlloc};



/// Makes any cache type work with the `CachedExtentAllocator`
///
/// Small blocks are expected to be allocated very frequently;
/// therefore, to prevent the allocator from having to search
/// for a suitable small block every time (and thus waste valuable
/// processor time), a cache handles the search for blocks for
/// small allocations. On the other hand, allocations of large
/// blocks are less frequent, and caching them is inefficient;
/// therefore, they are handled directly by the allocator
pub trait ExtentCache<const ALIGN: usize, Lay: LayoutDescriptor<ALIGN>>
where Self: Sized {

    /// Used to initialize the cache at compiltime
    const NEW: Self;

    /// Attempts to find a suitable extent for allocation
    ///
    /// # Extent allocation
    /// 1. **Request**: The allocator accepts an allocation request and passes the allocation `layout` to its cache (this function)
    /// 2. **Cache lookup**: If the suitable extent can found, this function removes the part
    /// of the `suitable` extent that is meant to be allocated and saves and returns the result
    /// 3. **Allocator lookup**: If the step 2 (cache lookup) failes, the inner raw allocator proceeds with lookup on its own
    ///
    /// # Behaviour
    /// ## Extent requirements
    /// A suitable extent must meet the following requirements:
    /// - The address alignment must meet the requirements of the given `layout`
    ///   - Must be greater than or equal to `layout.align()`
    /// - The size of the range must be greater than or equal to the size of the given `layout`
    ///
    /// > **These requirements may be asserted by the allocator**
    ///
    /// # Success
    /// > **`Ok(Some(SourceAllocation))`**
    ///
    /// If a suitable extent is found, the cache must locate an allocated extent
    /// that lies within the suitable extent already found. This allocated extent
    /// must meet the layout requirements (its size must be equal to the layout
    /// size, and its alignment must be greater than or equal to the layout alignment).
    ///
    /// After finding the allocated extent, the cache must remove that (allocated)
    /// extent from the suitable extent and store the remaining result back into its record.
    ///
    /// After that, this function must return `Ok(Some(SourceAllocation))`, and
    /// the allocator will update its state so that the `allocated` extent is
    /// detached from the rest of the `suitable` extent and is no longer part of it.
    ///
    /// ## `SourceAllocation`
    /// The `SourceAllocation` structure holds two extents:
    /// - `suitable`: This is the suitable extent, it must be returned in the exact
    /// state as it was located in the cache AND allocated by the extent allocator
    /// - `allocated`: This is the extent that will be returned by the allocator to the user
    ///   - It is required to fit into the suitable extent
    ///   - The size of this extent is required be equal to the layout size
    ///   - Address alignment must be greater than or equal to the layout alignment requirement
    ///
    /// > **These requirements may be asserted by the allocator**
    ///
    /// # Failure
    /// > **`Ok(None)`**
    ///
    /// If a suitable extent cannot be found, this function returns —even though it may seem
    /// counterintuitive— `Ok(None)`. In this case the allocator will handle the request on its own.
    ///
    /// # Out of memory
    ///
    /// > **`Err(RefuelRequest)`**
    ///
    /// If the cache does not have enough memory or if it determines that it needs to allocate
    /// additional memory, it must return `Err(RefuelRequest)`. In that case, the allocator
    /// calls the `refuel()` function and passes it a mutable reference to its internal allocator
    /// so that the cache can allocate the memory itself. It then continues processing the request on its own
    fn allocate_cached(&mut self, layout: Lay) -> Result<Option<SourceAllocation<ALIGN>>, RefuelRequest>;

    /// Allocates memory to be cached. This function is called on reqeust - when the
    /// `allocate_cached()` function returns `Err(RefuelRequest)`
    fn refuel(&mut self, gas_station: &mut RawExtentAlloc<ALIGN>);
}

/// Indicates that the cache has run out of memory and requires refuelling
pub struct RefuelRequest;

/// Returned by the `allocate_cached()` function from the `ExtentCache` trait, pairs the
/// extent that satisfies the allocation and an extent that will be allocated
#[repr(C)]
pub struct SourceAllocation<const ALIGN: usize> {
    /// The extent deemed to be suitable for the allocation
    /// - This is the full extent allocated by the allocation on `refuel()`
    pub suitable: InternalExtent<ALIGN>,
    /// The extent that is being allocated
    /// - This extent is removed from the `suitable` extent and allocated
    ///
    /// Must meet the alignment and size requirements posed by the `layout`
    /// - The size of the extent must be EQUAL to the requested size
    /// - Alignment must be greater than or equal to the layout's alignment requirement
    pub allocated: InternalExtent<ALIGN>,
}


mod default;
pub use default::*;
