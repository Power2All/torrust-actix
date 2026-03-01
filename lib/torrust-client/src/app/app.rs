use crate::app::enums::tab::Tab;
use crate::app::structs::app::TorrustClientApp;
use crate::app::structs::torrent_entry::TorrentEntry;
use crate::app::types::*;
use dark_light::Mode;
use egui::Ui;
use egui_aesthetix::themes::{StandardDark, StandardLight};
use egui_aesthetix::Aesthetix;

pub fn run() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_title("Torrust Client"),
        ..Default::default()
    };
    eframe::run_native(
        "torrust-client",
        native_options,
        Box::new(|cc| {
            let style = match dark_light::detect() {
                Ok(Mode::Dark) => StandardDark.custom_style(),
                _ => StandardLight.custom_style(),
            };
            cc.egui_ctx.set_style(style);
            Ok(Box::new(TorrustClientApp::new()))
        }),
    )
}

pub fn fmt_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;
    if bytes == 0 {
        "0 B".to_string()
    } else if bytes >= TB {
        format!("{:.2} TiB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub fn fmt_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "0 B/s".to_string();
    }
    format!("{}/s", fmt_bytes(bytes_per_sec))
}

pub fn render_status_bar(ui: &mut Ui, torrents: &[TorrentEntry]) {
    let total = torrents.len();
    let dl_speed: u64 = torrents.iter().map(|t| t.download_speed).sum();
    let ul_speed: u64 = torrents.iter().map(|t| t.upload_speed).sum();

    ui.horizontal(|ui| {
        ui.label(format!("{total} torrent(s)"));
        ui.separator();
        ui.label("DHT: N/A");
        ui.separator();
        ui.label(format!(
            "DL: {},  UL: {}",
            fmt_speed(dl_speed),
            fmt_speed(ul_speed)
        ));
        ui.separator();
        ui.label("IP filter: ✗");
    });
}

fn draw_cell(
    painter: &egui::Painter,
    x: &mut f32,
    y_center: f32,
    text: &str,
    width: f32,
    font_id: egui::FontId,
    color: egui::Color32,
) {
    painter.text(
        egui::pos2(*x + 4.0, y_center),
        egui::Align2::LEFT_CENTER,
        text,
        font_id,
        color,
    );
    *x += width + 1.0;
}

pub fn render_table_header(ui: &mut Ui) {
    let cols: &[(&str, f32)] = &[
        ("#", COL_QUEUE),
        ("Name", COL_NAME),
        ("Size", COL_SIZE),
        ("Remaining", COL_SIZE_REM),
        ("Status", COL_STATUS),
        ("Progress", COL_PROGRESS),
        ("ETA", COL_ETA),
        ("DL", COL_DL),
        ("UL", COL_UL),
        ("Avail.", COL_AVAIL),
        ("Ratio", COL_RATIO),
        ("Seeds", COL_SEEDS),
        ("Peers", COL_PEERS),
        ("Added on", COL_ADDED),
        ("Completed", COL_COMPLETED),
        ("Label", COL_LABEL),
    ];

    // egui 0.28: allocate_exact_size returns (Rect, Response)
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), HEADER_HEIGHT),
        egui::Sense::hover(),
    );

    // egui 0.28: child_ui takes (rect, layout, Option<UiStackInfo>)
    let mut child = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None);
    for (label, width) in cols {
        child.add_sized(
            [*width, HEADER_HEIGHT],
            egui::Label::new(egui::RichText::new(*label).strong()),
        );
        child.separator();
    }
}

pub fn render_torrent_row(
    ui: &mut Ui,
    entry: &TorrentEntry,
    index: usize,
    selected: bool,
) -> bool {
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    let rect = response.rect;

    // Row background
    let bg = if selected {
        ui.visuals().selection.bg_fill
    } else if index % 2 == 0 {
        ui.visuals().faint_bg_color
    } else {
        ui.visuals().panel_fill
    };
    painter.rect_filled(rect, 0.0, bg);

    let text_color = if selected {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };

    let font_id = egui::FontId::proportional(12.0);
    let mut x = rect.left();
    let y_center = rect.center().y;

    // Columns before progress bar
    let before_progress: &[(&str, f32)] = &[
        (&entry.queue.to_string(), COL_QUEUE),
        (&entry.name, COL_NAME),
        (&fmt_bytes(entry.size), COL_SIZE),
        (&fmt_bytes(entry.size_remaining), COL_SIZE_REM),
        (entry.status.label(), COL_STATUS),
    ];

    // We need owned strings for fmt_bytes results — collect them first
    let q_str = entry.queue.to_string();
    let sz_str = fmt_bytes(entry.size);
    let rem_str = fmt_bytes(entry.size_remaining);

    for &(text, width) in &[
        (q_str.as_str(), COL_QUEUE),
        (entry.name.as_str(), COL_NAME),
        (sz_str.as_str(), COL_SIZE),
        (rem_str.as_str(), COL_SIZE_REM),
        (entry.status.label(), COL_STATUS),
    ] {
        draw_cell(&painter, &mut x, y_center, text, width, font_id.clone(), text_color);
    }
    let _ = before_progress; // silence unused warning

    // Progress bar
    let progress_rect = egui::Rect::from_min_size(
        egui::pos2(x + 2.0, rect.top() + 3.0),
        egui::vec2(COL_PROGRESS - 4.0, ROW_HEIGHT - 6.0),
    );
    painter.rect_filled(progress_rect, 2.0, ui.visuals().extreme_bg_color);
    let filled_w = (progress_rect.width() * entry.progress).max(0.0);
    let filled_rect = egui::Rect::from_min_size(
        progress_rect.min,
        egui::vec2(filled_w, progress_rect.height()),
    );
    painter.rect_filled(filled_rect, 2.0, entry.status.progress_color());
    let pct_text = format!("{:.1}%", entry.progress * 100.0);
    painter.text(
        progress_rect.center(),
        egui::Align2::CENTER_CENTER,
        &pct_text,
        egui::FontId::proportional(10.0),
        text_color,
    );
    x += COL_PROGRESS + 1.0;

    // Columns after progress bar
    let dl_str = fmt_speed(entry.download_speed);
    let ul_str = fmt_speed(entry.upload_speed);
    let avail_str = format!("{:.3}", entry.availability);
    let ratio_str = format!("{:.3}", entry.ratio);
    let seeds_str = entry.seeds.to_string();
    let peers_str = entry.peers.to_string();

    for &(text, width) in &[
        (entry.eta.as_str(), COL_ETA),
        (dl_str.as_str(), COL_DL),
        (ul_str.as_str(), COL_UL),
        (avail_str.as_str(), COL_AVAIL),
        (ratio_str.as_str(), COL_RATIO),
        (seeds_str.as_str(), COL_SEEDS),
        (peers_str.as_str(), COL_PEERS),
        (entry.added_on.as_str(), COL_ADDED),
        (entry.completed_on.as_str(), COL_COMPLETED),
        (entry.label.as_str(), COL_LABEL),
    ] {
        draw_cell(&painter, &mut x, y_center, text, width, font_id.clone(), text_color);
    }

    response.clicked()
}

pub fn render_torrent_table(ui: &mut Ui, app: &mut TorrustClientApp) {
    egui::ScrollArea::both()
        .id_source("torrent_table")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            render_table_header(ui);
            ui.separator();
            for i in 0..app.torrents.len() {
                let selected = app.selected_torrent == Some(i);
                let clicked = {
                    let entry = &app.torrents[i];
                    render_torrent_row(ui, entry, i, selected)
                };
                if clicked {
                    app.selected_torrent = Some(i);
                }
            }
        });
}

pub fn render_overview_tab(ui: &mut Ui, entry: &TorrentEntry) {
    let left_rows: &[(&str, String)] = &[
        ("Name", entry.name.clone()),
        ("Save path", entry.save_path.clone()),
        (
            "Comment",
            if entry.comment.is_empty() {
                "–".to_string()
            } else {
                entry.comment.clone()
            },
        ),
        (
            "Private",
            if entry.private {
                "Yes".to_string()
            } else {
                "No".to_string()
            },
        ),
        ("Last download", entry.last_download.clone()),
        ("Total download", fmt_bytes(entry.total_downloaded)),
    ];
    let right_rows: &[(&str, String)] = &[
        ("Info hash", entry.info_hash.clone()),
        (
            "Pieces",
            format!(
                "{} × {}",
                entry.piece_count,
                fmt_bytes(entry.piece_length)
            ),
        ),
        ("Size", fmt_bytes(entry.size)),
        ("Ratio", format!("{:.3}", entry.ratio)),
        ("Last upload", entry.last_upload.clone()),
        ("Total upload", fmt_bytes(entry.total_uploaded)),
    ];

    ui.columns(2, |cols| {
        egui::Grid::new("overview_left")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(&mut cols[0], |ui| {
                for (key, val) in left_rows {
                    ui.label(egui::RichText::new(*key).strong());
                    ui.label(val);
                    ui.end_row();
                }
            });

        egui::Grid::new("overview_right")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(&mut cols[1], |ui| {
                for (key, val) in right_rows {
                    ui.label(egui::RichText::new(*key).strong());
                    ui.label(val);
                    ui.end_row();
                }
            });
    });
}

pub fn render_detail_panel(ui: &mut Ui, app: &mut TorrustClientApp) {
    ui.horizontal(|ui| {
        let tabs = [Tab::Overview, Tab::Files, Tab::Peers, Tab::Trackers];
        let labels = ["Overview", "Files", "Peers", "Trackers"];
        for (tab, label) in tabs.iter().zip(labels.iter()) {
            let selected = app.active_tab == *tab;
            if ui.selectable_label(selected, *label).clicked() {
                app.active_tab = tab.clone();
            }
        }
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_source("detail_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| match app.selected_torrent {
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("No torrent selected.");
                });
            }
            Some(idx) => match app.active_tab {
                Tab::Overview => render_overview_tab(ui, &app.torrents[idx]),
                Tab::Files => {
                    ui.label("Files tab — coming soon.");
                }
                Tab::Peers => {
                    ui.label("Peers tab — coming soon.");
                }
                Tab::Trackers => {
                    ui.label("Trackers tab — coming soon.");
                }
            },
        });
}
