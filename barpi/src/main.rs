use std::{env, os::linux::fs::MetadataExt, path::PathBuf, thread::sleep, time::Duration};

use barrier_client::start;
use env_logger::Env;
use log::{warn, info};
use synergy_hid::{ReportType, SynergyHid};
use tokio::{select, signal::unix::{signal, SignalKind}};
use tokio_util::sync::CancellationToken;
use usb_gadget::{
    default_udc,
    function::{hid::Hid, Handle},
    Class, Config, Gadget, Id, RegGadget, Strings,
};

mod client;

pub fn reg(funcs: Vec<Handle>) -> RegGadget {
    let udc = default_udc().expect("cannot get UDC");

    let mut config = Config::new("config");
    for func in funcs {
        config = config.with_function(func);
    }

    let reg = Gadget::new(
        Class::new(1, 2, 3),
        Id::new(4, 5),
        Strings::new("manufacturer", "product", "serial_number"),
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

    let (keyboard, keyboard_func) = get_hid_func(ReportType::Keyboard);
    let (mouse, mouse_func) = get_hid_func(ReportType::Mouse);
    let (consumer, consumer_func) = get_hid_func(ReportType::Consumer);

    let reg = reg(vec![keyboard_func, mouse_func, consumer_func]);

    println!(
        "HID keyboard device {:?} at {}",
        keyboard.device().unwrap(),
        keyboard.status().path().unwrap().display()
    );
    let keyboard_path = get_dev_for_hid(&keyboard).unwrap();
    println!("Dev file at {:?}", keyboard_path);

    println!(
        "HID mouse device {:?} at {}",
        mouse.device().unwrap(),
        mouse.status().path().unwrap().display()
    );
    let mouse_path = get_dev_for_hid(&mouse).unwrap();
    println!("Dev file at {:?}", mouse_path);

    println!(
        "HID consumer control device {:?} at {}",
        consumer.device().unwrap(),
        consumer.status().path().unwrap().display()
    );
    let consumer_path = get_dev_for_hid(&consumer).unwrap();
    println!("Dev file at {:?}", consumer_path);

    let fk = std::fs::File::create(keyboard_path).unwrap();
    let fm = std::fs::File::create(mouse_path).unwrap();
    let fc = std::fs::File::create(consumer_path).unwrap();
    let mut client = client::DummyActuator::new(1920, 1080, false, fk, fm, fc);

    let token = CancellationToken::new();
    let cloned_token = token.clone();

    let main_task = async move {
        loop {
            match start("192.168.2.59:24800", String::from("BARPI"), &mut client).await {
                Ok(_) => {}
                Err(e) => {
                    warn!("Error: {:?}", e);
                    sleep(Duration::from_secs(1));
                }
            }
        }
    };

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
