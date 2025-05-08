use std::net::SocketAddr;
use crate::cluster::structs::rx_data::RxData;
use crate::cluster::structs::tx_data::TxData;
use crate::cluster::structs::ws_connection::WsConnection;

impl WsConnection {
    pub fn new(address: SocketAddr) -> WsConnection
    {
        let mut ws_connection = WsConnection {
            server_id: String::from(""),
            rx_data: RxData {
                
            },
            tx_data: TxData {
                
            }
        };
        ws_connection
    }
}