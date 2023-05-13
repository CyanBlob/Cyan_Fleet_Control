pub mod api;

use std::env;
#[allow(unused)]
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use pollster::FutureExt as _;
#[allow(unused)]
use tokio::io::{AsyncWriteExt};
#[allow(unused)]
use tokio::net::TcpListener; // provides Future.block_on()

use spacedust::apis::agents_api::*;
#[allow(unused)]
use spacedust::apis::contracts_api::*;
use spacedust::apis::fleet_api::*;
use spacedust::apis::systems_api::*;

use spacedust::apis::configuration::Configuration;
#[allow(unused)]
use spacedust::apis::default_api::register;
#[allow(unused)]
use spacedust::models::register_request::{Faction, RegisterRequest};

use spacedust::models::*;

use crate::spacetraders::ShipyardShipWithWaypoint;

use self::api::spacetraders::{Render, RenderWithWaypoints, ShipWithNav, SpaceTraders};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppData {
    // Example stuff:
    label: String,
    test: SpaceTraders,

    // this how you opt-out of serialization of a member
    #[serde(skip)]
    value: f32,
    #[serde(skip)]
    conf: Configuration,
    contracts: Vec<Contract>,
    ships: Vec<ShipWithNav>,
    waypoints: Vec<Waypoint>,
    shipyard_waypoint: Option<Waypoint>,
    shipyard_ships: Option<Vec<ShipyardShipWithWaypoint>>,
    log: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AppState {
    data: Arc<Mutex<AppData>>,
}

impl AppState {
    pub fn new(_cc: &eframe::CreationContext<'_>, data: Arc<Mutex<AppData>>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        //if let Some(storage) = cc.storage {
        //return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        //}
        Self { data }
    }
}
impl Default for AppData {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            test: SpaceTraders {},
            value: 2.7,
            conf: Configuration::new(),
            contracts: vec![],
            ships: vec![],
            waypoints: vec![],
            shipyard_waypoint: None,
            shipyard_ships: None,
            log: vec![],
        }
    }
}

impl AppData {
    /// Called once before the first frame.
    pub fn new() -> Self {

        let mut state = AppData::default();
        state.conf.bearer_access_token = Some(
            env::var("SPACETRADERS_TOKEN")
                .expect("SPACETRADERS_TOKEN environment variable must be set"),
        );
        state
    }
}

impl eframe::App for AppState {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });

        let data_arc = self.data.clone();
        let mut data_mutex = data_arc.lock().unwrap();
        let data = data_mutex.deref_mut();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Cyan Fleet Control");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut data.label);
            });

            ui.add(egui::Slider::new(&mut data.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                *&mut data.value += 1.0;
            }

            if ui.button("Get info").clicked() {
                match get_my_agent(&data.conf).block_on() {
                    Ok(res) => {
                        println!("{:#?}", res);
                        match get_waypoint(
                            &data.conf,
                            res.data.headquarters.rsplit_once("-").unwrap().0,
                            &res.data.headquarters,
                        )
                        .block_on()
                        {
                            Ok(w) => data.log.push(format!("{:?}", w)),
                            Err(_) => data.log.push("Failed to get waypoint info".to_owned()),
                        }
                    }
                    Err(err_res) => {
                        panic!("{:#?}", err_res);
                    }
                }
                println!("\n");
            }

            egui::Window::new("Contracts")
                .vscroll(true)
                .show(ctx, |ui| {
                    if data.contracts.len() == 0 {
                        ui.label("No contracts available or accepted");
                    }
                    for contract in &mut data.contracts {
                        contract.render(ui, &data.conf);
                    }
                    if ui.button("Fetch").clicked() {}
                });

            {
                egui::Window::new("Fleet (simple)")
                    .vscroll(true)
                    .show(ctx, |ui| {
                        if data.ships.len() == 0 {
                            ui.label("No ships found in fleet");
                        }
                        for ship in &mut data.ships {
                            ship.render_with_waypoints(ui, &data.conf, &data.waypoints);
                        }
                        if ui.button("Fetch").clicked() {}
                    });

                egui::Window::new("Waypoints")
                    .vscroll(true)
                    .show(ctx, |ui| {
                        if data.waypoints.len() == 0 {
                            ui.label("No waypoints found");
                        }
                        for waypoint in &mut data.waypoints {
                            waypoint.render(ui, &data.conf);
                        }
                        if ui.button("Fetch").clicked() {
                            if data.ships.len() == 0 {
                                data.log.push(
                                    "Cannot fetch waypoints with 0 ships. Fetch ships first".into(),
                                );
                            }
                        }
                    });
            }

            egui::Window::new("Shipyard")
                .constrain(true)
                .vscroll(true)
                .show(ctx, |ui| {
                    if let Some(s) = &data.shipyard_ships {
                        for ship in s.iter() {
                            ui.label(format!("Ship: {:?}", ship.ship.description));
                            ui.label(format!("\tEngine"));
                            ui.label(format!("\t\tName {:?}", ship.ship.engine.name));
                            ui.label(format!("\t\tCondition {:?}", ship.ship.engine.condition));
                            ui.label(format!("\t\tSpeed {:?}", ship.ship.engine.speed));
                            ui.label(format!("\tModules"));

                            /*for m in &ship.modules {
                                ui.label(format!("\t\tName {:?}", m.name));
                                ui.label(format!("\t\tRange {:?}", m.range));

                                if let Some(c) = m.capacity {
                                    ui.label(format!("\t\tCapacity {:?}", c));
                                }
                                if let Some(m) = m.range {
                                    ui.label(format!("\t\tRange {:?}", m));
                                }

                                //ui.label(format!("\tRequirements {:?}", m.requirements));
                            }*/
                            ui.label(format!("\tPrice: {:?}", ship.ship.purchase_price));

                            if ui.button("Purchase").clicked() {
                                let req = purchase_ship_request::PurchaseShipRequest {
                                    ship_type: ship.ship.r#type.unwrap(),
                                    waypoint_symbol: ship.waypoint.clone(),
                                };
                                purchase_ship(&data.conf, Some(req)).block_on();
                            }

                            ui.separator();
                        }
                    }
                });
        });

        egui::TopBottomPanel::bottom("")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                egui::scroll_area::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.code(data.log.join("\n\n")).context_menu(|ui| {
                            if ui.button("Clear log").clicked() {
                                data.log.clear()
                            }
                        })
                    });
            });
    }
}
