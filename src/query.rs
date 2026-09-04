use std::any::TypeId;
use crate::Ecs;
use crate::EcsContainer;
use crate::DynSparseSet;
use crate::SparseSet;


pub trait IsQueryElement {
    type Item<'c>;
    type Type: 'static;
    type ContainerType: EcsContainer<Item=Self::Type>;
    type ContainerRef<'d>: IntoIterator<Item=Self::Item<'d>>;

    fn cast_container(container: &mut Box<dyn DynSparseSet>) -> Option<&mut Self::ContainerType>;
    fn typed_container(ecs: &mut Ecs) -> Option<&mut Self::ContainerType>;
    fn convert<'c>(item: &'c mut Self::Type) -> Self::Item<'c>;
}

impl<'a, T: 'static> IsQueryElement for &'a T {
    type Item<'c> = &'c T;
    type Type = T; 
    type ContainerType = SparseSet<T>;
    type ContainerRef<'d> = &'d SparseSet<T>;

    fn typed_container(ecs: &mut Ecs) -> Option<&mut Self::ContainerType> {
        let type_id = TypeId::of::<T>();
        let typed_container = ecs.silos.get_mut(&type_id).and_then(|x| x.as_any_mut().downcast_mut::<SparseSet<T>>());
        typed_container
    }

    fn convert<'c>(item: &'c mut Self::Type) -> Self::Item<'c> {
        item
    }

    fn cast_container(container: &mut Box<dyn DynSparseSet>) -> Option<&mut SparseSet<T>> {
        let typed_container =  container.as_any_mut().downcast_mut::<SparseSet<T>>();
        typed_container
    }
}

impl<'a, T> IsQueryElement for &'a mut T where T: 'static {
    type Item<'c> = &'c mut T;
    type Type = T; 
    type ContainerType = SparseSet<T>;
    type ContainerRef<'d> = &'d mut SparseSet<T>;

    fn typed_container(ecs: &mut Ecs) -> Option<&mut Self::ContainerType> {
        let type_id = TypeId::of::<T>();
        let typed_container = ecs.silos.get_mut(&type_id).and_then(|x| x.as_any_mut().downcast_mut::<SparseSet<T>>());
        typed_container
    }
    fn convert<'c>(item: &'c mut T) -> Self::Item<'c>{
        item
    }

    fn cast_container(container: &mut Box<dyn DynSparseSet>) -> Option<&mut Self::ContainerType> {
        let typed_container = container.as_any_mut().downcast_mut::<SparseSet<T>>();
        typed_container
    }
}


