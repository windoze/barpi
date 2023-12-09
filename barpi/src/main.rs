use std::{
    env, fs::File, io::BufReader, os::linux::fs::MetadataExt, path::PathBuf, thread::sleep,
    time::Duration,
};

use barrier_client::start;
use clap::Parser;
use clap_serde_derive::{serde::Serialize, ClapSerde};
use env_logger::Env;
use log::{debug, info, warn};
use synergy_hid::{ReportType, SynergyHid};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};
use tokio_util::sync::CancellationToken;
use usb_gadget::{
    default_udc,
    function::{hid::Hid, Handle},
    Class, Config, Gadget, Id, RegGadget, Strings,
};

mod client;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input files
    input: Vec<std::path::PathBuf>,

    /// Config file
    #[arg(short, long = "config", default_value = "config.yml")]
    config_path: std::path::PathBuf,

    /// Rest of arguments
    #[command(flatten)]
    pub config: <BarpiConfig as ClapSerde>::Opt,
}

#[derive(ClapSerde, Serialize, Debug)]
pub struct BarpiConfig {
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

    #[arg(hide = true, long, default_value = "3338")]
    pub usb_vid: u16,
    #[arg(hide = true, long, default_value = "49374")]
    pub usb_pid: u16,
    #[arg(hide = true, long, default_value = "0d0a.com")]
    pub usb_manufacturer: String,
    #[arg(hide = true, long, default_value = "BarPi HID Device")]
    pub usb_product: String,
    #[arg(hide = true, long, default_value = "0000000000000001")]
    pub usb_serial: String,
}

pub fn reg(funcs: Vec<Handle>, cfg: &BarpiConfig) -> RegGadget {
    let udc = default_udc().expect("cannot get UDC");

    let mut config = Config::new("config");
    for func in funcs {
        config = config.with_function(func);
    }

    let reg = Gadget::new(
        Class::new(0, 0, 0),
        Id::new(cfg.usb_vid, cfg.usb_pid),
        Strings::new(&cfg.usb_manufacturer, &cfg.usb_product, &cfg.usb_serial),
    )
    .with_config(config)
    .bind(&udc)
    .expect("cannot bind to UDC");

    assert!(reg.is_attached());
    assert_eq!(reg.udc().unwrap().unwrap(), udc.name());

    println!(
        "bound USB gadget {} at {} to {}",
        reg.name().to_string_lossy(),
        reg.path().display(),
        udc.name().to_string_lossy()
    );

    sleep(Duration::from_secs(3));

    reg
}

pub fn unreg(mut reg: RegGadget) -> std::io::Result<bool> {
    if env::var_os("KEEP_GADGET").is_some() {
        reg.detach();
        Ok(false)
    } else {
        reg.remove()?;
        sleep(Duration::from_secs(1));
        Ok(true)
    }
}

pub fn get_dev(prefix: &str, major: u8, minor: u8) -> anyhow::Result<PathBuf> {
    for entry in glob::glob(&format!("/dev/{prefix}*")).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let dev = std::fs::metadata(&path)
                    .expect("Failed to read metadata")
                    .st_rdev();
                if dev == (major as u64) << 8 | minor as u64 {
                    return Ok(path);
                }
            }
            Err(e) => return Err(e)?,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Device {major}:{minor} not found"),
    ))?
}

pub fn get_dev_for_hid(hid: &Hid) -> anyhow::Result<PathBuf> {
    let (major, minor) = hid.device()?;
    get_dev("hid", major, minor)
}

fn get_hid_func(report_type: ReportType) -> (Hid, Handle) {
    let (report_len, descriptor) = SynergyHid::get_report_descriptor(report_type);
    let mut builder = Hid::builder();
    builder.protocol = 1;
    builder.sub_class = 1;
    builder.report_len = report_len;
    builder.report_desc = descriptor.to_vec();
    let (hid, handle) = builder.build();
    (hid, handle)
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mut args = Args::parse();

    let cfg = if let Ok(f) = File::open(&args.config_path) {
        // Parse config with serde
        match serde_yaml::from_reader::<_, <BarpiConfig as ClapSerde>::Opt>(BufReader::new(f)) {
            // merge config already parsed from clap
            Ok(config) => BarpiConfig::from(config).merge(&mut args.config),
            Err(err) => panic!("Error in configuration file:\n{}", err),
        }
    } else {
        // If there is not config file return only config parsed from clap
        BarpiConfig::from(&mut args.config)
    };

    usb_gadget::remove_all().expect("cannot remove all gadgets");

    let (keyboard, keyboard_func) = get_hid_func(ReportType::Keyboard);
    let (mouse, mouse_func) = get_hid_func(ReportType::Mouse);
    let (consumer, consumer_func) = get_hid_func(ReportType::Consumer);

    let reg = reg(vec![keyboard_func, mouse_func, consumer_func], &cfg);

    debug!(
        "HID keyboard device {:?} at {}",
        keyboard.device().unwrap(),
        keyboard.status().path().unwrap().display()
    );
    let keyboard_path = get_dev_for_hid(&keyboard).unwrap();
    debug!("Dev file at {:?}", keyboard_path);

    debug!(
        "HID mouse device {:?} at {}",
        mouse.device().unwrap(),
        mouse.status().path().unwrap().display()
    );
    let mouse_path = get_dev_for_hid(&mouse).unwrap();
    debug!("Dev file at {:?}", mouse_path);

    debug!(
        "HID consumer control device {:?} at {}",
        consumer.device().unwrap(),
        consumer.status().path().unwrap().display()
    );
    let consumer_path = get_dev_for_hid(&consumer).unwrap();
    debug!("Dev file at {:?}", consumer_path);

    let fk = std::fs::File::create(keyboard_path).unwrap();
    let fm = std::fs::File::create(mouse_path).unwrap();
    let fc = std::fs::File::create(consumer_path).unwrap();

    let token = CancellationToken::new();

    let cloned_token: CancellationToken = token.clone();
    let mut client = client::DummyActuator::new(
        cfg.screen_width,
        cfg.screen_width,
        cfg.flip_mouse_wheel,
        fk,
        fm,
        fc,
        cloned_token,
    );

    let main_task = async move {
        loop {
            match start(&cfg.server, &cfg.screen_name, &mut client).await {
                Ok(_) => {}
                Err(e) => {
                    warn!("Error: {:?}", e);
                    sleep(Duration::from_secs(1));
                }
            }
        }
    };

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
    unreg(reg).unwrap();
}
