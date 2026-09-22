use rj::AsAny;
use super::EcsContainer;
use super::EcsGlobalContainer;
use super::DynEcsContainer;
use super::DynEcsEntityContainer;

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

impl<T: 'static> EcsGlobalContainer for GlobalContainer<T> {
    fn get(&self) -> &Self::Item {
        &self.inner
    }

    fn get_mut(&mut self) -> &mut Self::Item {
        &mut self.inner
    }
}
