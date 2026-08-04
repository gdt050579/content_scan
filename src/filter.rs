pub struct Filter {

}
impl Filter {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn should_process(&self, path: &str, depth: u32, size: u64) -> bool {
        true
    }
}