

use core::fmt::Debug;
use core::num::NonZero;

mod default;
pub use default::ExtentLayout;

use crate::extent_alloc::extent::Extent;


/// Defines the memory layout when allocating a extent. All extent
/// layout descriptors must satisfy one invariant: **both their size
/// and alignment must be non-zero**
pub trait LayoutDescriptor<const ALIGN: usize> where Self: Sized + Clone {
    /// Maximum possible amount of pages that the descriptor can hold
    const MAX_PAGE_COUNT: u64;


    /// Indicates the smallest possible alignment
    /// that the implementation supports
    const MIN_ALIGN: usize = ALIGN;

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


    /*/// Splits the layout into two smaller layouts. Together, these
    /// two layouts are equivalent to the layout that existed
    /// before this action. This means:
    /// 1. The sum of the layout sizes is the same as the
    /// size of the layout before this action
    /// 2. The alignment of the layouts may change only to
    /// the extent permitted by the specific implementation.
    fn split_half(&mut self) -> Option<Self>;


    /// Splits the layout and determines the best configuration for an extent with the given size
    fn split_at(&mut self, size: NonZero<u64>) -> Option<Self>;*/


    /// Removes the `other` layout from `self`, returns what is left after the removal (if anything)
    ///
    /// This function returning `None` does not mean failure, it simply
    /// means that the `other` extent was at least as big as `self`
    ///
    /// User is free to implement the function to best fit the current usecase. However,
    /// they have to guarantee, that the result's size will be equal to
    /// `self.size()saturating_sub(other.size())`
    fn remove(self, other: Self) -> Option<Self>;

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
