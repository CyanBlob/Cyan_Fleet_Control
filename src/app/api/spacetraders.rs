use egui::Ui;
use pollster::FutureExt;
use spacedust::apis::agents_api::*;
use spacedust::apis::contracts_api::*;
use spacedust::apis::default_api::*;
use spacedust::apis::fleet_api::*;
use spacedust::apis::systems_api::*;

use spacedust::models::*;

use spacedust::apis::configuration::Configuration;
use spacedust::apis::default_api::register;
use spacedust::models::register_request::{Faction, RegisterRequest};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SpaceTraders {}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ShipWithNav {
    pub ship: Ship,
    pub destination: String,
}

pub trait RenderWithWaypoints {
    fn render_with_waypoints(
        &mut self,
        ui: &mut Ui,
        conf: &Configuration,
        waypoints: &Vec<Waypoint>,
    ) where
        Self: std::fmt::Debug,
    {
        ui.code(format!("{:?}", &self));
    }
}

pub trait Render {
    fn render(&mut self, ui: &mut Ui, conf: &Configuration)
    where
        Self: std::fmt::Debug,
    {
        ui.code(format!("{:?}", &self));
    }
}

impl RenderWithWaypoints for ShipWithNav {
    fn render_with_waypoints(
        &mut self,
        ui: &mut Ui,
        conf: &Configuration,
        waypoints: &Vec<Waypoint>,
    ) where
        Self: std::fmt::Debug,
    {
        ui.vertical(|ui| {
            ui.label(format!("Symbol: {:?}", &self.ship.symbol));
            ui.label(format!("Current fuel: {:?}", &self.ship.fuel.current));
            ui.label(format!("Fuel capacity: {:?}", &self.ship.fuel.capacity));
            ui.label(format!("System: {:?}", &self.ship.nav.system_symbol));
            ui.label(format!("Waypoint: {:?}", &self.ship.nav.waypoint_symbol));

            ui.label(format!("Route start:"));
            ui.label(format!(
                "\tSymbol: {:?}",
                &self.ship.nav.route.departure.symbol
            ));
            ui.label(format!(
                "\tType: {:?}",
                &self.ship.nav.route.departure.r#type
            ));
            ui.label(format!(
                "Route start time: {:?}",
                &self.ship.nav.route.departure_time
            ));

            ui.label(format!("Route destination:"));
            ui.label(format!(
                "\tSymbol: {:?}",
                &self.ship.nav.route.destination.symbol
            ));
            ui.label(format!(
                "\tType: {:?}",
                &self.ship.nav.route.destination.r#type
            ));

            ui.label(format!(
                "Route arrival time: {:?}",
                &self.ship.nav.route.arrival
            ));

            ui.label(format!("Cargo space: {:?}", &self.ship.cargo.capacity));
            ui.label(format!("Cargo space used: {:?}", &self.ship.cargo.units));
            ui.label(format!(
                "Cargo space remaining: {:?}",
                &self.ship.cargo.capacity - &self.ship.cargo.units
            ));

            for cargo in &self.ship.cargo.inventory {
                ui.label(format!("Cargo:"));
                ui.label(format!("\tName: {:?}", cargo.name));
                ui.label(format!("\tUnits: {:?}", cargo.units));
            }

            ui.label(format!("Status: {:?}", &self.ship.nav.status));

            match &self.ship.nav.status {
                ShipNavStatus::InTransit => {}
                _ => {
                    ui.push_id(&self.ship.symbol, |ui| {
                        egui::ComboBox::from_label("Destination")
                            .selected_text(format!("{:?}", &mut self.destination))
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for waypoint in waypoints {
                                    ui.selectable_value(
                                        &mut self.destination,
                                        waypoint.symbol.clone(),
                                        format!("{:?}", waypoint.symbol),
                                    );
                                }
                            });
                    });
                    if self.destination != self.ship.nav.waypoint_symbol {
                        if ui.button("Begin journey").clicked() {
                            let req = navigate_ship_request::NavigateShipRequest::new(
                                self.destination.clone(),
                            );
                            match navigate_ship(&conf, &self.ship.symbol, Some(req)).block_on() {
                                Ok(_) => println!("Ship navigating successfully"),
                                Err(_) => println!("Could not start navigation"),
                            }
                        }
                    }
                }
            }

            match &self.ship.nav.status {
                ShipNavStatus::InOrbit => {
                    if ui.button("Dock").clicked() {
                        dock_ship(&conf, &self.ship.symbol, 0.0).block_on();
                    }
                }
                ShipNavStatus::Docked => {
                    if self.ship.cargo.capacity > self.ship.cargo.units {
                        if ui.button("Extract").clicked() {
                            let req = extract_resources_request::ExtractResourcesRequest { survey: None };
                            match extract_resources(&conf, &self.ship.symbol, Some(req)).block_on() {
                                Ok(r) => println!("{:?}", r),
                                Err(e) => println!("{:?}", e),
                            }
                        }
                    }
                    if ui.button("Deliver").clicked() {
                        let req = deliver_contract_request::DeliverContractRequest {
                            ship_symbol: self.ship.symbol.clone(),
                            trade_symbol: todo!(),
                            units: todo!(),
                        };
                        match deliver_contract(&conf, "", Some(req)).block_on() {
                            Ok(r) => println!("{:?}", r),
                            Err(e) => println!("{:?}", e),
                        }
                    }
                }
                _ => {}
            }

            ui.separator();
        });
    }
}

impl Render for Waypoint {
    fn render(&mut self, ui: &mut Ui, conf: &Configuration)
    where
        Self: std::fmt::Debug,
    {
        ui.vertical(|ui| {
            ui.label(format!("Symbol: {}", self.symbol.clone()));
            //ui.label(format!("Traits: {:?}", self.traits.clone()));

            if self.orbitals.len() > 0 {
                ui.label(format!(
                    "Orbitals: {:?}",
                    self.orbitals
                        .clone()
                        .iter()
                        .map(|o| &o.symbol)
                        .collect::<Vec::<&String>>()
                ));
            }

            self.traits.iter().for_each(|f| match f.symbol {
                waypoint_trait::Symbol::Shipyard => {
                    ui.label(format!("Shipyard: {}", f.name));
                }
                waypoint_trait::Symbol::TradingHub => {
                    ui.label(format!("Trading hub: {}", f.name));
                }
                waypoint_trait::Symbol::Marketplace => {
                    ui.label(format!("Marketplace: {}", f.name));
                }
                waypoint_trait::Symbol::BlackMarket => {
                    ui.label(format!("Black market: {}", f.name));
                }
                waypoint_trait::Symbol::MineralDeposits => {
                    ui.label(format!("Mineral deposits: {}", f.name));
                }
                _ => {}
            });
            ui.separator();
        });
    }
}

impl Render for Contract {
    fn render(&mut self, ui: &mut Ui, conf: &Configuration)
    where
        Self: std::fmt::Debug,
    {
        ui.vertical(|ui| {
            ui.label(format!("Type: {:?}", self.r#type));
            ui.label(format!("Faction: {:?}", self.faction_symbol));
            ui.label(format!(
                "Payment on accept: {:?}",
                self.terms.payment.on_accepted
            ));
            ui.label(format!(
                "Payment on complete: {:?}",
                self.terms.payment.on_fulfilled
            ));
            ui.label(format!(
                "Payment (total): {:?}",
                self.terms.payment.on_fulfilled + self.terms.payment.on_accepted
            ));
            ui.label(format!("Deadline: {:?}", self.terms.deadline));
            ui.label(format!("Expiration: {:?}", self.expiration));

            if let Some(d) = &self.terms.deliver {
                for delivery in d {
                    ui.label(format!("Delivery:"));
                    ui.label(format!("\tDeliver: {:?}", delivery.trade_symbol));
                    ui.label(format!("\tDestination: {:?}", delivery.destination_symbol));
                    ui.label(format!("\tRequired: {:?}", delivery.units_required));
                    ui.label(format!("\tFulfilled: {:?}", delivery.units_fulfilled));
                    ui.label(format!(
                        "\tRemaining: {:?}",
                        delivery.units_required - delivery.units_fulfilled
                    ));
                }
            }

            ui.label(format!("Accepted: {:?}", self.accepted));

            //ui.code(format!("{:?}", &self));
        });

        if self.accepted == false {
            if ui.button("Accept!").clicked() {
                match accept_contract(conf, &self.id, 0).block_on() {
                    Ok(r) => println!("{:?}", r),
                    Err(_) => println!("Failed to accept contract :("),
                }
            }
        }
    }
}

impl SpaceTraders {
    pub async fn create_account(&self, username: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create Configuration
        let mut conf = Configuration::new();

        // Create Register Request
        let reg_req = RegisterRequest::new(Faction::Quantum, "CyanBlob".to_string());

        // Register Agent
        let register_response = register(&conf, Some(reg_req)).await;

        match register_response {
            Ok(res) => {
                println!("{:#?}", res);
                // Update Config with Agent Token
                conf.bearer_access_token = Some(res.data.token);
            }
            Err(err_res) => {
                panic!("{:#?}", err_res);
            }
        }

        // Get Agent Details to Confirm Working
        match get_my_agent(&conf).await {
            Ok(res) => {
                println!("{:#?}", res);
                // Print Symbol
                println!("My Symbol: {:#?}", res.data.symbol);
            }
            Err(err_res) => {
                panic!("{:#?}", err_res);
            }
        }

        Ok(())
    }
}
