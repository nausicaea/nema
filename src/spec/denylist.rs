#[derive(Debug)]
pub struct Denylist(Vec<&'static str>);

impl Denylist {
    pub fn contains(&self, v: &str) -> bool {
        self.0.contains(&v)
    }
}

impl Default for Denylist {
    fn default() -> Self {
        Denylist(vec![
            // The quilted fabric API conflicts with fabric API, so it cannot be used
            "qsl", "qvIfYCYJ",
        ])
    }
}

