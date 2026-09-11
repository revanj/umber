mod sparse_set;
mod ecs;
mod query;

pub use sparse_set::SparseSet;
use sparse_set::GenerationalIndex;

use ecs::DynEcsContainer;
use ecs::EcsContainer;

use query::System;
use query::TypeIdArray;
use query::Downgrade;

pub use ecs::Entity;
pub use ecs::Ecs;
pub use ecs::Relation;
pub use ecs::Handle;
pub use ecs::MetaHandle;
pub use ecs::DynHandle;
pub use ecs::TreeOrder;
pub use ecs::Res;

pub use query::IsQueryElement;
pub use query::IsQuery;
pub use query::SystemDyn;
