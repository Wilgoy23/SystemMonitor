//! WizTree-style fast scan: read a drive's NTFS Master File Table directly off
//! the raw volume (requires administrator rights) and build an in-memory tree
//! of every folder's children and aggregated size. Navigation into that tree
//! is then instant, versus the minutes a full directory walk can take.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use super::explorer::EntryInfo;

/// A fully-scanned drive: for each directory, its (size-sorted) children and
/// the total size of everything beneath it.
pub struct ScanTree {
    pub drive_root: PathBuf,
    pub children: HashMap<PathBuf, Vec<EntryInfo>>,
    pub totals: HashMap<PathBuf, u64>,
}

pub type ScanResult = Result<ScanTree, String>;

/// Entry point run on a background thread; sends the finished tree (or an
/// error message) back over `tx`.
pub fn run_scan(drive_root: PathBuf, tx: Sender<ScanResult>) {
    let _ = tx.send(scan(drive_root));
}

#[cfg(windows)]
fn scan(drive_root: PathBuf) -> ScanResult {
    use ntfs_reader::{DefaultPathCache, FileInfo, Mft, Volume};

    // "C:\" -> drive letter 'C' -> raw device path r"\\.\C:".
    let root_str = drive_root.to_string_lossy();
    let letter = root_str
        .chars()
        .next()
        .ok_or_else(|| "empty drive root".to_string())?;
    let device = format!(r"\\.\{letter}:");
    // ntfs-reader prefixes every FileInfo.path with the device path we opened;
    // swap that back to the ordinary "C:" form so paths match what the rest of
    // the app (and the user) expect.
    let base = format!("{letter}:");

    // `Volume::new` itself fails (ElevationError) if we're not elevated.
    let volume = Volume::new(device.as_str()).map_err(|e| e.to_string())?;
    let mft = Mft::new(volume).map_err(|e| e.to_string())?;

    let mut children: HashMap<PathBuf, Vec<EntryInfo>> = HashMap::new();
    let mut totals: HashMap<PathBuf, u64> = HashMap::new();
    let mut cache = DefaultPathCache::default();
    let cluster_size = mft.volume().cluster_size();

    for file in mft.files() {
        let info = FileInfo::with_cache(&file, &mut cache);
        let Some(path) = info.path.as_ref() else {
            continue; // path reconstruction failed for this record
        };

        let raw = path.to_string_lossy();
        let rel = raw.strip_prefix(device.as_str()).unwrap_or(&raw);
        let real = PathBuf::from(format!("{base}{rel}"));

        // Dir sizes are fixed up after aggregation.
        let size = if info.is_directory {
            0
        } else {
            size_on_disk(&file, &real, info.size, cluster_size)
        };

        if let Some(parent) = real.parent() {
            children.entry(parent.to_path_buf()).or_default().push(EntryInfo {
                name: info.name.clone(),
                path: real.clone(),
                size,
                is_dir: info.is_directory,
            });
        }

        // Every file's bytes count toward each of its ancestor directories.
        if size > 0 {
            for ancestor in real.ancestors().skip(1) {
                *totals.entry(ancestor.to_path_buf()).or_insert(0) += size;
            }
        }
    }

    // Directories carry no $DATA size of their own, so show their subtree total
    // instead, then sort each listing biggest-first.
    for entries in children.values_mut() {
        for entry in entries.iter_mut() {
            if entry.is_dir {
                entry.size = totals.get(&entry.path).copied().unwrap_or(0);
            }
        }
        entries.sort_by(|a, b| b.size.cmp(&a.size));
    }

    Ok(ScanTree {
        drive_root,
        children,
        totals,
    })
}

/// Bytes a file's default stream occupies in clusters — Explorer's "Size on
/// disk". Worked out from the stream's data runs already in memory, so it
/// costs no extra volume reads: sparse holes count for nothing and data small
/// enough to live inside the MFT record (resident) takes no clusters at all.
#[cfg(windows)]
fn size_on_disk(
    file: &ntfs_reader::NtfsFile<'_>,
    path: &std::path::Path,
    logical: u64,
    cluster_size: u64,
) -> u64 {
    use ntfs_reader::{ExtentLocation, NtfsReaderError};

    let stream = match file.open_stream(None) {
        // CompactOS (WOF) files keep a sparse placeholder as their default
        // stream; the real compressed bytes live in this named stream.
        Err(NtfsReaderError::WofCompressedStream) => {
            file.open_stream(Some(std::ffi::OsStr::new("WofCompressedData")))
        }
        other => other,
    };

    match stream {
        Ok(stream) => stream
            .extents()
            .iter()
            .filter(|e| matches!(e.location, ExtentLocation::Volume { .. }))
            // Extents start on cluster boundaries but the last one is cut at
            // the logical size; round each back up to whole clusters.
            .map(|e| e.length.div_ceil(cluster_size) * cluster_size)
            .sum(),
        // NTFS-compressed or encrypted streams can't be laid out from the MFT
        // alone, and they're rare — ask Windows for just these.
        Err(_) => super::explorer::size_on_disk(path, logical),
    }
}

#[cfg(not(windows))]
fn scan(_drive_root: PathBuf) -> ScanResult {
    Err("MFT fast scan is only available on Windows".to_string())
}
