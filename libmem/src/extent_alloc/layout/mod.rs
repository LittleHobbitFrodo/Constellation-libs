

use core::fmt::Debug;
use core::num::NonZero;

mod default;
pub use default::ExtentLayout;


/// Defines the memory layout when allocating a extent. All extent
/// layout descriptors must satisfy one invariant: **both their size
/// and alignment must be non-zero**
pub trait LayoutDescriptor<const ALIGN: usize> where Self: Sized + Clone {
    /// Maximum possible amount of pages that the descriptor can hold
    const MAX_PAGE_COUNT: u64;

    /// An error type used by the `from_pages()` function
    type Err: Sized;

    /// Size (in pages) of the frame to allocate
    fn size(&self) -> NonZero<u64>;

    /// Constructs the descriptor from a count of pages
    fn from_pages(count: NonZero<u64>) -> Result<Self, Self::Err>;

    /// Constructs the descriptor without checking any invariant
    unsafe fn from_pages_unchecked(count: NonZero<u64>) -> Self;

    /// Returns the alignment requirements of the allocated frame
    ///
    /// The returned value is guaranteed to be greater than or equal
    /// to the `ALIGN` generic constant and power of two
    /// - This may be asserted by the allocator
    fn align(&self) -> NonZero<u64>;

    /*/// Splits a bigger extent into smaller pieces if it cannot be allocated
    ///
    /// # Behaviour
    /// This function is invoked by the `ExtentAllocator` when the current
    /// block is too large to be allocated and is intended to work as
    /// iterator. It thus may be called multiple times on the same frame
    ///
    /// `split()` searches for a valid frame size that is smaller than `self`.
    /// If found, it proceeds to shrink `self` by that size and returns the newly created frame
    /// - > **Note for physical allocators**: It is reccomended that the calculation is architecture-specific and tries to find a value
    /// that can be easily grabbed by the current paging implementaion
    ///
    /// Returns `None` if the smaller frame size cannot be found
    fn split(&mut self) -> Option<Self>;*/
}

/// An error type returned by `LayoutDescriptor::from_pages()`
/// as error by default
pub struct IntegerOverflow;
impl core::fmt::Debug for IntegerOverflow {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "integer overflow")
    }
}
