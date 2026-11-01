//! Elwynn Survival's rules: a hero of Goldshire fights with aimed, committed blows, a shield and a
//! dodge; gathers wood, food and ore; banks it at the camp; builds stations that improve the next
//! trip; and holds the camp when the kobolds come. Pure logic: no networking, no files, no clock of
//! its own. The server feeds it what players do and plays out the [`Event`]s it returns.

pub mod content;
pub mod content_abilities;
pub mod land;
pub mod save;
pub mod sim;
pub mod tune;

pub use content::{Content, ItemKind};
pub use land::{FlatLand, Land};
pub use sim::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_below;
#[cfg(test)]
mod tests_chase;
#[cfg(test)]
mod tests_abilities;
#[cfg(test)]
mod tests_bow;
#[cfg(test)]
mod tests_arena;
#[cfg(test)]
mod tests_board;
#[cfg(test)]
mod tests_moves;
#[cfg(test)]
mod tests_recast;
#[cfg(test)]
mod tests_classes;
#[cfg(test)]
mod tests_explore;
#[cfg(test)]
mod tests_floors;
#[cfg(test)]
mod tests_meter;
#[cfg(test)]
mod tests_siege;
#[cfg(test)]
mod tests_support;
#[cfg(test)]
mod tests_tactics;
#[cfg(test)]
mod tests_projectiles;
#[cfg(test)]
mod tests_class_warrior_paladin_hunter;
#[cfg(test)]
mod tests_class_rogue_priest_shaman;
#[cfg(test)]
mod tests_class_mage_warlock_druid;
#[cfg(test)]
mod tests_beta_mechanics;
#[cfg(test)]
mod tests_beta;
