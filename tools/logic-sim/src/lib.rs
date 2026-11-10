//! logic-sim: simulator and verifier for Scrap Mechanic logic circuits.
//!
//! - `circuit`: the model; loads a blueprint.json, a build slot or a netlist
//! - `sim`: tick-accurate simulator, many copies of the circuit at once
//! - `check`: is a circuit without memory correct, on every input
//! - `families`: what each circuit family has to compute
//! - `build`: netlists, a circuit builder for generators, and the blueprint writer
//! - `memory`: is a circuit with memory correct, by random operations against a model
//! - `parts`: pieces that several generators share (carry networks, full adders, decoders)
//! - `validate`: can it be built in the game
//! - `verify`: check a circuit against its family, with or without memory, and measure its timing
//! - `delay`: worst output delay seen over random input changes
//! - `balance`: pad a circuit with timers until every path is the same number of ticks
//! - `stream`: does a circuit keep up with inputs that change every tick
//! - `script`: small test scripts (set, tick, check)
//! - `api`: every command as a JSON request and reply, for the command line and for `serve`

pub mod api;
pub mod balance;
pub mod build;
pub mod check;
pub mod circuit;
pub mod delay;
pub mod families;
pub mod memory;
pub mod parts;
pub mod script;
pub mod sim;
pub mod stream;
pub mod validate;
pub mod verify;
