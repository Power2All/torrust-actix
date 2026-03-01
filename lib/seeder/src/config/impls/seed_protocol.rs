use crate::config::enums::seed_protocol::SeedProtocol;

impl SeedProtocol {
    pub fn has_bt(&self) -> bool {
        matches!(self, Self::Bt | Self::Both)
    }

    pub fn has_rtc(&self) -> bool {
        matches!(self, Self::Rtc | Self::Both)
    }
}