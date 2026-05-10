use enum_dispatch::enum_dispatch;

mod mbc1;
mod no_mbc;

pub use mbc1::Mbc1;
pub use no_mbc::NoMbc;

#[enum_dispatch]
pub trait MbcOps {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

#[enum_dispatch(MbcOps)]
pub enum Mbc {
    NoMbc,
    Mbc1,
}
