pub use rj::AsAny;
use std::any::{TypeId};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use foldhash::{HashMap, HashMapExt};

use crate::GenerationalIndex;
use crate::SparseSet;
use crate::IsQueryElement;
use crate::System;

pub trait DynEcsContainer: AsAny  {
    fn len(&self) -> usize; 
    fn contains_entity(&self, entity: Entity) -> bool;
}

pub trait EcsContainer: DynEcsContainer {
    type Item;
    fn entities(&self) -> impl Iterator<Item=Entity>;
    fn get(&self, ett: Entity) -> Option<&Self::Item>;
    fn get_mut(&mut self, ett: Entity) -> Option<&mut Self::Item>;
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
    pub(crate) fn index(&self) -> GenerationalIndex {
        self.0
    }
}

impl From<GenerationalIndex> for Entity {
    fn from(handle: GenerationalIndex) -> Self { Entity(handle) }
}

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
    pub silos: HashMap<TypeId, Box<dyn DynEcsContainer>>
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

    pub fn get_container<T: 'static>(&self) -> Option<&SparseSet<T>> {
        self.silos.get(&TypeId::of::<T>()).and_then(|x| x.as_any().downcast_ref())
    }

    pub fn get_container_mut<T: 'static>(&mut self) -> Option<&mut SparseSet<T>> {
        self.silos.get_mut(&TypeId::of::<T>()).and_then(|x| x.as_any_mut().downcast_mut())
    }

    pub fn get_containers_1<'a, A: IsQueryElement>(&'a mut self) -> (Option<&'a mut A::ContainerType>,) {
        let [a,] = self.silos.get_disjoint_mut([&TypeId::of::<A::Type>(),]);
        (a.and_then(|a| A::cast_container(a)),)
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

    pub fn get_clone<T: 'static + Clone>(&self, entity: Entity) -> Option<T> {
        self.get_container().and_then(|x| x.get(entity.0).cloned())
    }

    fn _get_ref<T: 'static>(&self, index: GenerationalIndex) -> Option<&T> {
        self.silos[&TypeId::of::<T>()]
            .as_any().downcast_ref::<SparseSet<T>>()
            .and_then(|x| x.get(index))
    }
    pub(crate) fn get<T: 'static>(&self, entity: Entity) -> Option<&T> { self._get_ref(entity.0) }

    pub(crate) fn _get_mut<T: 'static>(&mut self, index: GenerationalIndex) -> Option<&mut T> {
        let container = self.get_container_mut::<T>();
        container.and_then(|x| x.get_mut(index))
    }
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> { self._get_mut(entity.0) }

    pub fn add_child(&mut self, parent: Entity, child: Entity) {
        assert!(self.get::<Relation>(child).is_none());
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
            self.get_clone::<Relation>(child).unwrap().prev_sibling = Some(last_child);
            self.get_clone::<Relation>(last_child).unwrap().next_sibling = Some(child);
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
