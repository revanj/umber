use std::any::TypeId;
use crate::Ecs;
use crate::EcsContainer;
use crate::DynEcsContainer;
use crate::SparseSet;
use crate::Entity;
use crate::sparse_set::GenerationalIndex;
use foldhash::HashMap;

pub trait Downgrade {
    type Ref;
    fn downgrade(self) -> Self::Ref;
}

#[macro_export]
macro_rules! impl_downgrade {
    ($($t:ident),+) => {
        #[allow(non_snake_case)]
        impl<'a, $($t: 'a),+> Downgrade for ($(&'a mut $t,)+) {
            type Ref = ($(&'a $t,)+);
            fn downgrade(self) -> Self::Ref {
                let ($($t,)+) = self;
                let out: Self::Ref = ($($t,)+);
                out
            }
        }
    };
}

impl_downgrade!(A);
impl_downgrade!(A, B);
impl_downgrade!(A, B, C);

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
    type Ref<'a>;
    type Mut<'a>: Downgrade<Ref = Self::Ref<'a>>;
    type Ids: TypeIdArray + IntoIterator<Item=TypeId>;
    fn type_ids() -> Self::Ids;
    fn containers_to_muts<'a, const N: usize>(
        containers: <Self::Ids as TypeIdArray>::Containers<'a>,
        entities: [Entity; N],
    ) -> [Self::Mut<'a>; N];

}
impl<A: 'static> IsQuery for (A,) {
    type Ref<'a> = (&'a A,);
    type Mut<'a> = (&'a mut A,);
    type Ids = [TypeId; 1];
    fn type_ids() -> Self::Ids {
        [TypeId::of::<A>()]
    }
    fn containers_to_muts<'a, const N: usize>(
        containers: <Self::Ids as TypeIdArray>::Containers<'a>,
        entities: [Entity; N],
    ) -> [Self::Mut<'a>; N] {
        let rows: [GenerationalIndex; N] = ::std::array::from_fn(|i| entities[i].index());
        let mut it = containers.into_iter();
        let set = it.next()
            .expect("too few containers")
            .as_any_mut()
                .downcast_mut::<SparseSet<A>>()
                .expect("component type mismatch");

        let mut a: [Option<&'a mut A>; N] =
            set.get_disjoint_mut(rows).map(Some);

        std::array::from_fn(|i| (a[i].take().unwrap(),))
    }
}
impl<A: 'static, B: 'static> IsQuery for (A, B) {
    type Ref<'a> = (&'a A,&'a B);
    type Mut<'a> = (&'a mut A,&'a mut B);
    type Ids = [TypeId; 2];
    fn type_ids() -> Self::Ids {
        [TypeId::of::<A>(), TypeId::of::<B>()]
    }
    fn containers_to_muts<'a, const N: usize>(
        containers: <Self::Ids as TypeIdArray>::Containers<'a>,
        entities: [Entity; N],
    ) -> [Self::Mut<'a>; N] {
        let rows: [GenerationalIndex; N] = ::std::array::from_fn(|i| entities[i].index());
        let mut it = containers.into_iter();
        let set = it.next()
            .expect("too few containers")
            .as_any_mut()
                .downcast_mut::<SparseSet<A>>()
                .expect("component type mismatch");

        let mut a: [Option<&'a mut A>; N] =
            set.get_disjoint_mut(rows).map(Some);

        let set = it.next()
            .expect("too few containers")
            .as_any_mut()
                .downcast_mut::<SparseSet<B>>()
                .expect("component type mismatch");
        let mut b: [Option<&'a mut B>; N] =
            set.get_disjoint_mut(rows).map(Some);

        std::array::from_fn(|i| (a[i].take().unwrap(),b[i].take().unwrap()))
    }

}
impl<A: 'static, B: 'static, C: 'static> IsQuery for (A, B, C) {
    type Ref<'a> = (&'a A,&'a B,&'a C);
    type Mut<'a> = (&'a mut A,&'a mut B, &'a mut C);
    type Ids = [TypeId; 3];
    fn type_ids() -> Self::Ids {
        [TypeId::of::<A>(), TypeId::of::<B>(), TypeId::of::<C>()]
    }
    fn containers_to_muts<'a, const N: usize>(
        containers: <Self::Ids as TypeIdArray>::Containers<'a>,
        entities: [Entity; N],
    ) -> [Self::Mut<'a>; N] {
        let rows: [GenerationalIndex; N] = ::std::array::from_fn(|i| entities[i].index());
        let mut it = containers.into_iter();
        let set = it.next()
            .expect("too few containers")
            .as_any_mut()
                .downcast_mut::<SparseSet<A>>()
                .expect("component type mismatch");

        let mut a: [Option<&'a mut A>; N] =
            set.get_disjoint_mut(rows).map(Some);

        let set = it.next()
            .expect("too few containers")
            .as_any_mut()
                .downcast_mut::<SparseSet<B>>()
                .expect("component type mismatch");
        let mut b: [Option<&'a mut B>; N] =
            set.get_disjoint_mut(rows).map(Some);

        let set = it.next()
            .expect("too few containers")
            .as_any_mut()
                .downcast_mut::<SparseSet<C>>()
                .expect("component type mismatch");
        let mut c: [Option<&'a mut C>; N] =
            set.get_disjoint_mut(rows).map(Some);


        std::array::from_fn(|i| (a[i].take().unwrap(),b[i].take().unwrap(), c[i].take().unwrap()))
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
    fn call(&mut self, ecs: &mut Ecs);
}

impl<F, A> System<(A,)> for F 
    where A: IsQueryElement, F: FnMut(A) + for<'b> FnMut(A::Item<'b>)
{
    fn call(&mut self, ecs: &mut Ecs) {
        let Some(container) = A::typed_container(ecs) else {
            return;
        };
        for item in container.components_mut() {
            (self)(A::convert(item));
        }
    }
}

impl<F, A, B> System<(A, B)> for F 
    where F: FnMut(A, B) + for <'b> FnMut(A::Item<'b>, B::Item<'b>), A: IsQueryElement, B: IsQueryElement,
{
    fn call(&mut self, ecs: &mut Ecs) {
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
    where F: FnMut(A, B, C) + for <'b> FnMut(A::Item<'b>, B::Item<'b>, C::Item<'b>), 
        A: IsQueryElement, B: IsQueryElement, C: IsQueryElement
{
    fn call(&mut self, ecs: &mut Ecs) {
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
