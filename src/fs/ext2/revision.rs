#[derive(Clone, Copy)]
pub enum Revision {
    GoodOld = 0,
    Dynamic = 1,
    Unknown = 2,
}

impl From<u32> for Revision {
    fn from(value: u32) -> Self {
        if value == Revision::GoodOld as u32 {
            Revision::GoodOld
        } else if value == Revision::Dynamic as u32 {
            Revision::Dynamic
        } else {
            Revision::Unknown
        }
    }
}
