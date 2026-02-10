use crate::webtorrent::enums::wt_message::WtMessage;
use crate::webtorrent::enums::wt_message_type::WtMessageType;

impl WtMessage {
    pub fn message_type(&self) -> WtMessageType {
        match self {
            WtMessage::Announce(_) => WtMessageType::Announce,
            WtMessage::Scrape(_) => WtMessageType::Scrape,
            WtMessage::Offer(_) => WtMessageType::Offer,
            WtMessage::Answer(_) => WtMessageType::Answer,
        }
    }
}