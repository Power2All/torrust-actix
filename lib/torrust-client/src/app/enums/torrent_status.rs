/// Status of a torrent entry in the list.
#[derive(Clone, PartialEq)]
pub enum TorrentStatus {
    Paused,
    Completed,
    Downloading,
    Seeding,
}

impl TorrentStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Paused => "Paused",
            Self::Completed => "Completed",
            Self::Downloading => "Downloading",
            Self::Seeding => "Seeding",
        }
    }

    pub fn progress_color(&self) -> egui::Color32 {
        match self {
            Self::Completed | Self::Seeding | Self::Downloading => {
                egui::Color32::from_rgb(0, 168, 90)
            }
            Self::Paused => egui::Color32::from_rgb(160, 160, 160),
        }
    }
}
