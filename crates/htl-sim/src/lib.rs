//! Basin Battle's game rules. Pure logic: no networking, no files, no clocks of its own. The
//! server feeds it what players do and plays out the [`Event`]s it returns.

pub mod content;
pub mod describe;
pub mod geom;
pub mod items;
pub mod kit;
pub mod powers;
pub mod sim;
pub mod wares;

pub use content::Content;
pub use geom::{Flat, Ground, Vec3};
pub use sim::{
    Arena, BarSlot, Objective, BaseSite, BaseState, CardView, CastError, SkillBuild, SpellBar, EnemyKind, EntityId, EntityView, Event, HeldCard, Lane, Luck, Menu, Post, MenuKind,
    MenuOption, NpcKind, OfferKind, OfferView, PhaseName, PlayerResult, PlayerView, RerollCost, RunResult, Sim, Splash,
    SpellTarget, Status, StatsView, What, ShopItem, SiteView, SHOP_ICONS, FrontView, Ping, MODIFIERS, Hazard, GroundMark,
    SoldierKind,
};
pub use sim::gear_api::{EquipError, ItemInst, LootView};
pub use sim::RIFLE_SHOT;

#[cfg(test)]
mod tests;
