pub mod api;

use std::env;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

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

use self::api::spacetraders::{Render, RenderWithWaypoints, ShipWithNav, SpaceTraders};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
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
    shipyard_ships: Option<Vec<ShipyardShip>>,
    log: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AppState {
    data: Arc<Mutex<AppData>>,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>, data: Arc<Mutex<AppData>>) -> Self {
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
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        //if let Some(storage) = cc.storage {
        //return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        //}

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

        let mut data_arc = self.data.clone();
        let mut data_mutex = data_arc.lock().unwrap();
        let mut data = data_mutex.deref_mut();
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

            egui::Window::new("Contracts").show(ctx, |ui| {
                if data.contracts.len() == 0 {
                    ui.label("No contracts available or accepted");
                }
                for contract in &mut data.contracts {
                    contract.render(ui, &data.conf);
                }
                if ui.button("Fetch").clicked() {
                    match get_contracts(&data.conf, None, None).block_on() {
                        Ok(c) => {
                            data.log.push("Fetching contracts".into());
                            data.contracts.clear();

                            for contract in c.data {
                                println!("{:?}", contract);
                                data.contracts.push(contract);
                            }
                        }
                        Err(_) => data.log.push("Failed to get contracts".to_owned()),
                    }
                }
            });

            {
                egui::Window::new("Fleet (simple)").show(ctx, |ui| {
                    if data.ships.len() == 0 {
                        ui.label("No ships found in fleet");
                    }
                    for ship in &mut data.ships {
                        ship.render_with_waypoints(ui, &data.conf, &data.waypoints);
                    }
                    if ui.button("Fetch").clicked() {
                        data.log.push(format!("Fetching fleet: {:?}", &data.conf));
                        match get_my_ships(&data.conf, None, None).block_on() {
                            Ok(f) => {
                                data.log.push("Fetching fleet".into());
                                data.ships.clear();

                                for ship in f.data {
                                    println!("{:?}", ship);
                                    let destination = ship.nav.waypoint_symbol.clone();
                                    let ship_with_nav = ShipWithNav {
                                        ship,
                                        destination: destination,
                                    };
                                    data.ships.push(ship_with_nav);
                                }
                            }
                            Err(e) => {
                                data.log.push("Failed to update fleet".to_owned());
                                data.log.push(format!("{:?}", e));
                            }
                        }
                    }
                });

                egui::Window::new("Waypoints").show(ctx, |ui| {
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
                        let mut visible_systems: Vec<&String>;
                        visible_systems = data.ships
                            .iter()
                            .map(|ship| &ship.ship.nav.system_symbol)
                            .collect();
                        visible_systems.dedup();

                        data.waypoints.clear();
                        for system in visible_systems {
                            match get_system_waypoints(&data.conf, &system, None, None).block_on() {
                                Ok(w) => {
                                    data.log.push(
                                        format!("Fetched waypoints for system: {}", system).into(),
                                    );

                                    for waypoint in w.data {
                                        data.waypoints.push(waypoint);
                                    }
                                }
                                Err(_) => data.log.push(
                                    format!("Failed to fetch waypoints for system: {}", system)
                                        .to_owned(),
                                ),
                            }
                        }
                    }
                });
            }

            egui::Window::new("Shipyard").show(ctx, |ui| {
                let text = match &data.shipyard_waypoint {
                    Some(w) => w.symbol.clone(),
                    None => "None".to_owned(),
                };

                egui::ComboBox::from_label("Shipyard waypoint")
                    .selected_text(format!("{:?}", text))
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for waypoint in &data.waypoints {
                            for symbol in &waypoint.traits {
                                if symbol.symbol == waypoint_trait::Symbol::Shipyard {
                                    ui.selectable_value(
                                        &mut data.shipyard_waypoint,
                                        Some(waypoint.clone()),
                                        format!("{:?}", waypoint.symbol),
                                    );
                                }
                            }
                        }
                    });
                if let Some(w) = &data.shipyard_waypoint {
                    if ui.button("Load ships").clicked() {
                        match get_shipyard(&data.conf, &w.system_symbol, &w.symbol).block_on() {
                            Ok(r) => data.shipyard_ships = r.data.ships,
                            Err(_) => data.shipyard_ships = None,
                        }
                    }
                }

                if let Some(s) = &data.shipyard_ships {
                    for ship in s {
                        ui.label(format!("Ship: {:?}", ship.description));
                        ui.label(format!("\tEngine"));
                        ui.label(format!("\t\tName {:?}", ship.engine.name));
                        ui.label(format!("\t\tCondition {:?}", ship.engine.condition));
                        ui.label(format!("\t\tSpeed {:?}", ship.engine.speed));
                        ui.label(format!("\tModules"));

                        for m in &ship.modules {
                            ui.label(format!("\t\tName {:?}", m.name));
                            ui.label(format!("\t\tRange {:?}", m.range));

                            if let Some(c) = m.capacity {
                                ui.label(format!("\t\tCapacity {:?}", c));
                            }
                            if let Some(m) = m.range {
                                ui.label(format!("\t\tRange {:?}", m));
                            }

                            ui.label(format!("\tRequirements {:?}", m.requirements));
                        }
                        ui.label(format!("\tPrice: {:?}", ship.purchase_price));

                        if ui.button("Purchase").clicked() {
                            let req = purchase_ship_request::PurchaseShipRequest {
                                ship_type: ship.r#type.unwrap(),
                                waypoint_symbol: data
                                    .shipyard_waypoint
                                    .clone()
                                    .unwrap()
                                    .symbol
                                    .clone(),
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
