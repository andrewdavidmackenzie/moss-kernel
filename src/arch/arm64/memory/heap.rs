use core::{
    alloc::GlobalAlloc,
    arch::asm,
    ops::{Deref, DerefMut},
    ptr,
};

use crate::{
    arch::ArchImpl,
    memory::{
        PAGE_ALLOC, PageOffsetTranslator,
        page::{ClaimedPage, PgAllocGetter},
    },
    sync::OnceLock,
};
use libkernel::{
    CpuOps,
    memory::{
        PAGE_SIZE,
        address::VA,
        allocators::slab::{allocator::SlabAllocator, cache::SlabCache},
        region::PhysMemoryRegion,
    },
};

pub static SLAB_ALLOC: OnceLock<SlabAllocator<ArchImpl, PgAllocGetter, PageOffsetTranslator>> =
    OnceLock::new();

struct PerCpuCache {
    ptr: *mut SlabCache,
    flags: usize,
}

impl PerCpuCache {
    fn get() -> Self {
        let mut cache: *mut SlabCache = ptr::null_mut();

        unsafe { asm!("mrs {}, TPIDR_EL1", out(reg) cache, options(nostack, nomem)) };

        if cache.is_null() {
            panic!("Attempted to use alloc/free before CPU initalisation!");
        }

        let flags = ArchImpl::disable_interrupts();

        Self { ptr: cache, flags }
    }
}

impl Deref for PerCpuCache {
    type Target = SlabCache;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The pointer uses a CPU-banked register for access. We've
        // disabled interrupts so we know we cannot be preempted, therefore
        // mutable access to the cache is safe.
        unsafe { &(*self.ptr) }
    }
}

impl DerefMut for PerCpuCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The pointer uses a CPU-banked register for access. We've
        // disabled interrupts so we know we cannot be preempted, therefore
        // mutable access to the cache is safe.
        unsafe { &mut (*self.ptr) }
    }
}

impl Drop for PerCpuCache {
    fn drop(&mut self) {
        ArchImpl::restore_interrupt_state(self.flags);
    }
}

pub struct KHeap {}

impl KHeap {
    /// Calculates the Frame Allocator order required for a large allocation.
    fn calculate_huge_order(layout: core::alloc::Layout) -> usize {
        // Ensure we cover the size, rounding UP to the nearest page.
        let size = core::cmp::max(layout.size(), layout.align());
        let pages_needed = size.div_ceil(PAGE_SIZE);
        pages_needed.next_power_of_two().ilog2() as usize
    }

    pub fn init_for_this_cpu() {
        let page = ClaimedPage::alloc_zeroed().expect("Cannot allocate heap page");

        // SAFETY: We just successfully allocated the above page and the
        // lifetime of the returned pointer will be for the entire lifetime of
        // the kernel ('sttaic).
        let slab_cache = unsafe { SlabCache::from_page(page) };

        // Store the slab_cache pointer in the CPU-banked register `TPIDR_EL1`.
        #[allow(clippy::pointers_in_nomem_asm_block)]
        unsafe {
            asm!("msr TPIDR_EL1, {}", in(reg) slab_cache, options(nostack, nomem));
        }
    }
}

unsafe impl GlobalAlloc for KHeap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut cache = PerCpuCache::get();

        let Some(cache_line) = cache.get_cache(layout) else {
            // Allocation is too big for SLAB. Defer to using the frame
            // allocator directly.
            return PAGE_ALLOC
                .get()
                .unwrap()
                .alloc_frames(Self::calculate_huge_order(layout) as _)
                .unwrap()
                .leak()
                .start_address()
                .to_va::<PageOffsetTranslator>()
                .cast::<u8>()
                .as_ptr_mut();
        };

        if let Some(ptr) = cache_line.alloc() {
            // Fast path, cache-hit.
            return ptr;
        }

        // Fall back to the slab allocator.
        let mut slab = SLAB_ALLOC
            .get()
            .expect("Slab alocator not initalised")
            .allocator_for_layout(layout)
            .unwrap()
            .lock_save_irq();

        let ptr = slab.alloc();

        // Fill up our cache with objects from the (maybe freshly allocated)
        // slab.
        cache_line.fill_from(&mut slab);

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut cache = PerCpuCache::get();

        let Some(cache_line) = cache.get_cache(layout) else {
            // If the allocation didn't fit in the slab, we must have used the
            // FA directly.
            let allocated_region = PhysMemoryRegion::new(
                VA::from_ptr_mut(ptr as _).to_pa::<PageOffsetTranslator>(),
                PAGE_SIZE << Self::calculate_huge_order(layout),
            );

            unsafe {
                PAGE_ALLOC
                    .get()
                    .unwrap()
                    .alloc_from_region(allocated_region);
            }

            return;
        };

        if cache_line.free(ptr).is_ok() {
            return;
        }

        // The cache is full. Return some memory back to the slab allocator.
        let mut slab = SLAB_ALLOC
            .get()
            .expect("Slab alocator not initalised")
            .allocator_for_layout(layout)
            .unwrap()
            .lock_save_irq();

        slab.free(ptr);

        cache_line.drain_into(&mut slab);
    }
}

#[global_allocator]
static K_HEAP: KHeap = KHeap {};
