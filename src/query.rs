use std::any::TypeId;
use crate::Ecs;
use crate::EcsContainer;
use crate::DynEcsContainer;
use crate::SparseSet;
use foldhash::HashMap;

pub trait TypeIdArray {
    type Containers<'a>;
    fn fetch<'a>(&self, silos: &'a mut HashMap<TypeId, Box<dyn DynEcsContainer>>) -> Self::Containers<'a>;
}

impl<const M: usize> TypeIdArray for [TypeId; M] {
    type Containers<'a> = [&'a mut Box<dyn DynEcsContainer>; M];
    fn fetch<'a>(&self, silos: &'a mut HashMap<TypeId, Box<dyn DynEcsContainer>>) -> [&'a mut Box<dyn DynEcsContainer>; M] {
        silos.get_disjoint_mut(self.each_ref()).map(|r| r.unwrap())
    }
}

pub trait IsQuery {
    type Ids: TypeIdArray + IntoIterator<Item=TypeId>;
    fn type_ids() -> Self::Ids;
}
impl<A: 'static> IsQuery for (A,) {
    type Ids = [TypeId; 1];
    fn type_ids() -> Self::Ids {
        [TypeId::of::<A>()]
    }
}
impl<A: 'static, B: 'static> IsQuery for (A, B) {
    type Ids = [TypeId; 2];
    fn type_ids() -> Self::Ids {
        [TypeId::of::<A>(), TypeId::of::<B>()]
    }
}
impl<A: 'static, B: 'static, C: 'static> IsQuery for (A, B, C) {
    type Ids = [TypeId; 3];
    fn type_ids() -> Self::Ids {
        [TypeId::of::<A>(), TypeId::of::<B>(), TypeId::of::<C>()]
    }
}

pub trait IsQueryElement {
    type Item<'c>;
    type Type: 'static;
    type ContainerType: EcsContainer<Item=Self::Type>;
    type ContainerRef<'d>: IntoIterator<Item=Self::Item<'d>>;

    fn cast_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::ContainerType>;
    fn typed_container(ecs: &mut Ecs) -> Option<&mut Self::ContainerType>;
    fn convert<'c>(item: &'c mut Self::Type) -> Self::Item<'c>;
}

impl<'a, T: 'static> IsQueryElement for &'a T {
    type Item<'c> = &'c T;
    type Type = T; 
    type ContainerType = SparseSet<T>;
    type ContainerRef<'d> = &'d SparseSet<T>;

    fn typed_container(ecs: &mut Ecs) -> Option<&mut Self::ContainerType> {
        let typed_container = ecs.get_container_mut::<T>();
        typed_container
    }

    fn convert<'c>(item: &'c mut Self::Type) -> Self::Item<'c> {
        item
    }

    fn cast_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut SparseSet<T>> {
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
        let typed_container = ecs.get_container_mut::<T>();
        typed_container
    }
    fn convert<'c>(item: &'c mut T) -> Self::Item<'c>{
        item
    }

    fn cast_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::ContainerType> {
        let typed_container = container.as_any_mut().downcast_mut::<SparseSet<T>>();
        typed_container
    }
}

pub trait System<Params> {
    fn call(self, ecs: &mut Ecs);
}

impl<F, A> System<(A,)> for F 
    where A: IsQueryElement, F: Fn(A) + for<'b> Fn(A::Item<'b>)
{
    fn call(self, ecs: &mut Ecs) {
        let Some(container) = A::typed_container(ecs) else {
            return;
        };
        for item in container.components_mut() {
            (self)(A::convert(item));
        }
    }
}

impl<F, A, B> System<(A, B)> for F 
    where F: Fn(A, B) + for <'b> Fn(A::Item<'b>, B::Item<'b>), A: IsQueryElement, B: IsQueryElement,
{
    fn call(self, ecs: &mut Ecs) {
        let (Some(container_a), Some(container_b)) = ecs.get_containers_2::<A, B>()
        else { return };

        if container_a.len() < container_b.len() {
            for (ett, component) in container_a.entities_components_mut() {
                if let Some(component_b) = container_b.get_mut(ett) {
                    (self)(A::convert(component), B::convert(component_b));
                }
            }
        } else {
            for (ett, component) in container_b.entities_components_mut() {
                if let Some(component_a) = container_a.get_mut(ett) {
                    (self)(A::convert(component_a), B::convert(component));
                }
            }
        }
    }
}


impl<F, A, B, C> System<(A, B, C)> for F 
    where F: Fn(A, B, C) + for <'b> Fn(A::Item<'b>, B::Item<'b>, C::Item<'b>), 
        A: IsQueryElement, B: IsQueryElement, C: IsQueryElement
{
    fn call(self, ecs: &mut Ecs) {
        let (Some(container_a), Some(container_b), Some(container_c)) = ecs.get_containers_3::<A, B, C>()
        else { return };
        
        if container_a.len() <= container_b.len() && container_a.len() <= container_c.len() {
            for (ett, component) in container_a.entities_components_mut() {
                if let Some(component_b) = container_b.get_mut(ett)
                && let Some(component_c) = container_c.get_mut(ett){
                    (self)(A::convert(component), B::convert(component_b), C::convert(component_c));
                }
            }
        }

        else if container_b.len() <= container_a.len() && container_b.len() <= container_c.len() {
            for (ett, component) in container_b.entities_components_mut() {
                if let Some(component_a) = container_a.get_mut(ett)
                && let Some(component_c) = container_c.get_mut(ett){
                    (self)(A::convert(component_a), B::convert(component), C::convert(component_c));
                }
            }
        } else {
            for (ett, component) in container_c.entities_components_mut() {
                if let Some(component_a) = container_a.get_mut(ett)
                && let Some(component_b) = container_b.get_mut(ett){
                    (self)(A::convert(component_a), B::convert(component_b), C::convert(component));
                }
            }
        }
    }
}
