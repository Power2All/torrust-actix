use crate::app::app::{render_detail_panel, render_status_bar, render_torrent_table};
use crate::app::structs::app::TorrustClientApp;
use crate::app::types::*;

impl eframe::App for TorrustClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Status bar (bottom) ---
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(STATUS_BAR_HEIGHT)
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        render_status_bar(ui, &self.torrents);
                    },
                );
            });

        // --- Central panel: split top/bottom with drag handle ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let total_h = ui.available_height();
            let total_w = ui.available_width();
            let origin = ui.cursor().min;

            // Compute panel heights clamped to minimum sizes
            let split_h = (total_h * self.split_ratio)
                .max(MIN_PANEL_HEIGHT)
                .min(total_h - MIN_PANEL_HEIGHT - DRAG_HANDLE_HEIGHT);
            let bottom_h = total_h - split_h - DRAG_HANDLE_HEIGHT;

            let top_rect = egui::Rect::from_min_size(
                origin,
                egui::vec2(total_w, split_h),
            );
            let handle_rect = egui::Rect::from_min_size(
                egui::pos2(origin.x, origin.y + split_h),
                egui::vec2(total_w, DRAG_HANDLE_HEIGHT),
            );
            let bottom_rect = egui::Rect::from_min_size(
                egui::pos2(origin.x, origin.y + split_h + DRAG_HANDLE_HEIGHT),
                egui::vec2(total_w, bottom_h),
            );

            // Top panel — torrent table
            // egui 0.28: child_ui(rect, layout, Option<UiStackInfo>)
            let mut top_ui = ui.child_ui(
                top_rect,
                egui::Layout::top_down(egui::Align::LEFT),
                None,
            );
            render_torrent_table(&mut top_ui, self);

            // Drag handle
            let handle_response = ui.allocate_rect(handle_rect, egui::Sense::drag());
            if handle_response.hovered() || handle_response.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            let handle_color = if handle_response.hovered() || handle_response.dragged() {
                ui.visuals().widgets.hovered.bg_fill
            } else {
                ui.visuals().widgets.noninteractive.bg_fill
            };
            ui.painter().rect_filled(handle_rect, 0.0, handle_color);

            if handle_response.dragged() {
                let delta = handle_response.drag_delta().y;
                self.split_ratio = ((split_h + delta) / total_h)
                    .max(MIN_PANEL_HEIGHT / total_h)
                    .min((total_h - MIN_PANEL_HEIGHT - DRAG_HANDLE_HEIGHT) / total_h);
            }

            // Bottom panel — detail panel
            let mut bottom_ui = ui.child_ui(
                bottom_rect,
                egui::Layout::top_down(egui::Align::LEFT),
                None,
            );
            render_detail_panel(&mut bottom_ui, self);
        });
    }
}
