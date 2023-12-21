use barrier_client::{self, start};
use clap::Parser;
use env_logger::Env;

use log::{debug, error, info, warn};
use ser_actuator::SerbarActuator;
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::sync::CancellationToken;

mod ser_actuator;

#[derive(Parser, Debug)]
pub struct SerbarConfig {
    /// Barrier server address in "server:port" format
    #[arg(short = 's', long, env = "BARRIER_SERVER")]
    pub server: String,
    /// Screen name, must be accepted by the Barrier server
    #[arg(short = 'n', long, env = "SCREEN_NAME")]
    pub screen_name: String,
    /// Screen width
    #[arg(short = 'w', long, default_value = "1920", env = "SCREEN_WIDTH")]
    pub screen_width: u16,
    /// Screen height
    #[arg(short = 'e', long, default_value = "1080", env = "SCREEN_HEIGHT")]
    pub screen_height: u16,
    /// Flip mouse wheel
    #[arg(short = 'f', long, default_value = "false")]
    pub flip_mouse_wheel: bool,

    // USB ids
    #[arg(hide = true, long, default_value = "3338")]
    pub usb_vid: u16,
    #[arg(hide = true, long, default_value = "49374")]
    pub usb_pid: u16,
    #[arg(hide = true, long, default_value = "12345678")]
    pub usb_serial: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = SerbarConfig::parse();

    let ports = tokio_serial::available_ports()?;
    let mut path: Option<String> = None;
    for p in ports {
        match p.port_type {
            tokio_serial::SerialPortType::UsbPort(info) => {
                debug!(
                    "USB Port {}:{} : {} - {}",
                    info.vid,
                    info.pid,
                    p.port_name,
                    info.product.unwrap_or(String::from("Unknown"))
                );

                if info.vid == args.usb_vid
                    && info.pid == args.usb_pid
                    && info.serial_number == Some(args.usb_serial.clone())
                {
                    info!("Found Pico KVM at {}", p.port_name);
                    path = Some(p.port_name);
                }
            }
            tokio_serial::SerialPortType::BluetoothPort => {
                debug!("Bluetooth Port: {}", p.port_name);
            }
            tokio_serial::SerialPortType::PciPort => {
                debug!("PCI Port: {}", p.port_name);
            }
            tokio_serial::SerialPortType::Unknown => {
                debug!("Unknown Port: {}", p.port_name);
            }
        }
    }

    if path.is_none() {
        error!("No Pico KVM found");
        return Ok(());
    }

    let port = tokio_serial::new(path.clone().unwrap(), 115200).open_native_async()?;

    let token = CancellationToken::new();
    let cloned_token: CancellationToken = token.clone();
    tokio::task::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sighup = signal(SignalKind::hangup()).unwrap();
        loop {
            select! {
                _ = sigterm.recv() => info!("Recieve SIGTERM, shutting down..."),
                _ = sigint.recv() => info!("Recieve SIGINT, shutting down..."),
                _ = sighup.recv() => info!("Recieve SIGHUP, shutting down..."),
            };
            cloned_token.cancel();
        }
    });

    let mut actuator = SerbarActuator::new(
        args.screen_width,
        args.screen_height,
        args.flip_mouse_wheel,
        port,
    );
    let main_task = async move {
        start(args.server, args.screen_name, &mut actuator)
            .await
            .ok();
    };

    let join_handle = tokio::spawn(async move {
        select! {
            _ = token.cancelled() => (),
            _ = main_task => (),
        }
    });

    match join_handle.await {
        Ok(_) => {}
        Err(e) => {
            warn!("Error: {:?}", e);
        }
    }

    // TODO: Fix lifetime issue, this is too stupid
    let port = tokio_serial::new(path.clone().unwrap(), 115200).open_native_async()?;
    let mut actuator = SerbarActuator::new(
        args.screen_width,
        args.screen_height,
        args.flip_mouse_wheel,
        port,
    );
    actuator.clear().await?;

    Ok(())
}
