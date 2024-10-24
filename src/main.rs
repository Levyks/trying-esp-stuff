use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::*;
use esp_idf_svc::hal::peripherals::Peripherals;


fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // eventfd is needed by our mio poll implementation.  Note you should set max_fds
    // higher if you have other code that may need eventfd.
    log::info!("Setting up eventfd...");
    let config = esp_idf_svc::sys::esp_vfs_eventfd_config_t {
        max_fds: 1,
        ..Default::default()
    };
    esp_idf_svc::sys::esp! { unsafe { esp_idf_svc::sys::esp_vfs_eventfd_register(&config) } }?;

    log::info!("Setting up board...");
    let peripherals = Peripherals::take()?;
    let led = PinDriver::output(peripherals.pins.gpio2)?;

    log::info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            tokio::spawn(blink_led_loop(led)).await?
        })?;

    Ok(())
}

async fn blink_led_loop<'d, T: Pin>(mut led: PinDriver<'d, T, Output>, ) -> anyhow::Result<()> {
    loop {
        log::info!("Setting LED high");
        led.set_high()?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        log::info!("Setting LED low");
        led.set_low()?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
