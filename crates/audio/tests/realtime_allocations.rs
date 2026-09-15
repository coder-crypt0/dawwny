use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {static TRACK:Cell<bool>=const{Cell::new(false)};static COUNT:Cell<usize>=const{Cell::new(0)};}
struct CountedAllocator;
unsafe impl GlobalAlloc for CountedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACK.with(|track| {
            if track.get() {
                COUNT.with(|count| count.set(count.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        TRACK.with(|track| {
            if track.get() {
                COUNT.with(|count| count.set(count.get() + 1));
            }
        });
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountedAllocator = CountedAllocator;
#[test]
fn the_sample_loop_neither_allocates_nor_frees() {
    let mut r = dawwny_audio::Renderer::new(
        dawwny_audio::compile(&dawwny_core::demo_project(), 48000).unwrap(),
    );
    COUNT.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    for _ in 0..48000 {
        std::hint::black_box(r.next_frame());
    }
    r.rewind();
    TRACK.with(|t| t.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
}
