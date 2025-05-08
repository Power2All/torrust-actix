use crate::cluster::structs::rx_data::RxData;
use crate::cluster::structs::tx_data::TxData;

pub struct WsConnection {
    server_id: String,
    rx_data: RxData,
    tx_data: TxData
}