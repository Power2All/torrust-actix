use crate::websocket::structs::slave_client_state::SlaveClientState;

impl SlaveClientState {
    /// Creates the initial (disconnected) slave client state.
    pub fn new() -> Self {
        Self {
            encoding: None,
            connected: false,
            pending_requests: std::collections::HashMap::new(),
            request_counter: 0,
        }
    }

    /// Returns the next monotonically increasing request id for correlating responses.
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