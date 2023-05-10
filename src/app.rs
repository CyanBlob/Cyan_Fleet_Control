pub mod api;

use std::env;

use pollster::FutureExt as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener; // provides Future.block_on()

use spacedust::apis::agents_api::*;
use spacedust::apis::contracts_api::*;
use spacedust::apis::fleet_api::*;
use spacedust::apis::systems_api::*;

use spacedust::apis::configuration::Configuration;
use spacedust::apis::default_api::register;
use spacedust::models::register_request::{Faction, RegisterRequest};

use spacedust::models::*;

use self::api::spacetraders::{Render, SpaceTraders, RenderWithWaypoints, ShipWithNav};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppState {
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
    log: Vec<String>,
}

impl Default for AppState {
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
            log: vec![],
        }
    }
}

impl AppState {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        //if let Some(storage) = cc.storage {
        //return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        //}

        let mut state = AppState::default();
        state.conf.bearer_access_token = Some(env::var("SPACETRADERS_TOKEN").expect("SPACETRADERS_TOKEN environment variable must be set"));
        state
    }
}

impl eframe::App for AppState {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Cyan Fleet Control");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                *&mut self.value += 1.0;
            }

            if ui.button("Get info").clicked() {
                match get_my_agent(&self.conf).block_on() {
                    Ok(res) => {
                        println!("{:#?}", res);
                        match get_waypoint(
                            &self.conf,
                            res.data.headquarters.rsplit_once("-").unwrap().0,
                            &res.data.headquarters,
                        )
                        .block_on()
                        {
                            Ok(w) => self.log.push(format!("{:?}", w)),
                            Err(_) => self.log.push("Failed to get waypoint info".to_owned()),
                        }
                    }
                    Err(err_res) => {
                        panic!("{:#?}", err_res);
                    }
                }
                println!("\n");
            }

            egui::Window::new("Contracts").show(ctx, |ui| {
                if self.contracts.len() == 0 {
                    ui.label("No contracts available or accepted");
                }
                for contract in &mut self.contracts {
                    contract.render(ui, &self.conf);
                }
                if ui.button("Fetch").clicked() {
                    match get_contracts(&self.conf, None, None).block_on() {
                        Ok(c) => {
                            self.log.push("Fetching contracts".into());
                            self.contracts.clear();

                            for contract in c.data {
                                println!("{:?}", contract);
                                self.contracts.push(contract);
                            }
                        }
                        Err(_) => self.log.push("Failed to get contracts".to_owned()),
                    }
                }
            });

            egui::Window::new("Fleet (simple)").show(ctx, |ui| {
                if self.ships.len() == 0 {
                    ui.label("No ships found in fleet");
                }
                for ship in &mut self.ships {
                    ship.render_with_waypoints(ui, &self.conf, &self.waypoints);
                }
                if ui.button("Fetch").clicked() {
                    match get_my_ships(&self.conf, None, None).block_on() {
                        Ok(f) => {
                            self.log.push("Fetching fleet".into());
                            self.ships.clear();

                            for ship in f.data {
                                println!("{:?}", ship);
                                let destination = ship.nav.waypoint_symbol.clone();
                                let ship_with_nav = ShipWithNav {ship, destination: destination};
                                self.ships.push(ship_with_nav);
                            }
                        }
                        Err(_) => self.log.push("Failed to update fleet".to_owned()),
                    }
                }
            });

            egui::Window::new("Waypoints").show(ctx, |ui| {
                if self.waypoints.len() == 0 {
                    ui.label("No waypoints found");
                }
                for waypoint in &mut self.waypoints {
                    waypoint.render(ui, &self.conf);
                }
                if ui.button("Fetch").clicked() {
                    if self.ships.len() == 0 {
                        self.log.push("Cannot fetch waypoints with 0 ships. Fetch ships first".into());
                    }
                    let mut visible_systems: Vec<&String>;
                    visible_systems = self
                        .ships
                        .iter()
                        .map(|ship| &ship.ship.nav.system_symbol)
                        .collect();
                    visible_systems.dedup();

                    self.waypoints.clear();
                    for system in visible_systems {
                        match get_system_waypoints(&self.conf, &system, None, None).block_on() {
                            Ok(w) => {
                                self.log.push(
                                    format!("Fetched waypoints for system: {}", system).into(),
                                );

                                for waypoint in w.data {
                                    self.waypoints.push(waypoint);
                                }
                            }
                            Err(_) => self.log.push(
                                format!("Failed to fetch waypoints for system: {}", system)
                                    .to_owned(),
                            ),
                        }
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
                    .show(ui, |ui| {
                        ui.code(self.log.join("\n\n")).context_menu(|ui| {
                            if ui.button("Clear log").clicked() {
                                self.log.clear()
                            }
                        })
                    });
            });
    }
}
