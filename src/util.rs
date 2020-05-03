use std::cmp::Ordering;
use std::hint::unreachable_unchecked;

pub const EPSILON: f32 = 1e-6;

// Wrapper around a float that implements Ord.
#[derive(PartialOrd, PartialEq, Debug, Default, Clone, Copy)]
pub struct NotNaN(f32);

impl NotNaN {
    pub fn new(f: f32) -> Option<Self> {
        if f.is_nan() {
            None
        } else {
            Some(NotNaN(f))
        }
    }

    pub fn into_inner(self) -> f32 {
        self.0
    }
}

impl Eq for NotNaN {}

impl Ord for NotNaN {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => unsafe { unreachable_unchecked() },
        }
    }
}
