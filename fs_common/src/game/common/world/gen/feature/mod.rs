pub mod features;
pub mod placement_mods;

use std::fmt::Debug;

use rand::RngCore;

use crate::game::Registries;

use super::populator::ChunkContext;

pub type ProviderFn<T> = dyn Fn(&mut dyn rand::RngCore) -> T + Send + Sync;

pub trait ConfiguredFeature: Debug {
    fn try_place(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        seed: i32,
        rng: &mut dyn RngCore,
        registries: &Registries,
        ecs: &mut specs::World,
    );
}

#[derive(Debug)]
pub struct PlacedFeature {
    feature: Box<dyn ConfiguredFeature + Send + Sync>,
    placement_mods: Vec<Box<dyn PlacementModifier + Send + Sync>>,
}

impl PlacedFeature {
    pub fn new(feature: impl ConfiguredFeature + Send + Sync + 'static) -> Self {
        Self { feature: Box::new(feature), placement_mods: vec![] }
    }

    #[must_use]
    pub fn placement(mut self, modifier: impl PlacementModifier + Send + Sync + 'static) -> Self {
        self.placement_mods.push(Box::new(modifier));
        self
    }

    pub fn generate(
        &self,
        chunks: &mut ChunkContext<1>,
        seed: i32,
        rng: &mut dyn RngCore,
        registries: &Registries,
        ecs: &mut specs::World,
    ) {
        let mut positions = vec![(0, 0)];
        for m in &self.placement_mods {
            positions = positions
                .into_iter()
                .flat_map(|p| m.process(chunks, p, seed, rng, registries))
                .collect();
        }

        for pos in positions {
            self.feature
                .try_place(chunks, pos, seed, rng, registries, ecs);
        }
    }
}

pub trait PlacementModifier: Debug {
    fn process(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        seed: i32,
        rng: &mut dyn RngCore,
        registries: &Registries,
    ) -> Vec<(i32, i32)>;
}
