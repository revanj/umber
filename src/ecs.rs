pub use rj::AsAny;
use std::any::{TypeId};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use foldhash::{HashMap, HashMapExt};

use crate::GenerationalIndex;
use crate::SparseSet;
use crate::IsQueryElement;

pub trait DynSparseSet: AsAny {
    fn contains(&self, entity: Entity) -> bool;
}
pub trait EcsContainer {
    type Item;

    fn len(&self) -> usize; 
    fn entities(&self) -> impl Iterator<Item=Entity>;
    fn get(&self, ett: Entity) -> Option<&Self::Item>;
    fn get_mut(&mut self, ett: Entity) -> Option<&mut Self::Item>;
    fn contains_entity(&self, ett: Entity) -> bool; 
    fn components(&mut self) -> impl Iterator<Item=&Self::Item>; 
    fn components_mut(&mut self) -> impl Iterator<Item=&mut Self::Item>; 
    fn entities_components(&self) -> impl Iterator<Item=(Entity, &Self::Item)>;
    fn entities_components_mut(&mut self) -> impl Iterator<Item=(Entity, &mut Self::Item)>;
}

pub enum TreeOrder {
    PostOrder,
    PreOrder,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub struct Entity(GenerationalIndex);
impl Entity {
    pub fn index(&self) -> GenerationalIndex {
        self.0
    }
}

impl From<GenerationalIndex> for Entity {
    fn from(handle: GenerationalIndex) -> Self { Entity(handle) }
}

pub trait TypeIdArray {
    type Containers<'a>;
    fn fetch<'a>(&self, silos: &'a mut HashMap<TypeId, Box<dyn DynSparseSet>>) -> Self::Containers<'a>;
}

//pub trait AccessArray {
//    type Containers<'a>;
//    fn fetch<'a>(&self, silos: &'a mut HashMap<TypeId, Box<dyn DynSparseSet>>) -> Self::Containers<'a>;
//}


impl<const M: usize> TypeIdArray for [TypeId; M] {
    type Containers<'a> = [&'a mut Box<dyn DynSparseSet>; M];
    fn fetch<'a>(&self, silos: &'a mut HashMap<TypeId, Box<dyn DynSparseSet>>) -> [&'a mut Box<dyn DynSparseSet>; M] {
        silos.get_disjoint_mut(self.each_ref()).map(|r| r.unwrap())
    }
}

pub trait TypeIds {
    type Ref<'a>;
    type Mut<'a>: Downgrade<Ref = Self::Ref<'a>>;
    type Ids: TypeIdArray + IntoIterator<Item=TypeId>;

    fn type_ids() -> Self::Ids;
    fn containers_to_muts<'a, const N: usize>(
        containers: <Self::Ids as TypeIdArray>::Containers<'a>, 
        entities: [Entity; N]) -> [Self::Mut<'a>; N];
}

#[macro_export]
macro_rules! impl_query {
    ($($t:ident),+) => {
        impl<$($t: 'static),+> TypeIds for ($($t,)+) {
            type Ref<'a> = ($(&'a $t,)+);
            type Mut<'a> = ($(&'a mut $t,)+);
            type Ids = [TypeId; <[&str]>::len(&[$(stringify!($t)),+])];

            fn type_ids() -> Self::Ids {
                [$(TypeId::of::<$t>()),+]
            }

            #[allow(non_snake_case)]
            fn containers_to_muts<'a, const N: usize>(
                containers: <Self::Ids as TypeIdArray>::Containers<'a>,
                entities: [Entity; N],
            ) -> [Self::Mut<'a>; N] {
                let rows: [GenerationalIndex; N] = ::std::array::from_fn(|i| entities[i].index());

                let mut it = containers.into_iter();
                $(
                    let set = it
                        .next()
                        .expect("too few containers")
                        .as_any_mut()
                        .downcast_mut::<SparseSet<$t>>()
                        .expect("component type mismatch");
                    let mut $t: [Option<&'a mut $t>; N] =
                        set.get_disjoint_mut(rows).map(Some);
                )+

                std::array::from_fn(|i| ($( $t[i].take().unwrap(), )+))
            }
        }
    };
}

impl_query!(A);
impl_query!(A, B);
impl_query!(A, B, C);

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

pub struct Handle<T>(GenerationalIndex, PhantomData<T>);

#[derive(Clone, Copy)]
pub struct Relation {
    first_child: Option<Entity>,
    last_child: Option<Entity>,
    prev_sibling: Option<Entity>,
    next_sibling: Option<Entity>,
    parent: Option<Entity>,
} impl Relation {
    pub fn new() -> Self { Self {
        first_child: None,
        last_child: None,
        prev_sibling: None,
        next_sibling: None,
        parent: None,
    }}

    pub fn has_children(&self) -> bool {
        self.first_child.is_some()
    }
}

use std::sync::atomic::AtomicU32;
static COUNTER: AtomicU32 = AtomicU32::new(0);

pub trait DynResource: AsAny + 'static {}
impl<T: AsAny + 'static> DynResource for T {}

#[derive(Copy, Clone)]
pub struct Res<T> {
    _phantom: PhantomData<T>
}

pub struct Ecs {
    resources: HashMap<TypeId, Box<dyn DynResource>>,
    entities: Vec<Entity>,
    pub silos: HashMap<TypeId, Box<dyn DynSparseSet>>
} impl Ecs {
    pub fn new() -> Self { Self { 
        resources: HashMap::new(),
        entities: Vec::new(), 
        silos: HashMap::new(),
    }}

    pub fn new_entity(&mut self) -> Entity {
        let ett = Entity(
            GenerationalIndex::new(
                COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0));

       self.entities.push(ett);
       ett
    }

    pub fn add_resource<T: DynResource>(&mut self, value: T) -> Res<T> {
        self.resources.insert(TypeId::of::<T>(), Box::new(value));
        Res { _phantom: PhantomData }
    }
    
    pub fn get_resource<T: DynResource>(&self) -> &T {
        let type_name = std::any::type_name::<T>();

        let dyn_res = self.resources.get(&TypeId::of::<T>())
            .expect("invald resource handle");
        let typed_res = dyn_res.as_any().downcast_ref::<T>()
            .expect(format!("failed to cast dyn resource to type {type_name}").as_str());

        typed_res
    }

    pub fn get_resource_mut<T: DynResource>(&mut self) -> &mut T {
        let type_name = std::any::type_name::<T>();

        let dyn_res = self.resources.get_mut(&TypeId::of::<T>())
            .expect("invald resource handle");
        let typed_res = dyn_res.as_any_mut().downcast_mut::<T>()
            .expect(format!("failed to cast dyn resource to type {type_name}").as_str());

        typed_res
    }

    pub fn get_resource_clone<T: DynResource + Clone>(&self) -> T {
        self.get_resource::<T>().clone()
    }

    pub fn add<T: 'static>(&mut self, entity: Entity, component: T) {
        let type_name = std::any::type_name::<T>();

        if let Some(dyn_map) = self.silos.get_mut(&TypeId::of::<T>()) {
            let typed_map = dyn_map
                .as_any_mut()
                .downcast_mut::<SparseSet<T>>()
                .expect(format!("failed to cast dyn container to container of type {type_name}").as_str());

            typed_map.insert(entity.0, component);
        } else {
            let mut new_map = SparseSet::<T>::new();
            new_map.insert(entity.0, component);
            self.silos.insert(TypeId::of::<T>(), Box::new(new_map));
        }
    }

    pub fn query<Q>(&self) -> Vec<Entity> where Q: TypeIds {
        let mut ret = Vec::new();
    'entity_filter:
        for e in &self.entities {
            for type_id in Q::type_ids() {
                if !self.silos.contains_key(&type_id) { continue 'entity_filter; }
                if !self.silos[&type_id].contains(*e) {
                    continue 'entity_filter;
                }
            }
            ret.push(e.clone());
        }

        ret
    }
    
    pub fn query_tree<Q>(&self, root: Entity, order: TreeOrder) -> Vec<Entity> where Q: TypeIds {
        let mut ret = Vec::new();
        for type_id in Q::type_ids() {
            if !self.silos.contains_key(&type_id) { return ret; }
        }

        let mut filter_entity = |e| {
            for type_id in Q::type_ids() {
                if !self.silos[&type_id].contains(e) { return; }
            }
            ret.push(e);
        };
        match order {
            TreeOrder::PostOrder => self.visit_post_order(root, &mut filter_entity),
            TreeOrder::PreOrder => self.visit_pre_order(root, &mut filter_entity),
        }

        ret
    }

    pub fn get_container_mut<T: 'static>(&mut self) -> Option<&mut SparseSet<T>> {
        let type_name =  std::any::type_name::<T>();
        self.silos.get_mut(&TypeId::of::<T>())
            .expect(format!("no container for type {type_name} in silo").as_str())
            .as_any_mut().downcast_mut::<SparseSet<T>>()
    }

    pub fn get_container<T: 'static>(&self) -> Option<&SparseSet<T>> {
        self.silos[&TypeId::of::<T>()].as_any().downcast_ref::<SparseSet<T>>()
    }

    pub fn get_containers_2<'a, A: IsQueryElement, B: IsQueryElement>(&'a mut self)
        -> (Option<&'a mut A::ContainerType>, Option<&'a mut B::ContainerType>) 
    {
        let [a, b] = self.silos.get_disjoint_mut([&TypeId::of::<A::Type>(), &TypeId::of::<B::Type>()]);
        (a.and_then(|a| A::cast_container(a)), b.and_then(|b| B::cast_container(b)))
    }

    pub fn get_containers_3<'a, 
        A: IsQueryElement, 
        B: IsQueryElement,
        C: IsQueryElement>(&'a mut self) -> (
            Option<&'a mut A::ContainerType>, 
            Option<&'a mut B::ContainerType>, 
            Option<&'a mut C::ContainerType>) 
    {
        let [a, b, c] = self.silos.get_disjoint_mut([
            &TypeId::of::<A::Type>(), 
            &TypeId::of::<B::Type>(), 
            &TypeId::of::<C::Type>()]);

        (a.and_then(|a| A::cast_container(a)), 
            b.and_then(|b| B::cast_container(b)), 
            c.and_then(|c| C::cast_container(c)))
    }

    pub fn get_containers_4<'a, 
        A: IsQueryElement, 
        B: IsQueryElement,
        C: IsQueryElement,
        D: IsQueryElement>(&'a mut self) -> (
            Option<&'a mut A::ContainerType>, 
            Option<&'a mut B::ContainerType>, 
            Option<&'a mut C::ContainerType>, 
            Option<&'a mut D::ContainerType>) 
    {
        let [a, b, c, d] = self.silos.get_disjoint_mut([
            &TypeId::of::<A::Type>(), 
            &TypeId::of::<B::Type>(), 
            &TypeId::of::<C::Type>(),
            &TypeId::of::<D::Type>()]);

        (a.and_then(|a| A::cast_container(a)), 
            b.and_then(|b| B::cast_container(b)), 
            c.and_then(|c| C::cast_container(c)),
            d.and_then(|d| D::cast_container(d)))
    }

    pub fn get_disjoint_mut<T, const N: usize>(&mut self, entities: [Entity; N]) -> [T::Mut<'_>; N]
    where T: TypeIds
    {
        let containers = T::type_ids().fetch(&mut self.silos);
        T::containers_to_muts(containers, entities)
    }

    pub fn get<T: 'static + Clone>(&self, entity: Entity) -> Option<T> {
        self.get_container().and_then(|x| x.get(entity.0).cloned())
    }

    fn _get_ref<T: 'static>(&self, index: GenerationalIndex) -> Option<&T> {
        self.silos[&TypeId::of::<T>()]
            .as_any().downcast_ref::<SparseSet<T>>()
            .and_then(|x| x.get(index))
    }
    pub fn get_ref<T: 'static>(&self, entity: Entity) -> Option<&T> { self._get_ref(entity.0) }

    pub fn _get_mut<T: 'static>(&mut self, index: GenerationalIndex) -> Option<&mut T> {
        let container = self.get_container_mut::<T>();
        container.and_then(|x| x.get_mut(index))
    }
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> { self._get_mut(entity.0) }

    pub fn add_child(&mut self, parent: Entity, child: Entity) {
        assert!(self.get_ref::<Relation>(child).is_none());
        let mut child_relation = Relation::new();
        child_relation.parent = Some(parent);
        self.add(child, child_relation);
        let parent_has_child = self.get::<Relation>(parent).unwrap().has_children();
        if !parent_has_child {
            let parent_ref = self.get_mut::<Relation>(parent).unwrap();
            parent_ref.first_child = Some(child);
            parent_ref.last_child = Some(child);
        } else {
            let last_child = self.get::<Relation>(parent).unwrap().last_child.unwrap();
            self.get::<Relation>(child).unwrap().prev_sibling = Some(last_child);
            self.get::<Relation>(last_child).unwrap().next_sibling = Some(child);
            self.get_mut::<Relation>(parent).unwrap().last_child = Some(child);
        }
    }

    pub fn get_children(&self, node: Entity) -> Vec<Entity> {
        let mut iter = self.get::<Relation>(node).unwrap().first_child;
        let mut ret = Vec::new();
        loop {
            if let Some(child) = iter {
                ret.push(child);
                iter = self.get::<Relation>(child).unwrap().next_sibling;
            } else { break ret; }
        }
    }

    pub fn visit_parent_child<Q>(
        &mut self,
        root: Entity,
        closure: &mut impl for<'a> FnMut(Q::Ref<'a>, Q::Mut<'a>),
    )
    where
        Q: TypeIds,
    {
        let children = self.get_children(root);
        for child_handle in children {
            let [parent, child] = self.get_disjoint_mut::<Q, _>([
                Entity::from(root.index()),
                Entity::from(child_handle.index()),
            ]);
            closure(parent.downgrade(), child);
            self.visit_parent_child::<Q>(child_handle, closure);
        }
    }

    pub fn visit_post_order(
        &self,
        root: Entity,
        closure: &mut impl FnMut(Entity))
    {
        let children = self.get_children(root);
        for c in children.iter().rev() {
            self.visit_post_order(*c, closure);
        }
        closure(root);
    }

    pub fn visit_pre_order(
        &self,
        root: Entity,
        closure: &mut impl FnMut(Entity))
    {
        closure(root);
        let children = self.get_children(root);
        for c in children {
            self.visit_pre_order(c, closure);
        }
    }

    pub fn exec<Params, H: System<Params>>(&mut self, system: H) {
        system.call(self);
    }
}

impl<T: DynResource> Index<Res<T>> for Ecs {
    type Output = T;
    fn index(&self, _index: Res<T>) -> &Self::Output {
        self.get_resource::<T>()
    }
}

impl<T: DynResource> IndexMut<Res<T>> for Ecs {
    fn index_mut(&mut self, _index: Res<T>) -> &mut Self::Output {
        self.get_resource_mut::<T>()
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



#[cfg(test)]
mod test {
    use crate::ecs::{Ecs};

    struct C1(bool);
    struct C2(bool);

    #[test]
    fn test() {
        let mut ecs = Ecs::new();
        let ett = ecs.new_entity();
        ecs.add(ett, C1(true));
        ecs.add(ett, C2(false));

        let closure_system = |c1: &C1, c2: &C2| {
            println!("c1 is {}, c2 is {}", c1.0, c2.0);
        };

        ecs.exec(print_component);
        ecs.exec(closure_system);
        ecs.exec(change_c1);
        ecs.exec(print_component);
    }

    fn print_component(c1: &C1, c2: &C2) {
        println!("c1 is {}, c2 is {}", c1.0, c2.0);
    }
    fn change_c1(c1: &mut C1) {
        c1.0 = false;
    }
}
