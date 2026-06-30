// the renderer needs to own a cache of all the compiled pipeline variants so this is that

// this is overly complicated and hacky because the key needs to be type erased
// for this code not to be duplicated across every pipeline type.

trait PipelineKey {
    fn eq(&self, other: &dyn PipelineKey) -> bool;
    fn hash(&self) -> u64;
    fn any(&self) -> &dyn std::any::Any;
}

impl<T: Eq + std::hash::Hash + 'static> PipelineKey for T {
    fn eq(&self, other: &dyn PipelineKey) -> bool {
        if let Some(key) = other.any().downcast_ref::<T>() {
            self == key
        } else {
            false
        }
    }
    fn hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&(std::any::TypeId::of::<T>(), self), &mut hasher);
        std::hash::Hasher::finish(&hasher)
    }
    fn any(&self) -> &dyn std::any::Any {
        self
    }
}

impl PartialEq for Box<dyn PipelineKey> {
    fn eq(&self, other: &Self) -> bool {
        PipelineKey::eq(self.as_ref(), other.as_ref())
    }
}

impl Eq for Box<dyn PipelineKey> {}

impl std::hash::Hash for Box<dyn PipelineKey> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let hash = PipelineKey::hash(self.as_ref());
        state.write_u64(hash);
    }
}

fn key_of(key: impl Eq + std::hash::Hash + 'static) -> Box<dyn PipelineKey> {
    Box::new(key)
}

pub(crate) trait Pipeline {
    type Variant: Eq + PartialEq + std::hash::Hash + Clone + 'static;

    fn compile(renderer: &crate::Renderer, v: &Self::Variant) -> wgpu::RenderPipeline;
    fn get(renderer: &crate::Renderer, v: Self::Variant) -> wgpu::RenderPipeline {
        if let Some(pipeline) = renderer
            .pipelines
            .lock()
            .unwrap()
            .map
            .get(&key_of(v.clone()))
        {
            return pipeline.clone();
        }
        let pipeline = Self::compile(renderer, &v);
        renderer
            .pipelines
            .lock()
            .unwrap()
            .map
            .insert(key_of(v.clone()), pipeline);
        renderer
            .pipelines
            .lock()
            .unwrap()
            .map
            .get(&key_of(v.clone()))
            .unwrap()
            .clone()
    }
}

#[derive(Default)]
pub(crate) struct RenderPipelineCache {
    map: std::collections::HashMap<Box<dyn PipelineKey>, wgpu::RenderPipeline>,
}
