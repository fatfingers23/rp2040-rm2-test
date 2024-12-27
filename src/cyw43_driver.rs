use cyw43::Control;
use cyw43_pio::PioSpi;
use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_net_wiznet::Device;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_2, PIN_3, PIN_4, PIN_5, PIO0};
use embassy_rp::peripherals::{PIN_23, PIN_24, PIN_25, PIN_29};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

pub async fn setup_cyw43<'a>(
    pio0: PIO0,
    wl_on: PIN_2,
    data_out_in: PIN_4,
    cs: PIN_3,
    wl_clk: PIN_5,
    dma_ch0: DMA_CH0,
    spawner: Spawner,
) -> (Device<'a>, Control<'a>) {
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
    //Un-comment below line for bluetooth
    // let btfw = include_bytes!("../cyw43-firmware/43439A0_btfw.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x101b0000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x101f8000
    //     probe-rs download 43439A0_btfw.bin --format bin --chip RP2040 --base-address 0x10141400
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 224190) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };
    //let btfw = unsafe { core::slice::from_raw_parts(0x10141400 as *const u8, 6164) };

    let pwr = Output::new(wl_on, Level::Low);
    let cs = Output::new(cs, Level::High);
    let mut pio = Pio::new(pio0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        data_out_in,
        wl_clk,
        dma_ch0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    info!("Starting CYW43");
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    info!("CYW43 started");
    //Un comment the below new_with_bluetooth line for bluetooth
    // let (net_device, _bt_device, mut control, runner) =
    //     cyw43::new_with_bluetooth(state, pwr, spi, fw, btfw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    (net_device, control)
}
