use alien_derive::IntEnum;

#[derive(Clone, Copy, IntEnum)]
pub enum Revision {
    GoodOld = 0,
    Dynamic = 1,
    #[default]
    Unknown,
}
