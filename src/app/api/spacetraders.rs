use egui::Ui;
use pollster::FutureExt;
use spacedust::apis::agents_api::*;
use spacedust::apis::contracts_api::*;
use spacedust::apis::systems_api::*;

use spacedust::models::*;

use spacedust::apis::configuration::Configuration;
use spacedust::apis::default_api::register;
use spacedust::models::register_request::{Faction, RegisterRequest};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SpaceTraders {}

pub trait Render {
    fn render(&mut self, ui: &mut Ui, conf: &Configuration)
    where
        Self: std::fmt::Debug,
    {
        ui.code(format!("{:?}", &self));
    }
}

impl Render for Ship {}
impl Render for Waypoint {}

impl Render for Contract {
    fn render(&mut self, ui: &mut Ui, conf: &Configuration)
    where
        Self: std::fmt::Debug,
    {
        ui.code(format!("{:?}", &self));

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
        println!("CREATE ACCOUNT");
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
