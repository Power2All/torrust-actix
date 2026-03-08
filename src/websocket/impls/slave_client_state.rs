use crate::websocket::structs::slave_client_state::SlaveClientState;

impl SlaveClientState {
    pub fn new() -> Self {
        Self {
            encoding: None,
            connected: false,
            pending_requests: std::collections::HashMap::new(),
            request_counter: 0,
        }
    }

    pub fn next_request_id(&mut self) -> u64 {
        self.request_counter = self.request_counter.wrapping_add(1);
        self.request_counter
    }
}

impl Default for SlaveClientState {
    fn default() -> Self {
        Self::new()
    }
}