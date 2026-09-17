pub use rj::AsAny;
use rj::TypeIds;
use std::any::{TypeId};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use foldhash::{HashMap, HashMapExt};
use std::fmt::Debug;

use crate::{GenerationalIndex, SystemDyn};
use crate::SparseSet;
use crate::IsQueryElement;
use crate::System;
use crate::IsQuery;
use crate::TypeIdArray;
use crate::Downgrade;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DynHandle {
    id: GenerationalIndex,
    type_id: TypeId,
}

impl<T: 'static> From<Handle<T>> for DynHandle {
    fn from(value: Handle<T>) -> Self { Self { id: value.0, type_id: TypeId::of::<T>() } }
}
impl<T: 'static> From<&Handle<T>> for DynHandle {
    fn from(value: &Handle<T>) -> Self { Self { id: value.0, type_id: TypeId::of::<T>() } }
}

pub struct Handle<T>(GenerationalIndex, PhantomData<T>);
impl<T> Handle<T> {
    pub fn new() -> Self {
        let val = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self (GenerationalIndex::new(val, 0) , PhantomData)
    }
    pub fn index(&self) -> GenerationalIndex {
	self.0
    }
    pub fn entity(&self) -> Entity {
        Entity::from(self.0)
    }
}

impl<T> From<usize> for Handle<T> {
    fn from(value: usize) -> Self {
        Self(GenerationalIndex::new(value as u32, 0), PhantomData)
    }
}

impl<T> From<GenerationalIndex> for Handle<T> {
    fn from(value: GenerationalIndex) -> Self {
        Self(value, PhantomData)
    }
}
impl<T> From<Entity> for Handle<T> {
    fn from(value: Entity) -> Self {
        Self { 0: value.index(), 1: PhantomData }
    }
}

impl<T: 'static> TryFrom<DynHandle> for Handle<T> {
    type Error = DynHandle;

    fn try_from(value: DynHandle) -> Result<Self, Self::Error> {
        if value.type_id == TypeId::of::<T>() {
            Ok(Self(value.id, PhantomData))
        } else {
            Err(value)
        }
    }
}

impl<'a, T: 'static> TryFrom<&'a DynHandle> for Handle<T> {
    type Error = &'a DynHandle;

    fn try_from(value: &'a DynHandle) -> Result<Handle<T>, &'a DynHandle> {
        if value.type_id == TypeId::of::<T>() {
            Ok(Self(value.id, PhantomData))
        } else {
            Err(value)
        }
    }
}

impl<T> Clone for Handle<T> { fn clone(&self) -> Self { *self } }
impl<T> Copy for Handle<T> {}

impl<T> Eq for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle<{:?}>({:?})", std::any::type_name::<T>(), self.0)
    }
}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

pub struct MetaHandle<T, Meta> {
    handle: Handle<T>,
    pub metadata: Meta,
} impl<T, Meta> MetaHandle<T, Meta> {
    pub fn new(metadata: Meta) -> Self {
        Self {
            handle: Handle::new(),
            metadata,
        }
    }

    pub fn handle(&self) -> &Handle<T> { &self.handle }
}

impl<T, Meta: Clone> Clone for MetaHandle<T, Meta> {
    fn clone(&self) -> Self { Self{ handle: self.handle, metadata: self.metadata.clone() } }}
impl<T, Meta: Copy> Copy for MetaHandle<T, Meta> {}

impl<T, Meta: Eq> Eq for MetaHandle<T, Meta> {}
impl<T, Meta: PartialEq> PartialEq for MetaHandle<T, Meta> {
    fn eq(&self, other: &Self) -> bool {
        self.metadata == other.metadata &&
        self.handle.0 == other.handle.0
    }}

impl<T, Meta: Debug> Debug for MetaHandle<T, Meta> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MetaHandle<{:?}, {:?}>({}, {:?})",
               std::any::type_name::<T>(),
               std::any::type_name::<Meta>(),
               self.handle.0,
               self.metadata)
    }
}

impl<T, Meta> Hash for MetaHandle<T, Meta> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.handle.0.hash(state);
    }
}

pub trait DynEcsContainer: AsAny {
    fn is_resource(&self) -> bool;
    fn as_component(&mut self) -> Option<&mut dyn DynEcsEntityContainer>;
}

pub trait EcsContainer: DynEcsContainer {
    type Item;
}

pub trait EcsResourceContainer: EcsContainer {
    fn get(&self) -> &Self::Item;
    fn get_mut(&mut self) -> &mut Self::Item;
}

pub trait DynEcsEntityContainer: DynEcsContainer {
    fn len(&self) -> usize; 
    fn contains_entity(&self, entity: Entity) -> bool;
    fn entities_vec(&self) -> Vec<Entity>;
}

pub trait EcsEntityContainer: DynEcsEntityContainer  + EcsContainer {
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

pub struct GlobalContainer<T> {
    pub inner: T
} impl<T> GlobalContainer<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }
}

impl<T: 'static> AsAny for GlobalContainer<T> {
    fn as_any(self: &Self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(self: &mut Self) -> &mut dyn std::any::Any {
        self
    }
}

impl<T: 'static> DynEcsContainer for GlobalContainer<T> {
    fn is_resource(&self) -> bool {
        true
    }

    fn as_component(&mut self) -> Option<&mut dyn DynEcsEntityContainer> {
        None
    }
}

impl<T: 'static> EcsContainer for GlobalContainer<T> {
    type Item = T;
}

impl<T: 'static> EcsResourceContainer for GlobalContainer<T> {
    fn get(&self) -> &Self::Item {
        &self.inner
    }

    fn get_mut(&mut self) -> &mut Self::Item {
        &mut self.inner
    }
}

#[derive(Copy, Clone)]
pub struct Res<T> {
    _phantom: PhantomData<T>
}


pub(crate) struct Data {
    pub entities: Vec<Entity>,
    pub resources: HashMap<TypeId, Box<dyn DynEcsContainer>>,
    pub silos: HashMap<TypeId, Box<dyn DynEcsEntityContainer>>,
} impl Data {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            entities: Vec::new(), 
            silos: HashMap::new(),
        }
    }
}


pub struct Ecs {
    data: Data,
    entity_triggers: HashMap<Entity, HashMap<TypeId, Vec<Box<dyn SystemDyn>>>>
} impl Ecs {
    pub fn new() -> Self { Self { 
        data: Data::new(),
        entity_triggers: HashMap::new()
    }}

    pub fn new_entity(&mut self) -> Entity {
        let ett = Entity(
            GenerationalIndex::new(
                COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0));

       self.data.entities.push(ett);
       ett
    }

    pub fn add_resource<T: 'static>(&mut self, value: T) -> Res<T> {
        self.data.resources.insert(TypeId::of::<T>(), Box::new(GlobalContainer::new(value)));
        Res { _phantom: PhantomData }
    }
    
    pub fn get_resource<T: 'static>(&self) -> &T {
        let type_name = std::any::type_name::<T>();

        let dyn_res = self.data.resources.get(&TypeId::of::<T>())
            .expect("invald resource handle");
        let typed_res = dyn_res.as_any().downcast_ref::<GlobalContainer<T>>()
            .expect(format!("failed to cast dyn resource to type {type_name}").as_str());

        &typed_res.inner
    }

    pub fn get_resource_clone<T: Clone + 'static>(&self) -> T {
        let type_name = std::any::type_name::<T>();

        let dyn_res = self.data.resources.get(&TypeId::of::<T>())
            .expect("invald resource handle");
        let typed_res = dyn_res.as_any().downcast_ref::<GlobalContainer<T>>()
            .expect(format!("failed to cast dyn resource to type {type_name}").as_str());

        typed_res.inner.clone()
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> &mut T {
        let type_name = std::any::type_name::<T>();

        let dyn_res = self.data.resources.get_mut(&TypeId::of::<T>())
            .expect("invald resource handle");
        let typed_res = dyn_res.as_any_mut().downcast_mut::<GlobalContainer<T>>()
            .expect(format!("failed to cast dyn resource to type {type_name}").as_str());

        &mut typed_res.inner
    }

    pub fn add<T: 'static>(&mut self, entity: Entity, component: T) -> Handle<T> {
        let type_name = std::any::type_name::<T>();

        if let Some(dyn_map) = self.data.silos.get_mut(&TypeId::of::<T>()) {
            let typed_map = dyn_map
                .as_any_mut()
                .downcast_mut::<SparseSet<T>>()
                .expect(format!("failed to cast dyn container to container of type {type_name}").as_str());

            typed_map.insert(entity.0, component);
        } else {
            let mut new_map = SparseSet::<T>::new();
            new_map.insert(entity.0, component);
            self.data.silos.insert(TypeId::of::<T>(), Box::new(new_map));
        }

        Handle::from(entity)
    }

    pub fn remove<T: 'static>(&mut self, entity: Entity) {
        let type_name = std::any::type_name::<T>();

        if let Some(dyn_map) = self.data.silos.get_mut(&TypeId::of::<T>()) {
            let typed_map = dyn_map
                .as_any_mut()
                .downcast_mut::<SparseSet<T>>()
                .expect(format!("failed to cast dyn container to container of type {type_name}").as_str());

            typed_map.remove(entity.0);
        } 
    }

    pub fn remove_typed<T: 'static>(&mut self, handle: Handle<T>) {
        self.remove::<T>(handle.entity());
    }

    pub fn get_container<T: 'static>(&self) -> Option<&SparseSet<T>> {
        self.data.silos.get(&TypeId::of::<T>()).and_then(|x| x.as_any().downcast_ref())
    }

    pub fn get_container_mut<T: 'static>(&mut self) -> Option<&mut SparseSet<T>> {
        self.data.silos.get_mut(&TypeId::of::<T>()).and_then(|x| x.as_any_mut().downcast_mut())
    }

    pub fn get_clone<T: 'static + Clone>(&self, entity: Entity) -> Option<T> {
        self.get_container().and_then(|x| x.get(entity.0).cloned())
    }
    pub fn get_typed_clone<T: 'static + Clone>(&self, handle: Handle<T>) -> Option<T> {
        self.data.silos[&TypeId::of::<T>()]
            .as_any().downcast_ref::<SparseSet<T>>()
            .and_then(|x| x.get(handle.index()).cloned())
    }

    fn _get<T: 'static>(&self, index: GenerationalIndex) -> Option<&T> {
        self.data.silos[&TypeId::of::<T>()]
            .as_any().downcast_ref::<SparseSet<T>>()
            .and_then(|x| x.get(index))
    }
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> { self._get(entity.0) }
    pub fn get_typed<T: 'static>(&self, handle: Handle<T>) -> Option<&T> { self._get(handle.index()) }

    
    pub(crate) fn _get_mut<T: 'static>(&mut self, index: GenerationalIndex) -> Option<&mut T> {
        let container = self.get_container_mut::<T>();
        container.and_then(|x| x.get_mut(index))
    }
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> { self._get_mut(entity.0) }
    pub fn get_typed_mut<T: 'static>(&mut self, handle: Handle<T>) -> Option<&mut T> { self._get_mut(handle.index()) }


    pub fn query<Q: IsQuery>(&self) -> Vec<Entity> {
        let mut ret = Vec::new();
    'entity_filter:
        for e in &self.data.entities {
            for type_id in Q::type_ids() {
                if !self.data.silos.contains_key(&type_id) { continue 'entity_filter; }
                if !self.data.silos[&type_id].contains_entity(*e) {
                    continue 'entity_filter;
                }
            }
            ret.push(e.clone());
        }

        ret
    }
    
    pub fn query_tree<Q: IsQuery>(&self, root: Entity, order: TreeOrder) -> Vec<Entity> {
        let mut ret = Vec::new();
        for type_id in Q::type_ids() {
            if !self.data.silos.contains_key(&type_id) { return ret; }
        }

        let mut filter_entity = |e| {
            for type_id in Q::type_ids() {
                if !self.data.silos[&type_id].contains_entity(e) { return; }
            }
            ret.push(e);
        };
        match order {
            TreeOrder::PostOrder => self.visit_post_order(root, &mut filter_entity),
            TreeOrder::PreOrder => self.visit_pre_order(root, &mut filter_entity),
        }

        ret
    }

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
            self.get_mut::<Relation>(child).unwrap().prev_sibling = Some(last_child);
            self.get_mut::<Relation>(last_child).unwrap().next_sibling = Some(child);
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

    pub fn get_disjoint_mut<T, const N: usize>(&mut self, entities: [Entity; N]) -> [T::Mut<'_>; N]
    where T: IsQuery 
    {
        let containers = T::type_ids().fetch(&mut self.data.silos);
        T::containers_to_muts(containers, entities)
    }

    pub fn visit_parent_child<Q: IsQuery>(
        &mut self,
        root: Entity,
        closure: &mut impl for<'a> FnMut(Q::Ref<'a>, Q::Mut<'a>),
    )
    {
        let children = self.get_children(root);
        for child_handle in children {
            let [parent, child] = self.get_disjoint_mut::<Q, _>([
                root,
                child_handle,
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

    pub fn exec<Params, H: System<Params>>(&mut self, mut system: H) {
        system.call(&mut self.data);
    }

    pub fn add_entity_trigger<Trigger: 'static, Params, S: System<Params> + SystemDyn +'static>(
        &mut self, entity: Entity, system: S) 
    {
        if !self.entity_triggers.contains_key(&entity) {
            self.entity_triggers.insert(entity, HashMap::new());
        }
        if !self.entity_triggers[&entity].contains_key(&TypeId::of::<Trigger>()) {
            self.entity_triggers.get_mut(&entity).unwrap().insert(TypeId::of::<Trigger>(), Vec::new());
        }
        self.entity_triggers.get_mut(&entity).unwrap().get_mut(&TypeId::of::<Trigger>()).unwrap().push(Box::new(system));
    }

    pub fn entity_trigger<Trigger: 'static>(&mut self, entity: Entity, trigger: Trigger) {
        if let Some(hash_map) = self.entity_triggers.get_mut(&entity) 
        && let Some(systems) = hash_map.get_mut(&TypeId::of::<Trigger>()) { 
            for s in systems {
                s.call(&mut self.data);
            }
        }
    }
}

impl<T: 'static> Index<Handle<T>> for Ecs {
    type Output = T;
    fn index(&self, index: Handle<T>) -> &Self::Output {
        self.get_typed(index).unwrap()
    }
}

impl<T: 'static> IndexMut<Handle<T>> for Ecs {
    fn index_mut(&mut self, index: Handle<T>) -> &mut Self::Output {
        self.get_typed_mut(index).unwrap()
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
    use crate::{ecs::Ecs, query::GlobalMut};

    struct C1(bool);
    struct C2(bool);
    struct CR(bool);

    #[test]
    fn test() {
        let mut ecs = Ecs::new();
        let ett = ecs.new_entity();
        ecs.add(ett, C1(true));
        ecs.add(ett, C2(false));
        ecs.add_resource(CR(true));


        let closure_system = |c1: &C1, c2: &C2| {
            println!("c1 is {}, c2 is {}", c1.0, c2.0);
        };

        ecs.exec(print_component);
        ecs.exec(closure_system);
        ecs.exec(change_c1);
        ecs.exec(print_component);
        ecs.get_resource_mut::<CR>().0 = false;
        println!("CR is now {:?}", ecs.get_resource::<CR>().0);
        ecs.get_resource_mut::<CR>().0 = true;
        println!("CR is now {:?}", ecs.get_resource::<CR>().0);
    }

    fn print_component(c1: &C1, c2: &C2) {
        println!("c1 is {}, c2 is {}", c1.0, c2.0);
    }
    fn change_c1(c1: &mut C1) {
        c1.0 = false;
    }
}
