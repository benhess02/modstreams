use std::{
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui;
use modstreams_core::{ModstreamsClient, Packet};

struct Control {
    subscribed_channel_name: String,
    channel_name: String,
    value: String,
    min_num: f32,
    max_num: f32,
    increment: f64,
}

fn run_read_thread(
    mut client: ModstreamsClient,
    controls: Arc<Mutex<Vec<Control>>>,
    ctx: egui::Context,
) {
    loop {
        if let Packet::Message { channel, content } = client.read().unwrap() {
            if let Ok(s) = String::from_utf8(content) {
                let mut _controls = controls.lock().unwrap();
                for control in _controls.iter_mut() {
                    if control.subscribed_channel_name == channel {
                        control.value = s.clone();
                    }
                }
                ctx.request_repaint();
            }
        }
    }
}

struct ControlsApp {
    client: ModstreamsClient,
    controls: Arc<Mutex<Vec<Control>>>,
    color: egui::Color32,
}

impl ControlsApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        client: ModstreamsClient,
        controls: Arc<Mutex<Vec<Control>>>,
    ) -> Self {
        let read_client = client.try_clone().unwrap();
        let read_controls = controls.clone();
        let read_ctx = cc.egui_ctx.clone();
        thread::spawn(move || run_read_thread(read_client, read_controls, read_ctx));
        Self {
            client,
            controls,
            color: egui::Color32::BLACK,
        }
    }
}

impl eframe::App for ControlsApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let mut controls = self.controls.lock().unwrap();
            let mut to_remove = None;
            for i in 0..controls.len() {
                let control = &mut controls[i];
                let channel_response =
                    ui.add(egui::TextEdit::singleline(&mut control.channel_name));
                let value_response = ui.add_enabled(
                    control.channel_name.len() != 0,
                    egui::TextEdit::singleline(&mut control.value),
                );

                let mut changed = value_response.changed();

                match control.value.parse::<f32>() {
                    Ok(mut v) => {
                        control.min_num = control.min_num.min(v);
                        control.max_num = control.max_num.max(v);
                        let slider_response = ui.add(
                            egui::widgets::Slider::new(&mut v, control.min_num..=control.max_num)
                                .step_by(control.increment)
                                .show_value(false),
                        );
                        if slider_response.changed() {
                            control.value = v.to_string();
                            changed = true;
                        }
                        ui.horizontal(|ui| {
                            ui.label("Min");
                            ui.add(egui::DragValue::new(&mut control.min_num));
                            ui.label("Max");
                            ui.add(egui::DragValue::new(&mut control.max_num));
                            ui.label("Step");
                            ui.add(egui::DragValue::new(&mut control.increment));
                        });
                    }
                    Err(_) => {}
                }

                if channel_response.changed() {
                    self.client
                        .unsubscribe(&control.subscribed_channel_name)
                        .unwrap();
                    self.client.subscribe(&control.channel_name).unwrap();
                    control.subscribed_channel_name = control.channel_name.clone();
                }

                if changed {
                    self.client
                        .send(&control.channel_name, control.value.as_bytes())
                        .unwrap();
                }

                if ui.button("Remove").clicked() {
                    to_remove = Some(i);
                }
                ui.add_space(10.);
            }

            if let Some(index) = to_remove {
                let removed = controls.remove(index);
                self.client.unsubscribe(&removed.channel_name).unwrap();
            }

            if ui.button("Add").clicked() {
                controls.push(Control {
                    subscribed_channel_name: String::new(),
                    channel_name: String::new(),
                    value: String::new(),
                    min_num: 0.,
                    max_num: 100.,
                    increment: 1.,
                });
            }
        });
    }
}

fn main() {
    let client = ModstreamsClient::new(7460);
    let controls = Arc::new(Mutex::new(Vec::new()));
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Controls",
        native_options,
        Box::new(|cc| Ok(Box::new(ControlsApp::new(cc, client, controls)))),
    )
    .unwrap();
}
