use std::{time::Duration, thread::sleep};

use usb_gadget::{function::Handle, RegGadget, default_udc, Gadget, Class, Config, Id, Strings};


pub fn reg(func: Handle) -> RegGadget {
    let udc = default_udc().expect("cannot get UDC");

    let reg =
        Gadget::new(Class::new(1, 2, 3), Id::new(4, 5), Strings::new("manufacturer", "product", "serial_number"))
            .with_config(Config::new("config").with_function(func))
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

fn main() {
    println!("Hello, world!");
}
