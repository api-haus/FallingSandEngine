use crate::game::common::world::{
    material::{self, color::Color, MaterialInstance, PhysicsType},
    CHUNK_SIZE,
};

use super::{ChunkContext, Populator};

pub struct TestPopulator;

impl<const S: u8> Populator<S> for TestPopulator {
    fn populate(&self, mut chunks: ChunkContext<S>, _seed: i32) {
        for x in 0..i32::from(CHUNK_SIZE) {
            for y in 0..i32::from(CHUNK_SIZE) {
                let m = chunks.get(x as i32, y as i32).unwrap();
                if m.material_id != material::AIR {
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            let m2 = chunks.get(x as i32 + dx, y as i32 + dy).unwrap();
                            if m2.material_id == material::AIR {
                                chunks
                                    .set(
                                        x as i32,
                                        y as i32,
                                        MaterialInstance {
                                            material_id: material::TEST,
                                            physics: PhysicsType::Solid,
                                            color: Color::ROSE,
                                        },
                                    )
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}