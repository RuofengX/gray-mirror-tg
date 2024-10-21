use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
    pub link: String,
    pub desc: String,
    pub source: String,
}
impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.link == other.link
    }
}
impl Display for Link {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.desc.fmt(f)
    }
}
impl Link {
    pub fn new(link: String, desc: String, source: &impl Display) -> Self {
        Self {
            link,
            desc,
            source: format!("{}", source),
        }
    }
}
