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
    let mut project = dawwny_core::demo_project();
    for (track, preset) in project.tracks.iter_mut().zip(dawwny_core::sound_presets()) {
        track.instrument = dawwny_core::Instrument::Synth;
        track.patch = preset.patch;
    }
    project.tracks[0]
        .patch
        .effects
        .push(dawwny_core::EffectSlot {
            enabled: true,
            effect: dawwny_core::Effect::Equalizer {
                low_db: 2.0,
                mid_db: -1.0,
                high_db: 1.0,
            },
        });
    project.tracks[1]
        .patch
        .effects
        .push(dawwny_core::EffectSlot {
            enabled: true,
            effect: dawwny_core::Effect::Compressor {
                threshold_db: -18.0,
                ratio: 3.0,
                attack_ms: 10.0,
                release_ms: 150.0,
                makeup_db: 0.0,
                mix: 1.0,
            },
        });
    let mut r = dawwny_audio::Renderer::new(dawwny_audio::compile(&project, 48000).unwrap());
    COUNT.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    for _ in 0..48000 {
        std::hint::black_box(r.next_frame());
    }
    r.rewind();
    TRACK.with(|t| t.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
}

mod support;
#[test]
fn sample_notes_seeking_and_live_control_neither_allocate_nor_free() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sf2");
    support::bank(&path);
    let mut r = dawwny_audio::Renderer::new(
        dawwny_audio::compile(&support::project(&path), 16000).unwrap(),
    );
    COUNT.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    for _ in 0..16000 {
        std::hint::black_box(r.next_frame());
    }
    r.seek_beats(0.5);
    r.rewind();
    r.live_event(dawwny_audio::LiveEvent::NoteOn {
        channel: 16,
        pitch: 60,
        velocity: 0.9,
    });
    for _ in 0..2000 {
        std::hint::black_box(r.next_live_frame());
    }
    r.live_event(dawwny_audio::LiveEvent::AllOff);
    TRACK.with(|t| t.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
}
