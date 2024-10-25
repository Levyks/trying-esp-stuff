use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::{BinaryColor};
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, LineHeight, Text, TextStyleBuilder};
use embedded_graphics::Drawable;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::*;
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::FromValueType;
use ssd1315::interface::I2cDisplayInterface;
use ssd1315::*;
use std::thread;

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

    log::info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(tokio_main())?;

    Ok(())
}

async fn tokio_main() -> anyhow::Result<()> {
    log::info!("Setting up board...");
    let peripherals = Peripherals::take()?;

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let green_led = PinDriver::output(peripherals.pins.gpio12)?;
    let red_led = PinDriver::output(peripherals.pins.gpio13)?;

    thread::spawn(|| {
        if let Err(e) = display_loop(i2c) {
            log::error!("Display loop failed: {:?}", e);
        }
    });
    tokio::spawn(blink_led_loop(green_led, red_led)).await??;

    Ok(())
}

fn display_loop(i2c: I2cDriver) -> anyhow::Result<()> {
    log::info!("Starting display loop");

    let interface = I2cDisplayInterface::new_interface(i2c);
    let mut display = Ssd1315::new(interface);
    let config = config::Ssd1315DisplayConfig::preset_config();
    display.set_custom_config(config);
    display.init_screen();

    let character_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
    let text_style = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .line_height(LineHeight::Percent(150))
        .build();

    let messages = vec!["HELLO\nWORLD", "FROM", "RUST", ":)"];

    let texts = messages.iter().map(|message| {
        Text::with_text_style(message, Point::new(64, 22), character_style, text_style)
    }).collect::<Vec<_>>();

    loop {
        for text in &texts {
            display.clear(BinaryColor::Off)?;
            text.draw(&mut display)?;
            display.flush_screen();
            FreeRtos::delay_ms(1000);
        }
    }
}

async fn blink_led_loop<'d, G: Pin, R: Pin>(
    mut green_led: PinDriver<'d, G, Output>,
    mut red_led: PinDriver<'d, R, Output>,
) -> anyhow::Result<()> {
    let one_sec = tokio::time::Duration::from_secs(1);
    let mut interval = tokio::time::interval(one_sec);
    loop {
        log::info!("Changing to red");
        green_led.set_low()?;
        red_led.set_high()?;
        interval.tick().await;
        log::info!("Changing to green");
        green_led.set_high()?;
        red_led.set_low()?;
        interval.tick().await;
        log::info!("Keeping both on for a bit");
        red_led.set_high()?;
        interval.tick().await;
    }
}
