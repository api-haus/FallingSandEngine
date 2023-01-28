use std::collections::BTreeMap;

use egui::TextureOptions;
use fs_common::game::{
    common::world::material::placer::{self, MaterialPlacer, MaterialPlacerID},
    Registries,
};

pub struct DrawUI {
    textures: BTreeMap<u16, egui::TextureHandle>,
    pub selected: MaterialPlacerID,
}

impl DrawUI {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            textures: BTreeMap::new(),
            selected: placer::AIR_PLACER,
        }
    }

    pub fn render(&mut self, egui_ctx: &egui::Context, registries: &Registries) {
        for (id, (_meta, placer)) in &registries.material_placers {
            self.textures.entry(*id).or_insert_with(|| {
                egui_ctx.load_texture(
                    format!("{id}"),
                    gen_material_preview(placer.as_ref()),
                    TextureOptions::LINEAR,
                )
            });
        }

        egui::Window::new("Draw")
            .resizable(false)
            .show(egui_ctx, |ui| {
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Min)
                        .with_cross_align(egui::Align::Min)
                        .with_main_wrap(true),
                    |ui| {
                        for (id, tex) in &self.textures {
                            if ui
                                .add(
                                    egui::ImageButton::new(tex, (40.0, 40.0))
                                        .selected(*id == self.selected),
                                )
                                .on_hover_text(
                                    registries
                                        .material_placers
                                        .get(id)
                                        .unwrap()
                                        .0
                                        .display_name
                                        .to_string(),
                                )
                                .clicked()
                            {
                                self.selected = *id;
                            };
                        }
                    },
                );
            });
    }
}

fn gen_material_preview(placer: &dyn MaterialPlacer) -> egui::ColorImage {
    let width = 20;
    let height = 20;
    let fake_nearest_neighbor_scale = 3;
    let mut img = egui::ColorImage::new(
        [
            width * fake_nearest_neighbor_scale,
            height * fake_nearest_neighbor_scale,
        ],
        egui::Color32::TRANSPARENT,
    );
    for y in 0..height {
        for x in 0..width {
            let mat = placer.pixel(x as i64, y as i64);
            let col = egui::Rgba::from_srgba_unmultiplied(
                mat.color.r,
                mat.color.g,
                mat.color.b,
                mat.color.a,
            )
            .into();

            for xx in 0..fake_nearest_neighbor_scale {
                for yy in 0..fake_nearest_neighbor_scale {
                    img[(
                        x * fake_nearest_neighbor_scale + xx,
                        y * fake_nearest_neighbor_scale + yy,
                    )] = col;
                }
            }
        }
    }
    img
}
