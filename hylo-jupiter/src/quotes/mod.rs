mod state;

pub mod token_operation;

use fix::typenum::N9;
use hylo_idl::tokens::{TokenMint, HYLOSOL, JITOSOL};

pub use state::*;

pub trait LST: TokenMint<Exp = N9> {}
impl LST for JITOSOL {}
impl LST for HYLOSOL {}
