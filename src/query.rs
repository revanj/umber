use std::any::TypeId;
use std::marker::PhantomData;
use crate::EcsEntityContainer;
use crate::DynEcsEntityContainer;
use crate::SparseSet;
use crate::Entity;
use crate::GenerationalIndex;
use crate::ecs::Data;
use crate::ecs::DynEcsContainer;
use crate::ecs::EcsGlobalContainer as EcsGlobalContainer;
use crate::ecs::GlobalContainer;
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
    fn fetch<'a>(&self, silos: &'a mut HashMap<TypeId, Box<dyn DynEcsEntityContainer>>) -> Self::Containers<'a>;
}

impl<const M: usize> TypeIdArray for [TypeId; M] {
    type Containers<'a> = [&'a mut Box<dyn DynEcsEntityContainer>; M];
    fn fetch<'a>(&self, silos: &'a mut HashMap<TypeId, Box<dyn DynEcsEntityContainer>>) -> [&'a mut Box<dyn DynEcsEntityContainer>; M] {
        silos.get_disjoint_mut(self.each_ref()).map(|r| r.unwrap())
    }
}

pub trait IsQuery {
    type Ref<'a>;
    type Mut<'a>: Downgrade<Ref = Self::Ref<'a>>;
    type Ids: TypeIdArray + IntoIterator<Item=TypeId>;
    const ARITY: usize;
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
    const ARITY: usize = 1;
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
    const ARITY: usize = 2;
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
    const ARITY: usize = 3;
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

pub struct NonExistent;

pub trait IsQueryElement {
    type Item<'c>;
    type Type: 'static;
    type GlobalType: 'static;
    type ComponentType: 'static;
    type ComponentContainerType: EcsEntityContainer<Item=Self::Type> + 'static;
    type GlobalContainerType: EcsGlobalContainer<Item=Self::Type> + 'static;

    fn cast_global_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::GlobalContainerType>;
    fn cast_component_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::ComponentContainerType>;
    fn typed_global_container(data: &mut Data) -> Option<&mut Self::GlobalContainerType>;
    fn typed_component_container(data: &mut Data) -> Option<&mut Self::ComponentContainerType>;
    fn convert<'c>(item: &'c mut Self::Type) -> Self::Item<'c>;
}

impl<'a, T: 'static> IsQueryElement for &'a T {
    type Item<'c> = &'c T;
    type Type = T; 
    type GlobalType = NonExistent;
    type ComponentType = T;
    type ComponentContainerType = SparseSet<T>;
    type GlobalContainerType = GlobalContainer<T>;

    fn typed_component_container(data: &mut Data) -> Option<&mut Self::ComponentContainerType> {
        data.silos.get_mut(&TypeId::of::<T>()).and_then(|x| x.as_any_mut().downcast_mut())
    }

    fn typed_global_container(data: &mut Data) -> Option<&mut Self::GlobalContainerType> {
        None
    }

    fn convert<'c>(item: &'c mut Self::Type) -> Self::Item<'c> {
        item
    }

    fn cast_component_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::ComponentContainerType> {
        let typed_container =  container.as_any_mut().downcast_mut::<SparseSet<T>>();
        typed_container
    }

    fn cast_global_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::GlobalContainerType> {
        None
    }
}

impl<'a, T> IsQueryElement for &'a mut T where T: 'static {
    type Item<'c> = &'c mut T;
    type Type = T; 
    type GlobalType = NonExistent;
    type ComponentType = T;
    type ComponentContainerType = SparseSet<T>;
    type GlobalContainerType = GlobalContainer<T>;

    fn typed_component_container(data: &mut Data) -> Option<&mut Self::ComponentContainerType> {
        data.silos.get_mut(&TypeId::of::<T>()).and_then(|x| x.as_any_mut().downcast_mut())
    }
    fn typed_global_container(data: &mut Data) -> Option<&mut Self::GlobalContainerType> {
        None
    }
    fn convert<'c>(item: &'c mut T) -> Self::Item<'c>{
        item
    }

    fn cast_component_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::ComponentContainerType> {
        let typed_container = container.as_any_mut().downcast_mut::<SparseSet<T>>();
        typed_container
    }
    fn cast_global_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::GlobalContainerType> {
        None
    }
}

pub struct Global<'a, T> { pub inner: &'a T }
pub struct GlobalMut<'a, T> { pub inner: &'a mut T }

impl<'a, T> IsQueryElement for Global<'a, T> where T: 'static {
    type Item<'c> = &'c T;
    type Type = T;
    type GlobalType = T;
    type ComponentType = NonExistent;
    type GlobalContainerType = GlobalContainer<T>;
    type ComponentContainerType = SparseSet<T>;

    fn typed_global_container(data: &mut Data) -> Option<&mut Self::GlobalContainerType> {
        data.resources.get_mut(&TypeId::of::<T>()).and_then(|x| x.as_any_mut().downcast_mut())
    }
    fn typed_component_container(data: &mut Data) -> Option<&mut Self::ComponentContainerType> {
        None
    }
    fn convert<'c>(item: &'c mut T) -> Self::Item<'c>{
        item
    }

    fn cast_global_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::GlobalContainerType> {
        let typed_container = container.as_any_mut().downcast_mut::<GlobalContainer<T>>();
        typed_container
    }

    fn cast_component_container(_container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::ComponentContainerType> {
        None
    }
}

impl<'a, T> IsQueryElement for GlobalMut<'a, T> where T: 'static {
    type Item<'c> = &'c mut T;
    type Type = T;
    type GlobalType = T;
    type ComponentType = NonExistent;
    type GlobalContainerType = GlobalContainer<T>;
    type ComponentContainerType = SparseSet<T>;

    fn typed_global_container(data: &mut Data) -> Option<&mut Self::GlobalContainerType> {
        data.resources.get_mut(&TypeId::of::<T>()).and_then(|x| x.as_any_mut().downcast_mut())
    }
    fn typed_component_container(data: &mut Data) -> Option<&mut Self::ComponentContainerType> {
        None
    }
    fn convert<'c>(item: &'c mut T) -> Self::Item<'c>{
        item
    }

    fn cast_global_container(container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::GlobalContainerType> {
        let typed_container = container.as_any_mut().downcast_mut::<GlobalContainer<T>>();
        typed_container
    }

    fn cast_component_container(_container: &mut Box<dyn DynEcsContainer>) -> Option<&mut Self::ComponentContainerType> {
        None
    }
}

pub trait SystemDyn {
    fn call(&mut self, data: &mut Data);
    //fn call_entity(&mut self, data: &mut Data, entity: Entity);
}

pub trait System<Params> {
    fn call(&mut self, data: &mut Data);
    //fn call_entity(&mut self, data: &mut Data, entity: Entity);
}

impl<F, A> System<fn(A,)> for F 
    where A: IsQueryElement, F: FnMut(A) + for<'b> FnMut(A::Item<'b>)
{
    fn call(&mut self, data: &mut Data)
    {
        if let Some(container) = A::typed_component_container(data) { 
            for item in container.components_mut() {
                (self)(A::convert(item));
            }
        } else if let Some(container) = A::typed_global_container(data) {
            (self)(A::convert(container.get_mut()));
        }
    }
}

pub struct SystemFn<F, Params> {
    f: F,
    _params: PhantomData<Params>,
}

impl<F, Params> SystemDyn for SystemFn<F, Params>
where F: System<Params>,
{
    fn call(&mut self, data: &mut Data) {
        System::<Params>::call(&mut self.f, data)
    }
}

pub trait IntoSystemDyn<Params>: Sized {
    fn into_dyn(self) -> SystemFn<Self, Params>;
}

impl<F, Params> IntoSystemDyn<Params> for F
where
    F: System<Params>,
{
    fn into_dyn(self) -> SystemFn<Self, Params> {
        SystemFn { f: self, _params: PhantomData }
    }
}

fn shortest_at(containers: &mut [&mut dyn DynEcsContainer]) -> Option<(usize, usize)>{
    let mut min_len_pos: Option<(usize, usize)> = None;
    for i in 0..containers.len() {
        if !containers[i].is_resource() {
            let container_silo_len = containers[i].as_component().unwrap().len();
            if min_len_pos.is_none() {
                min_len_pos = Some((container_silo_len, i));
            }
            if let Some((min_len, _)) = min_len_pos && min_len > container_silo_len {
                min_len_pos = Some((container_silo_len, i));
            }
        }
    }
    
    min_len_pos
}

fn container_to_arg<A: IsQueryElement>(container: &mut dyn DynEcsContainer, ett: Entity) -> Option<A::Item<'_>> {
    let arg_a: A::Item<'_>;
    if container.is_resource() {
        arg_a = A::convert(container.as_any_mut().downcast_mut::<A::GlobalContainerType>().unwrap().get_mut());
    } else {
        let component = container.as_any_mut().downcast_mut::<A::ComponentContainerType>().unwrap().get_mut(ett);
        if component.is_none() { return None; }
        arg_a = A::convert(component.unwrap());
    }

    Some(arg_a)
}

impl<F, A, B> System<fn(A, B)> for F 
    where F: FnMut(A, B) + for <'b> FnMut(A::Item<'b>, B::Item<'b>), A: IsQueryElement, B: IsQueryElement,
{
    fn call(&mut self, data: &mut Data)
    {
        let [a_silo, b_silo] = data.silos.get_disjoint_mut([&TypeId::of::<A::ComponentType>(), &TypeId::of::<B::ComponentType>()]);
        let [a_res, b_res] = data.resources.get_disjoint_mut([&TypeId::of::<A::GlobalType>(), &TypeId::of::<B::GlobalType>()]);
        let a: Option<&mut dyn DynEcsContainer> =
            match (a_silo, a_res) {
                (None, Some(a_res)) => { Some(&mut **a_res) },
                (Some(a_silo), None) => { Some(&mut **a_silo) },
                (None, None) => { None }
                _ => { None }
            };
        let b: Option<&mut dyn DynEcsContainer> =
            match (b_silo, b_res) {
                (None, Some(b_res)) => { Some(&mut **b_res) },
                (Some(b_silo), None) => { Some(&mut **b_silo) },
                (None, None) => { None }
                _ => { None }
            };
        let (Some(container_a), Some(container_b)) = (a, b)
        else { return };

        let mut containers = [container_a, container_b];
        
        let entt = if let Some((len, pos)) = shortest_at(&mut containers) {
            Some(containers[pos].as_component().unwrap().entities_vec())
        } else {
            None
        };

        let [container_a, container_b] = containers;
        if let Some(entt) = entt {
            for ett in entt {
                let Some(arg_a) = container_to_arg::<A>(container_a, ett) else { continue; };
                let Some(arg_b) = container_to_arg::<B>(container_b, ett) else { continue; };
                (self)(arg_a, arg_b)
            }
        } else {
            (self)(A::convert(container_a.as_any_mut().downcast_mut::<A::GlobalContainerType>().unwrap().get_mut()), 
                B::convert(container_b.as_any_mut().downcast_mut::<B::GlobalContainerType>().unwrap().get_mut()));
        }
    }
}


impl<F, A, B, C> System<fn(A, B, C)> for F 
    where F: FnMut(A, B, C) + for <'b> FnMut(A::Item<'b>, B::Item<'b>, C::Item<'b>), 
        A: IsQueryElement, B: IsQueryElement, C: IsQueryElement
{
    fn call(&mut self, data: &mut Data)
    {
        let [a_silo, b_silo, c_silo] = data.silos.get_disjoint_mut([
            &TypeId::of::<A::ComponentType>(), 
            &TypeId::of::<B::ComponentType>(),
            &TypeId::of::<C::ComponentType>()]);

        let [a_res, b_res, c_res] = data.resources.get_disjoint_mut([
            &TypeId::of::<A::GlobalType>(), 
            &TypeId::of::<B::GlobalType>(),
            &TypeId::of::<C::GlobalType>(),
        ]);
        let a: Option<&mut dyn DynEcsContainer> =
            match (a_silo, a_res) {
                (None, Some(a_res)) => { Some(&mut **a_res) },
                (Some(a_silo), None) => { Some(&mut **a_silo) },
                (None, None) => { None }
                _ => { None }
            };
        let b: Option<&mut dyn DynEcsContainer> =
            match (b_silo, b_res) {
                (None, Some(b_res)) => { Some(&mut **b_res) },
                (Some(b_silo), None) => { Some(&mut **b_silo) },
                (None, None) => { None }
                _ => { None }
            };
        let c: Option<&mut dyn DynEcsContainer> =
            match (c_silo, c_res) {
                (None, Some(c_res)) => { Some(&mut **c_res) },
                (Some(c_silo), None) => { Some(&mut **c_silo) },
                (None, None) => { None }
                _ => { None }
            };
        let (Some(container_a), Some(container_b), Some(container_c)) = (a, b, c)
        else { return };

        let mut containers = [container_a, container_b, container_c];
        
        let entt = if let Some((len, pos)) = shortest_at(&mut containers) {
            Some(containers[pos].as_component().unwrap().entities_vec())
        } else {
            None
        };

        let [container_a, container_b, container_c] = containers;
        if let Some(entt) = entt {
            for ett in entt {
                let Some(arg_a) = container_to_arg::<A>(container_a, ett) else { continue; };
                let Some(arg_b) = container_to_arg::<B>(container_b, ett) else { continue; };
                let Some(arg_c) = container_to_arg::<C>(container_c, ett) else { continue; };
                (self)(arg_a, arg_b, arg_c)
            }
        } else {
            (self)(
                A::convert(container_a.as_any_mut().downcast_mut::<A::GlobalContainerType>().unwrap().get_mut()), 
                B::convert(container_b.as_any_mut().downcast_mut::<B::GlobalContainerType>().unwrap().get_mut()),
                C::convert(container_c.as_any_mut().downcast_mut::<C::GlobalContainerType>().unwrap().get_mut()));
        }
    }
}
