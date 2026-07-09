use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;
use super::mft_scan::{self, ScanResult, ScanTree};
use super::Panel;
use crate::charts;
use crate::metrics::SysHandles;
use crate::platform;

pub(crate) struct EntryInfo {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) size: u64,
    pub(crate) is_dir: bool,
}

#[derive(Clone)]
struct DriveInfo {
    label: String,
    path: PathBuf,
    used: u64,
    total: u64,
    /// A local NTFS volume the MFT fast scan can read (given admin rights).
    fast_scan_ok: bool,
}

enum ScanMsg {
    Entry(EntryInfo),
    Done,
    Error(String),
}

#[derive(Default)]
pub struct ExplorerPanel {
    roots: Vec<DriveInfo>,
    current_dir: Option<PathBuf>,
    entries: Vec<EntryInfo>,
    dir_total: u64,
    loading: bool,
    error: Option<String>,
    rx: Option<Receiver<ScanMsg>>,
    // MFT fast-scan state. `tree`, once built, backs instant navigation.
    tree: Option<ScanTree>,
    tree_rx: Option<Receiver<ScanResult>>,
    scanning: bool,
    /// True when the currently shown entries came from the MFT tree (logical
    /// sizes) rather than the live walk (on-disk sizes) — drives the label.
    viewing_tree: bool,
}

impl Panel for ExplorerPanel {
    fn name(&self) -> &str {
        "Storage"
    }

    fn refresh(&mut self, h: &SysHandles) {
        self.roots = h
            .disks
            .iter()
            .map(|d| {
                let path = d.mount_point().to_path_buf();
                let fast_scan_ok = !d.is_removable()
                    && d.file_system().to_string_lossy().eq_ignore_ascii_case("ntfs");
                DriveInfo {
                    label: path.to_string_lossy().into_owned(),
                    path,
                    total: d.total_space(),
                    used: d.total_space().saturating_sub(d.available_space()),
                    fast_scan_ok,
                }
            })
            .collect();

        // Drain a finished MFT scan, if one is in flight.
        if let Some(tree_rx) = &self.tree_rx {
            match tree_rx.try_recv() {
                Ok(Ok(tree)) => {
                    self.tree = Some(tree);
                    self.tree_rx = None;
                    self.scanning = false;
                    self.error = None;
                    // If we're already viewing a folder, refill it from the tree.
                    if self.current_dir.is_some() {
                        let dir = self.current_dir.clone();
                        self.navigate(dir);
                    }
                }
                Ok(Err(err)) => {
                    self.error = Some(err);
                    self.tree_rx = None;
                    self.scanning = false;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.tree_rx = None;
                    self.scanning = false;
                }
            }
        }

        let Some(rx) = &self.rx else { return };
        let mut changed = false;
        loop {
            match rx.try_recv() {
                Ok(ScanMsg::Entry(entry)) => {
                    self.dir_total += entry.size;
                    self.entries.push(entry);
                    changed = true;
                }
                Ok(ScanMsg::Done) => {
                    self.loading = false;
                }
                Ok(ScanMsg::Error(err)) => {
                    self.error = Some(err);
                    self.loading = false;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.loading = false;
                    break;
                }
            }
        }
        if changed {
            self.entries.sort_by(|a, b| b.size.cmp(&a.size));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut pending_nav: Option<Option<PathBuf>> = None; // Some(None) = back to drive list
        let mut pending_scan: Option<PathBuf> = None;
        let mut pending_elevate = false;
        let elevated = platform::is_elevated();

        // Navigation controls only make sense once we've drilled into a drive;
        // at the drive overview there's nowhere "up" to go.
        if let Some(dir) = self.current_dir.clone() {
            ui.horizontal(|ui| {
                if ui.button("💾 Drives").clicked() {
                    pending_nav = Some(None);
                }
                ui.separator();
                if let Some(parent) = dir.parent() {
                    if ui.button("⬆ Up").clicked() {
                        pending_nav = Some(Some(parent.to_path_buf()));
                    }
                }
                ui.weak(dir.display().to_string());
                if self.loading {
                    ui.spinner();
                }
            });
            ui.separator();
        }

        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::from_rgb(0xD9, 0x3A, 0x3A), err);
        }

        match self.current_dir.clone() {
            None => {
                if self.roots.is_empty() {
                    ui.weak("No drives found.");
                }
                for drive in self.roots.clone() {
                    let free = drive.total.saturating_sub(drive.used);
                    let title = egui::RichText::new(format!("💾 {}", drive.label)).heading();
                    if ui
                        .add(egui::Label::new(title).sense(egui::Sense::click()))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Explore what's using this drive")
                        .clicked()
                    {
                        pending_nav = Some(Some(drive.path.clone()));
                    }
                    charts::usage_bar(ui, "", drive.used, drive.total);
                    ui.weak(format!("{} free", charts::format_bytes(free)));

                    if drive.fast_scan_ok {
                        if self.scanning {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.weak("Scanning the Master File Table…");
                            });
                        } else if elevated {
                            let label = if self.tree.is_some() {
                                "⚡ Re-scan (admin)"
                            } else {
                                "⚡ Fast scan (admin)"
                            };
                            if ui
                                .button(label)
                                .on_hover_text(
                                    "Read the NTFS Master File Table directly — sizes the whole \
                                     drive in seconds. Sizes shown are logical (not on-disk).",
                                )
                                .clicked()
                            {
                                pending_scan = Some(drive.path.clone());
                            }
                        } else if ui
                            .button("⚡ Fast scan — needs admin")
                            .on_hover_text(
                                "Relaunches the app with administrator rights so it can read \
                                 the NTFS Master File Table directly (WizTree-style fast scan).",
                            )
                            .clicked()
                        {
                            pending_elevate = true;
                        }
                    }
                    ui.add_space(12.0);
                }
                if !self.roots.is_empty() {
                    ui.separator();
                    ui.weak("Click a drive to drill into its folders, sorted by size.");
                }
            }
            Some(_) => {
                egui::Grid::new("explorer_grid")
                    .striped(true)
                    .num_columns(4)
                    .show(ui, |ui| {
                        ui.strong("Name");
                        ui.strong(if self.viewing_tree {
                            "Size (MFT)"
                        } else {
                            "Size on disk"
                        });
                        ui.strong("");
                        ui.strong("");
                        ui.end_row();

                        for entry in &self.entries {
                            let icon = if entry.is_dir { "📁" } else { "📄" };
                            if entry.is_dir {
                                if ui.link(format!("{icon} {}", entry.name)).clicked() {
                                    pending_nav = Some(Some(entry.path.clone()));
                                }
                            } else {
                                ui.label(format!("{icon} {}", entry.name));
                            }
                            ui.label(charts::format_bytes(entry.size));
                            charts::size_bar(ui, entry.size, self.dir_total.max(1));
                            if ui.small_button("Reveal").clicked() {
                                reveal_in_explorer(&entry.path, entry.is_dir);
                            }
                            ui.end_row();
                        }
                    });

                if self.entries.is_empty() && !self.loading {
                    ui.weak("Empty (or nothing readable in this folder).");
                }
            }
        }

        if let Some(target) = pending_nav {
            self.navigate(target);
        }
        if let Some(drive) = pending_scan {
            self.start_fast_scan(drive);
        }
        if pending_elevate && platform::relaunch_as_admin() {
            // The elevated instance is starting; exit this one so only it runs.
            std::process::exit(0);
        }
    }
}

impl ExplorerPanel {
    fn navigate(&mut self, dir: Option<PathBuf>) {
        self.current_dir = dir.clone();
        self.entries.clear();
        self.dir_total = 0;
        self.error = None;
        self.viewing_tree = false;
        self.rx = None; // drop any in-flight walk receiver

        let Some(dir) = dir else {
            self.loading = false;
            return;
        };

        // If a fast-scan tree covers this path, serve it from memory instantly.
        if let Some(tree) = &self.tree {
            if dir.starts_with(&tree.drive_root) {
                self.entries = tree
                    .children
                    .get(&dir)
                    .map(|entries| {
                        entries
                            .iter()
                            .map(|e| EntryInfo {
                                name: e.name.clone(),
                                path: e.path.clone(),
                                size: e.size,
                                is_dir: e.is_dir,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.dir_total = tree.totals.get(&dir).copied().unwrap_or(0);
                self.viewing_tree = true;
                self.loading = false;
                return;
            }
        }

        // Otherwise fall back to the live, lazy directory walk.
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.loading = true;
        thread::spawn(move || scan_dir(dir, tx));
    }

    fn start_fast_scan(&mut self, drive: PathBuf) {
        let (tx, rx) = mpsc::channel();
        self.tree_rx = Some(rx);
        self.tree = None;
        self.scanning = true;
        self.error = None;
        thread::spawn(move || mft_scan::run_scan(drive, tx));
    }
}

/// Runs on a background thread. Sends each immediate child as its size
/// becomes known so the UI can fill in progressively. If the receiving
/// `ExplorerPanel` has since navigated elsewhere, `rx` is dropped and
/// `tx.send` starts failing — that's our cue to stop early.
fn scan_dir(dir: PathBuf, tx: Sender<ScanMsg>) {
    let read_dir = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) => {
            let _ = tx.send(ScanMsg::Error(format!("Can't read {}: {e}", dir.display())));
            let _ = tx.send(ScanMsg::Done);
            return;
        }
    };

    // Files know their size immediately (metadata is cached from the directory
    // read on Windows, so this is essentially free), so stream them right away.
    // Subdirectories need a full recursive walk to size — collect them for the
    // parallel pass below.
    let mut subdirs: Vec<(String, PathBuf)> = Vec::new();
    for entry in read_dir.flatten() {
        // `file_type()` is served from the directory listing (no extra syscall),
        // unlike `path().is_symlink()` which stats the path afresh.
        let Ok(file_type) = entry.file_type() else { continue };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            subdirs.push((name, path));
        } else {
            let logical = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let size = size_on_disk(&path, logical);
            if tx
                .send(ScanMsg::Entry(EntryInfo { name, path, size, is_dir: false }))
                .is_err()
            {
                return;
            }
        }
    }

    // Each subdirectory's recursive size walk is independent, so fan them out
    // across a worker pool. On SSD/NVMe this overlaps the many small I/O waits
    // and is the main speedup; on a spinning disk it's roughly a wash.
    if !subdirs.is_empty() {
        let workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(1, 16)
            .min(subdirs.len());

        let queue = Arc::new(Mutex::new(subdirs));
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::with_capacity(workers);

        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let cancelled = Arc::clone(&cancelled);
            let tx = tx.clone();
            handles.push(thread::spawn(move || loop {
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }
                let Some((name, path)) = queue.lock().unwrap().pop() else {
                    break;
                };
                let size = dir_size(&path);
                if tx
                    .send(ScanMsg::Entry(EntryInfo { name, path, size, is_dir: true }))
                    .is_err()
                {
                    // Receiver gone (user navigated away) — tell peers to stop
                    // grabbing new work so we don't fight the new scan for I/O.
                    cancelled.store(true, Ordering::Relaxed);
                    break;
                }
            }));
        }

        for h in handles {
            let _ = h.join();
        }
    }

    let _ = tx.send(ScanMsg::Done);
}

/// Sums a directory's subtree size, skipping unreadable entries and
/// symlinks (to avoid cycles) rather than failing the whole scan.
fn dir_size(root: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else { continue };
        for entry in read_dir.flatten() {
            // Cached from the listing — avoids a per-entry `symlink_metadata`
            // syscall, which roughly halves the syscalls in the walk.
            let Ok(file_type) = entry.file_type() else { continue };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if let Ok(metadata) = entry.metadata() {
                total += size_on_disk(&entry.path(), metadata.len());
            }
        }
    }

    total
}

/// Actual physical bytes a file occupies on disk — cluster-rounded, and
/// smaller than the logical size for NTFS-compressed or sparse files. This is
/// the value Windows Explorer labels "Size on disk". Falls back to the logical
/// size if the query fails (e.g. the file is locked or access-denied).
#[cfg(windows)]
fn size_on_disk(path: &Path, logical: u64) -> u64 {
    use std::os::windows::ffi::OsStrExt;

    // GetCompressedFileSizeW reports the real on-disk footprint, honouring
    // compression/sparseness, which the cached directory metadata does not.
    extern "system" {
        fn GetCompressedFileSizeW(file_name: *const u16, file_size_high: *mut u32) -> u32;
        fn GetLastError() -> u32;
    }
    const INVALID_FILE_SIZE: u32 = 0xFFFF_FFFF;
    const NO_ERROR: u32 = 0;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut high: u32 = 0;
    // SAFETY: `wide` is a valid NUL-terminated UTF-16 string living past the
    // call, and `high` is a live, writable u32.
    let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), &mut high) };

    // INVALID_FILE_SIZE is also a legitimate low dword, so it's only an error
    // when GetLastError additionally reports one.
    if low == INVALID_FILE_SIZE && unsafe { GetLastError() } != NO_ERROR {
        return logical;
    }
    (u64::from(high) << 32) | u64::from(low)
}

#[cfg(not(windows))]
fn size_on_disk(_path: &Path, logical: u64) -> u64 {
    logical
}

fn reveal_in_explorer(path: &Path, is_dir: bool) {
    let result = if is_dir {
        Command::new("explorer").arg(path).spawn()
    } else {
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
    };
    // explorer.exe's exit status is unreliable even on success; best-effort,
    // nothing sensible to do with an error here besides not crashing.
    let _ = result;
}
