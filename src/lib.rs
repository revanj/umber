mod ecs;
mod query;
mod containers;

pub use containers::sparse_set::SparseSet;
pub use containers::GenerationalIndex;

use ecs::DynEcsEntityContainer;
use ecs::EcsEntityContainer;

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
pub use ecs::DynResource;

pub use query::IsQueryElement;
pub use query::IsQuery;
pub use query::SystemDyn;
