mod sparse_set;
mod ecs;
mod query;

pub use sparse_set::SparseSet;
use sparse_set::GenerationalIndex;

use ecs::DynEcsContainer;
use ecs::EcsContainer;

pub use ecs::Entity;
pub use ecs::Ecs;
pub use query::IsQueryElement;

use query::System;
