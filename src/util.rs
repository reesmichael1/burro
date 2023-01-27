#[derive(PartialEq, PartialOrd)]
pub struct OrdFloat {
    pub val: f64,
}

impl Ord for OrdFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other)
            .expect("should not receive NaN OrdFloat")
    }
}

impl Eq for OrdFloat {}
