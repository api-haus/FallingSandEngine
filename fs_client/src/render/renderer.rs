use std::fs;

use egui::{plot::HLine, Align2, RichText, WidgetText};
use fs_common::game::{
    common::{
        world::{material::Color, Position, Velocity},
        FileHelper, Rect,
    },
    GameData,
};
use glium::{Blend, Display, DrawParameters, PolygonMode};
use glium_glyph::{
    glyph_brush::{rusttype::Font, Section},
    GlyphBrush,
};
use glutin::{dpi::LogicalSize, event_loop::EventLoop};
use specs::ReadStorage;

use crate::{
    render::egui::DebugUI,
    world::{ClientChunk, WorldRenderer},
    Client,
};

use super::{drawing::RenderTarget, shaders::Shaders};

pub static mut BUILD_DATETIME: Option<&str> = None;
pub static mut GIT_HASH: Option<&str> = None;

pub struct Renderer<'a> {
    // pub fonts: Fonts,
    pub glyph_brush: GlyphBrush<'a, 'a>,
    pub shaders: Shaders,
    pub display: Display,
    pub world_renderer: WorldRenderer,
    pub egui_glium: egui_glium::EguiGlium,
    // pub version_info_cache_1: Option<(u32, u32, GPUImage)>,
    // pub version_info_cache_2: Option<(u32, u32, GPUImage)>,
}

pub struct Fonts {
    // pub pixel_operator: Font<'ttf, 'static>,
}

impl<'a> Renderer<'a> {
    #[profiling::function]
    pub fn create(event_loop: &EventLoop<()>, file_helper: &FileHelper) -> Result<Self, String> {
        let wb = glutin::window::WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1200_i16, 800_i16))
            .with_title("FallingSandRust");
        let cb = glutin::ContextBuilder::new();
        let display = glium::Display::new(wb, cb, event_loop).unwrap();

        let egui_glium = egui_glium::EguiGlium::new(&display);

        log::info!("glversion = {:?}", display.get_opengl_version());

        let shaders = Shaders::new(&display, file_helper);

        let pixel_operator =
            fs::read(file_helper.asset_path("font/pixel_operator/PixelOperator.ttf")).unwrap();
        let fonts = vec![Font::from_bytes(pixel_operator).unwrap()];

        let glyph_brush = GlyphBrush::new(&display, fonts);

        Ok(Renderer {
            glyph_brush,
            shaders,
            display,
            world_renderer: WorldRenderer::new(),
            egui_glium,
            // version_info_cache_1: None,
            // version_info_cache_2: None,
        })
    }

    #[profiling::function]
    pub fn render(
        &mut self,
        game: &mut GameData<ClientChunk>,
        client: &mut Client,
        delta_time: f64,
        partial_ticks: f64,
    ) {
        let mut target = RenderTarget::new(&mut self.display, &self.shaders, &mut self.glyph_brush);
        target.clear(Color::BLACK);

        Self::render_internal(
            &mut self.world_renderer,
            &mut target,
            game,
            client,
            delta_time,
            partial_ticks,
        );

        {
            profiling::scope!("version info");

            target.queue_text(Section {
                text: "Development Build",
                screen_position: (4.0, target.height() as f32 - 40.0),
                bounds: (150.0, 20.0),
                color: Color::WHITE.into(),
                ..Section::default()
            });
            target.queue_text(Section {
                text: format!(
                    "{} ({})",
                    unsafe { BUILD_DATETIME }.unwrap_or("???"),
                    unsafe { GIT_HASH }.unwrap_or("???")
                )
                .as_str(),
                screen_position: (4.0, target.height() as f32 - 20.0),
                bounds: (200.0, 20.0),
                color: Color::WHITE.into(),
                ..Section::default()
            });
            target.draw_queued_text();
        }

        {
            profiling::scope!("egui");

            self.egui_glium.run(&self.display, |egui_ctx| {
                if game.settings.debug {
                    // TODO: reimplement vsync for glutin
                    // let last_vsync = game.settings.vsync;
                    // let last_minimize_on_lost_focus = game.settings.minimize_on_lost_focus;

                    egui::Window::new("Debug").show(egui_ctx, |ui| {
                        if let Some(w) = &client.world {
                            if let Some(eid) = w.local_entity {
                                if let Some(world) = &game.world {
                                    let (
                                        velocity_storage,
                                        position_storage,
                                    ) = world.ecs.system_data::<(
                                        ReadStorage<Velocity>,
                                        ReadStorage<Position>,
                                    )>();

                                    let pos = position_storage
                                        .get(eid)
                                        .expect("Missing Position component on local_entity");

                                    let vel = velocity_storage
                                        .get(eid)
                                        .expect("Missing Velocity component on local_entity");

                                    ui.label(format!("({:.2}, {:.2})", pos.x, pos.y));
                                    ui.label(format!("({:.2}, {:.2})", vel.x, vel.y));
                                }
                            }
                        }

                        game.settings.debug_ui(ui);
                    });

                    // TODO: this should be somewhere better
                    // maybe clone the Settings before each frame and at the end compare it?

                    // if game.settings.vsync != last_vsync {
                    //     let si_des = if game.settings.vsync {
                    //         SwapInterval::VSync
                    //     } else {
                    //         SwapInterval::Immediate
                    //     };

                    //     self.sdl.as_ref().unwrap().sdl_video.gl_set_swap_interval(si_des).unwrap();
                    // }

                    // if last_minimize_on_lost_focus != game.settings.minimize_on_lost_focus {
                    //     sdl2::hint::set_video_minimize_on_focus_loss(
                    //         game.settings.minimize_on_lost_focus,
                    //     );
                    // }
                }

                egui::Window::new("stats")
                    .title_bar(false)
                    .anchor(Align2::RIGHT_BOTTOM, [0.0, 0.0])
                    .default_pos([target.width() as f32, target.height() as f32])
                    .default_width(200.0)
                    .show(egui_ctx, |ui| {
                        let a = match game.process_stats.cpu_usage {
                            Some(c) => format!("CPU: {:.0}%", c),
                            None => "CPU: n/a".to_string(),
                        };
                        let b = match game.process_stats.memory {
                            Some(m) => format!(" mem: {:.1} MB", m as f32 / 1000.0),
                            None => " mem: n/a".to_string(),
                        };

                        let text = format!("{a} {b}");

                        ui.label(text);

                        let nums: Vec<f32> = game
                            .fps_counter
                            .frame_times
                            .iter()
                            .filter(|n| **n != 0.0)
                            .map(|f| *f / 1_000_000.0)
                            .collect();
                        let avg_mspf: f32 = nums.iter().sum::<f32>() / nums.len() as f32;

                        let chart = egui::plot::BarChart::new(
                            nums.iter()
                                .enumerate()
                                .map(|(i, v)| egui::plot::Bar::new(i as f64, *v as f64))
                                .collect(),
                        );

                        egui::plot::Plot::new("frame_times")
                            .view_aspect(3.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .show_axes([false, true])
                            .include_y(50.0)
                            .show(ui, |plot_ui| {
                                plot_ui.text(egui::plot::Text::new(
                                    egui::plot::Value::new(nums.len() as f32 / 2.0, 45.0),
                                    WidgetText::RichText(
                                        RichText::new(format!(
                                            "mspf: {:.2} fps: {:.0}/{:.0}",
                                            avg_mspf,
                                            1000.0 / avg_mspf,
                                            1_000_000_000.0
                                                / game
                                                    .fps_counter
                                                    .frame_times
                                                    .iter()
                                                    .copied()
                                                    .reduce(f32::max)
                                                    .unwrap()
                                        ))
                                        .size(14.0),
                                    ),
                                ));
                                plot_ui.hline(HLine::new(1000.0 / 144.0).name("144"));
                                plot_ui.hline(HLine::new(1000.0 / 60.0).name("60"));
                                plot_ui.hline(HLine::new(1000.0 / 30.0).name("30"));
                                plot_ui.bar_chart(chart);
                            });

                        let nums: Vec<f32> = game
                            .fps_counter
                            .tick_times
                            .iter()
                            .filter(|n| **n != 0.0)
                            .map(|f| *f / 1_000_000.0)
                            .collect();
                        let avg_mspt: f32 = nums.iter().sum::<f32>() / nums.len() as f32;

                        let chart = egui::plot::BarChart::new(
                            nums.iter()
                                .enumerate()
                                .map(|(i, v)| egui::plot::Bar::new(i as f64, *v as f64))
                                .collect(),
                        );

                        egui::plot::Plot::new("tick_mspt")
                            .view_aspect(3.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .show_axes([false, true])
                            .include_y(30.0)
                            .show(ui, |plot_ui| {
                                plot_ui.text(egui::plot::Text::new(
                                    egui::plot::Value::new(nums.len() as f32 / 2.0, 27.0),
                                    WidgetText::RichText(
                                        RichText::new(format!("tick mspt: {:.2}", avg_mspt))
                                            .size(14.0),
                                    ),
                                ));
                                plot_ui.bar_chart(chart)
                            });

                        let nums: Vec<f32> = game
                            .fps_counter
                            .tick_physics_times
                            .iter()
                            .filter(|n| **n != 0.0)
                            .map(|f| *f / 1_000_000.0)
                            .collect();
                        let avg_mspt_physics: f32 = nums.iter().sum::<f32>() / nums.len() as f32;

                        let chart = egui::plot::BarChart::new(
                            nums.iter()
                                .enumerate()
                                .map(|(i, v)| egui::plot::Bar::new(i as f64, *v as f64))
                                .collect(),
                        );

                        egui::plot::Plot::new("phys_mspt")
                            .view_aspect(3.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .show_axes([false, true])
                            .include_y(10.0)
                            .show(ui, |plot_ui| {
                                plot_ui.text(egui::plot::Text::new(
                                    egui::plot::Value::new(nums.len() as f32 / 2.0, 9.0),
                                    WidgetText::RichText(
                                        RichText::new(format!(
                                            "phys mspt: {:.2}",
                                            avg_mspt_physics
                                        ))
                                        .size(14.0),
                                    ),
                                ));
                                plot_ui.bar_chart(chart)
                            });
                    });

                client.main_menu.render(egui_ctx, &game.file_helper);
                if let Some(debug_ui) = &mut client.debug_ui {
                    debug_ui.render(egui_ctx);
                }
            });

            {
                profiling::scope!("egui_glium::paint");
                self.egui_glium.paint(&self.display, &mut target.frame);
            }
        }

        target.finish().unwrap();
    }

    #[profiling::function]
    fn render_internal(
        world_renderer: &mut WorldRenderer,
        target: &mut RenderTarget,
        game: &mut GameData<ClientChunk>,
        client: &mut Client,
        delta_time: f64,
        partial_ticks: f64,
    ) {
        target.base_transform.push();
        target.base_transform.translate(-1.0, 1.0);
        target
            .base_transform
            .scale(2.0 / target.width() as f64, -2.0 / target.height() as f64);

        {
            profiling::scope!("test stuff");
            target.rectangle(
                Rect::new_wh(
                    40.0 + ((game.tick_time as f32 / 5.0).sin() * 20.0),
                    30.0 + ((game.tick_time as f32 / 5.0).cos().abs() * -10.0),
                    15.0,
                    15.0,
                ),
                Color::rgb(255, 0, 0),
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    ..Default::default()
                },
            );

            let rects = (0..10000)
                .step_by(15)
                .map(|i| {
                    let thru = (i as f32 / 10000.0 * 255.0) as u8;
                    let thru2 = (((i % 1000) as f32 / 1000.0) * 255.0) as u8;
                    let timeshift = ((1.0 - ((i % 1000) as f32 / 1000.0)).powi(8) * 200.0) as i32;

                    let color = Color::rgba(0, thru, 255 - thru, thru2);

                    (
                        Rect::new_wh(
                            75.0 + (i as f32 % 1000.0)
                                + (((game.frame_count as f32 / 2.0 + (i as i32 / 2) as f32
                                    - timeshift as f32)
                                    / 100.0)
                                    .sin()
                                    * 50.0),
                            (i as f32 / 1000.0) * 100.0
                                + (((game.frame_count as f32 / 2.0 + (i as i32 / 2) as f32
                                    - timeshift as f32)
                                    / 100.0)
                                    .cos()
                                    * 50.0),
                            20.0,
                            20.0,
                        ),
                        color,
                    )
                })
                .collect::<Vec<_>>();
            target.rectangles_colored(
                &rects,
                DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
        }

        if let Some(w) = &mut game.world {
            // let pixel_operator2 = self.sdl.as_ref().unwrap()
            //     .sdl_ttf
            //     .load_font(
            //         game.file_helper.asset_path("font/pixel_operator/PixelOperator.ttf"),
            //         16,
            //     )
            //     .unwrap();
            // let f = Fonts { pixel_operator: pixel_operator2 };

            world_renderer.render(w, target, delta_time, &game.settings, client, partial_ticks);
        }

        target.base_transform.pop();
    }
}
