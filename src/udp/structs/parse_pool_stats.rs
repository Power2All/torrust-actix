#[derive(Debug, Clone)]
pub struct ParsePoolStats {
    pub current_size: usize,
    pub current_capacity: usize,
    pub segments: usize,
    pub high_water_mark: usize,
    pub grow_count: usize,
    pub drops: usize,
}