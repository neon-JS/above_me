use log::{error, info};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};

use ddb::fetch_aircraft;

mod api;
mod aprs;
mod config;
mod ddb;
mod haversine;
mod ogn;
mod position;
mod time;

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Loading config...");
    let config = match config::load() {
        Ok(c) => {
            info!("Loaded config successfully!");
            c
        }
        Err(e) => {
            error!("Could not load config: {e}");
            return;
        }
    };

    info!("Loading aircraft data...");
    let aircraft = match fetch_aircraft(&config.ddb_url).await {
        Ok(a) => {
            info!("Loaded aircraft data successfully!");
            a
        }
        Err(e) => {
            error!("Could not fetch aircraft data: {e}");
            return;
        }
    };

    let mut join_set = JoinSet::new();

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (status_tx, mut status_rx) = mpsc::channel(32);

    let app = api::App::create();
    let app_update = app.clone();

    join_set.spawn(async move {
        info!("Initializing API...");
        api::init(&config.bind_to, app, shutdown_rx)
            .await
            .expect("Could not start API server");
    });

    join_set.spawn(async move {
        info!("Initializing APRS client...");

        loop {
            if let Err(e) = aprs::init(&config.aprs, &status_tx, &aircraft).await {
                error!("Client stopped with error: {e}");
                break;
            } else {
                /* Server may disconnect us at some point. Just reconnect and carry on. */
                info!("Client disconnected. Reconnecting...");
            }
        }

        shutdown_tx.send(()).unwrap();
    });

    join_set.spawn(async move {
        while let Some(status) = status_rx.recv().await {
            app_update.push_status(status);
        }
    });

    while (join_set.join_next().await).is_some() {}
    info!("Shutdown");
}
