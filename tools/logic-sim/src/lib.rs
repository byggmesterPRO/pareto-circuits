//! logic-sim: simulator and verifier for Scrap Mechanic logic circuits.
//!
//! - `circuit`: the model; loads a blueprint.json, a build slot or a netlist
//! - `sim`: tick-accurate simulator, many copies of the circuit at once
//! - `check`: is a circuit without memory correct, on every input
//! - `families`: what each circuit family has to compute
//! - `validate`: can it be built in the game
//! - `delay`: worst output delay seen over random input changes
//! - `script`: small test scripts (set, tick, check)
//! - `api`: every command as a JSON request and reply, for the command line and for `serve`

pub mod api;
pub mod check;
pub mod circuit;
pub mod delay;
pub mod families;
pub mod script;
pub mod sim;
pub mod validate;
