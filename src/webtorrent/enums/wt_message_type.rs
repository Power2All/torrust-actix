#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WtMessageType {
    Announce,
    Scrape,
    Offer,
    Answer,
    Unknown,
}