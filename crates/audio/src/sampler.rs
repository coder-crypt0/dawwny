//! Native SF2 loading and bounded sample playback. All I/O happens before playback.
use anyhow::{Context, Result, ensure};
use dawwny_core::SampleInstrument;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
    time::SystemTime,
};

pub const SAMPLE_MEMORY_LIMIT: usize = 256 * 1024 * 1024;
pub const SAMPLE_VOICES_PER_TRACK: usize = 64;

#[derive(Debug, Clone)]
pub struct SamplePreset {
    pub name: String,
    pub bank: u16,
    pub program: u8,
}
#[derive(Debug)]
pub struct SampleBank {
    pub file: PathBuf,
    pub name: String,
    pub presets: Vec<SamplePreset>,
    pub storage_bytes: usize,
    release: f32,
    font: Arc<SoundFont>,
}
struct CacheEntry {
    file: PathBuf,
    modified: Option<SystemTime>,
    length: u64,
    bank: Weak<SampleBank>,
}
static CACHE: OnceLock<Mutex<Vec<CacheEntry>>> = OnceLock::new();

pub fn load_sample_bank(path: &Path) -> Result<Arc<SampleBank>> {
    ensure!(
        path.is_absolute()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("sf2")),
        "Choose an absolute local .sf2 file"
    );
    let file = path.canonicalize().with_context(|| {
        format!(
            "Missing sample bank: {}. Import the bank again to relocate it.",
            path.display()
        )
    })?;
    let mut reader = BufReader::new(File::open(&file)?);
    let metadata = reader.get_ref().metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= SAMPLE_MEMORY_LIMIT as u64,
        "SF2 file must be at most 256 MiB"
    );
    let modified = metadata.modified().ok();
    // Control threads only. Weak entries do not keep unused samples resident.
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("Sample cache unavailable"))?;
    cache.retain(|entry| entry.bank.strong_count() > 0);
    for entry in cache.iter() {
        if entry.file == file && entry.modified == modified && entry.length == metadata.len() {
            if let Some(bank) = entry.bank.upgrade() {
                return Ok(bank);
            }
        }
    }
    let estimate = preflight(&mut reader, metadata.len())?;
    let resident: usize = cache
        .iter()
        .filter_map(|e| e.bank.upgrade())
        .map(|b| b.storage_bytes)
        .sum();
    ensure!(
        resident.saturating_add(estimate) <= SAMPLE_MEMORY_LIMIT,
        "Loaded sample banks exceed the 256 MiB budget. Unload unused banks first."
    );
    reader.seek(SeekFrom::Start(0))?;
    let font = Arc::new(SoundFont::new(&mut reader).context("Unsupported or invalid SF2 bank")?);
    let mut presets: Vec<_> = font
        .get_presets()
        .iter()
        .map(|p| SamplePreset {
            name: p.get_name().to_owned(),
            bank: p.get_bank_number() as u16,
            program: p.get_patch_number() as u8,
        })
        .collect();
    presets.sort_by_key(|p| (p.bank, p.program));
    ensure!(!presets.is_empty(), "SF2 contains no playable presets");
    let release = font
        .get_instruments()
        .iter()
        .flat_map(|i| i.get_regions())
        .map(|r| r.get_release_volume_envelope())
        .fold(0.05_f32, f32::max);
    let factor = font
        .get_presets()
        .iter()
        .flat_map(|p| p.get_regions())
        .map(|r| r.get_release_volume_envelope())
        .fold(1.0_f32, f32::max);
    let release = (release * factor + 0.05).clamp(0.05, 30.0);
    let bank = Arc::new(SampleBank {
        file: file.clone(),
        name: font.get_info().get_bank_name().to_owned(),
        presets,
        storage_bytes: estimate,
        release,
        font,
    });
    cache.push(CacheEntry {
        file,
        modified,
        length: metadata.len(),
        bank: Arc::downgrade(&bank),
    });
    Ok(bank)
}

fn u16_at(bytes: &[u8], offset: usize) -> usize {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn header(reader: &mut impl Read) -> Result<([u8; 4], u64)> {
    let mut h = [0; 8];
    reader.read_exact(&mut h)?;
    Ok((h[..4].try_into().unwrap(), u32_at(&h, 4) as u64))
}
// Validate all container lengths and cross-table spans before the third-party reader slices them.
fn preflight(reader: &mut (impl Read + Seek), length: u64) -> Result<usize> {
    let (id, size) = header(reader)?;
    let mut form = [0; 4];
    reader.read_exact(&mut form)?;
    ensure!(
        &id == b"RIFF" && &form == b"sfbk" && size + 8 == length,
        "Not a complete SF2 RIFF bank"
    );
    let mut tables = HashMap::new();
    let mut wave_bytes = 0usize;
    let mut table_bytes = 0usize;
    for expected in [b"INFO", b"sdta", b"pdta"] {
        let (id, size) = header(reader)?;
        let start = reader.stream_position()?;
        ensure!(
            &id == b"LIST" && size >= 4 && start + size <= length,
            "Invalid SF2 list length"
        );
        reader.read_exact(&mut form)?;
        ensure!(
            &form == expected,
            "SF2 needs INFO, sdta and pdta lists in order"
        );
        while reader.stream_position()? < start + size {
            ensure!(
                start + size - reader.stream_position()? >= 8,
                "Truncated SF2 chunk header"
            );
            let (id, count) = header(reader)?;
            let at = reader.stream_position()?;
            ensure!(
                count % 2 == 0 && at + count <= start + size,
                "Invalid SF2 chunk length or padding"
            );
            if expected == b"INFO" {
                ensure!(count <= 65536, "SF2 metadata field exceeds 64 KiB");
                if &id == b"ifil" || &id == b"iver" {
                    ensure!(count == 4, "Invalid SF2 version field");
                }
            } else if expected == b"sdta" && &id == b"smpl" {
                ensure!(
                    wave_bytes == 0 && count >= 4,
                    "Missing or duplicate SF2 sample data"
                );
                wave_bytes = count as usize;
            } else if expected == b"pdta" {
                table_bytes += count as usize;
                ensure!(
                    table_bytes <= 2 * 1024 * 1024,
                    "SF2 mapping tables exceed 2 MiB"
                );
                let mut data = vec![0; count as usize];
                reader.read_exact(&mut data)?;
                ensure!(
                    tables.insert(id, data).is_none(),
                    "Duplicate SF2 mapping table"
                );
            }
            reader.seek(SeekFrom::Start(at + count))?;
        }
    }
    ensure!(
        reader.stream_position()? == length && wave_bytes > 0,
        "Unexpected trailing SF2 data or missing samples"
    );
    for (id, width, min) in [
        (b"phdr", 38, 2),
        (b"pbag", 4, 2),
        (b"pgen", 4, 1),
        (b"inst", 22, 2),
        (b"ibag", 4, 2),
        (b"igen", 4, 1),
        (b"shdr", 46, 2),
    ] {
        let data = tables.get(id).context("Missing SF2 mapping table")?;
        ensure!(
            data.len() % width == 0 && data.len() / width >= min,
            "Invalid SF2 mapping records"
        );
    }
    for (head, width, offset, bag, generators) in [
        (b"phdr", 38, 24, b"pbag", b"pgen"),
        (b"inst", 22, 20, b"ibag", b"igen"),
    ] {
        let headers = &tables[head];
        let bags = &tables[bag];
        let gens = &tables[generators];
        let indexes: Vec<_> = headers
            .chunks_exact(width)
            .map(|h| u16_at(h, offset))
            .collect();
        ensure!(
            indexes[0] == 0
                && indexes.windows(2).all(|w| w[0] < w[1])
                && *indexes.last().unwrap() < bags.len() / 4,
            "Invalid SF2 zone span"
        );
        let indexes: Vec<_> = bags.chunks_exact(4).map(|h| u16_at(h, 0)).collect();
        ensure!(
            indexes[0] == 0
                && indexes.windows(2).all(|w| w[0] <= w[1])
                && *indexes.last().unwrap() <= gens.len() / 4,
            "Invalid SF2 generator span"
        );
    }
    let mut seen = std::collections::HashSet::new();
    for p in tables[b"phdr"]
        .chunks_exact(38)
        .take(tables[b"phdr"].len() / 38 - 1)
    {
        let program = u16_at(p, 20);
        let bank = u16_at(p, 22);
        ensure!(
            program <= 127 && bank <= 128 && seen.insert((bank, program)),
            "Invalid or duplicate SF2 bank/program"
        );
    }
    for s in tables[b"shdr"]
        .chunks_exact(46)
        .take(tables[b"shdr"].len() / 46 - 1)
    {
        ensure!(
            u32_at(s, 20) < u32_at(s, 24) && (400..=384000).contains(&u32_at(s, 36)),
            "Invalid SF2 sample header"
        );
        ensure!(
            u16_at(s, 44) & 0x8000 == 0,
            "ROM sample banks are unsupported"
        );
    }
    // Mapping objects include fixed generator arrays, strings and zone vectors.
    Ok(wave_bytes + table_bytes * 16)
}

pub(crate) struct Sampler {
    synth: Synthesizer,
    bank: Arc<SampleBank>,
    config: SampleInstrument,
}
impl Sampler {
    pub fn new(
        bank: Arc<SampleBank>,
        config: &SampleInstrument,
        rate: u32,
        polyphony: usize,
    ) -> Result<Self> {
        ensure!(
            bank.presets
                .iter()
                .any(|p| p.bank == config.bank && p.program == config.program),
            "No SF2 preset at bank {}, program {} in {}",
            config.bank,
            config.program,
            bank.name
        );
        let mut settings = SynthesizerSettings::new(rate as i32);
        settings.block_size = 16;
        settings.maximum_polyphony = polyphony;
        settings.enable_reverb_and_chorus = false;
        let mut synth = Synthesizer::new(&bank.font, &settings)?;
        synth.set_master_volume(0.7);
        for channel in 0..16 {
            // RustySynth adds 128 on channel 10; compensate so every internal lane uses the chosen preset.
            let bank = config.bank as i32 - if channel == 9 { 128 } else { 0 };
            synth.process_midi_message(channel, 0xB0, 0, bank);
            synth.process_midi_message(channel, 0xC0, config.program as i32, 0);
            synth.process_midi_message(channel, 0xB0, 7, 127);
        }
        Ok(Self {
            synth,
            bank,
            config: config.clone(),
        })
    }
    pub fn on(&mut self, channel: u8, pitch: u8, velocity: f32) {
        self.synth.note_on(
            channel as i32,
            pitch as i32,
            (velocity * 127.0).round().clamp(1.0, 127.0) as i32,
        );
    }
    pub fn off(&mut self, channel: u8, pitch: u8) {
        self.synth.note_off(channel as i32, pitch as i32);
    }
    pub fn release_seconds(&self) -> f32 {
        self.bank.release
    }
    pub fn clear(&mut self) {
        self.synth.reset();
        for channel in 0..16 {
            self.synth.process_midi_message(
                channel,
                0xB0,
                0,
                self.config.bank as i32 - if channel == 9 { 128 } else { 0 },
            );
            self.synth
                .process_midi_message(channel, 0xC0, self.config.program as i32, 0);
            self.synth.process_midi_message(channel, 0xB0, 7, 127);
        }
    }
    pub fn frame(&mut self) -> [f32; 2] {
        let mut l = [0.0];
        let mut r = [0.0];
        self.synth.render(&mut l, &mut r);
        [l[0], r[0]]
    }
}
