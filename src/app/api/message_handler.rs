use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use pollster::FutureExt;
#[allow(unused)]
use spacedust::apis::configuration::Configuration;
use strum_macros::EnumIter;

#[allow(unused)]
use spacedust::apis::agents_api::*;
#[allow(unused)]
use spacedust::apis::contracts_api::*;
#[allow(unused)]
use spacedust::apis::default_api::*;
use spacedust::apis::fleet_api::*;
use spacedust::apis::systems_api::*;

use spacedust::models::*;

#[allow(unused)]
use spacedust::apis::default_api::register;

use crate::spacetraders::{ShipWithNav, ShipyardShipWithWaypoint};
use crate::AppData;

#[derive(Debug, EnumIter, Clone, Copy)]
pub enum Message {
    GetFleet,
    GetWaypoints,
    GetContracts,
    GetShipyards,
}

pub struct MessageHandler;

impl MessageHandler {
    pub async fn handle_message(&self, m: &Message, state: Arc<Mutex<AppData>>) {
        let mut state_guard = state.lock().unwrap();
        let data = state_guard.deref_mut();
        data.log.push(format!("Handling message: {:?}", &m));
        match m {
            Message::GetFleet => match get_my_ships(&data.conf, None, None).block_on() {
                Ok(f) => {
                    data.log.push("Fetching fleet".into());
                    data.ships.clear();

                    for ship in f.data {
                        let destination = ship.nav.waypoint_symbol.clone();
                        let ship_with_nav = ShipWithNav {
                            ship,
                            destination: destination,
                        };
                        data.ships.push(ship_with_nav);
                    }
                }
                Err(_) => {}
            },
            Message::GetWaypoints => {
                let visible_systems = &self.get_visible_systems(data);

                data.waypoints.clear();
                for system in visible_systems {
                    match get_system_waypoints(&data.conf, &system, None, None).block_on() {
                        Ok(w) => {
                            data.log
                                .push(format!("Fetched waypoints for system: {}", system).into());

                            for waypoint in w.data {
                                data.waypoints.push(waypoint);
                            }
                        }
                        Err(_) => data.log.push(
                            format!("Failed to fetch waypoints for system: {}", system).to_owned(),
                        ),
                    }
                }
            }
            Message::GetContracts => match get_contracts(&data.conf, None, None).block_on() {
                Ok(c) => {
                    data.log.push("Fetching contracts".into());
                    data.contracts.clear();

                    for contract in c.data {
                        data.contracts.push(contract);
                    }
                }
                Err(_) => data.log.push("Failed to get contracts".to_owned()),
            },
            Message::GetShipyards => {
                data.shipyard_ships = None;

                let mut ships: Vec<ShipyardShipWithWaypoint> = vec![];
                for w in &data.waypoints {
                    if w.traits
                        .iter()
                        .any(|f| f.symbol == waypoint_trait::Symbol::Shipyard)
                    {
                        match get_shipyard(&data.conf, &w.system_symbol, &w.symbol).block_on() {
                            Ok(r) => {
                                if let Some(s) = r.data.ships {
                                    for ship in s {
                                        ships.push(ShipyardShipWithWaypoint {ship: ship, waypoint: w.symbol.clone()});
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    }

                    if ships.len() == 0 {
                        data.shipyard_ships = None;
                    }

                    if let Some(s) = &mut data.shipyard_ships {
                        s.clear();
                    }

                    for i in 0..ships.len() {
                        if let None = data.shipyard_ships {
                            data.shipyard_ships = Some(vec![]);
                        }

                        let tmp = data.shipyard_ships.as_mut();

                        if let Some(s) = tmp {
                            s.push(ships.get(i).unwrap().clone());
                        }
                    }
                }
            }
        }
    }
    fn get_visible_systems(&self, data: &AppData) -> Vec<String> {
        let mut visible_systems: Vec<String>;
        visible_systems = data
            .ships
            .iter()
            .map(|ship| ship.ship.nav.system_symbol.clone())
            .collect();
        visible_systems.dedup();
        visible_systems
    }
}
