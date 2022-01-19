//! A simple Driver for the Waveshare 7.5" E-Ink Display (V3) via SPI
//!
//! # References
//!
//! - [Datasheet](https://www.waveshare.com/wiki/7.5inch_e-Paper_HAT)
//! - [Waveshare C driver](https://github.com/waveshare/e-Paper/blob/702def0/RaspberryPi%26JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c)
//! - [Waveshare Python driver](https://github.com/waveshare/e-Paper/blob/702def0/RaspberryPi%26JetsonNano/python/lib/waveshare_epd/epd7in5_V2.py)
//!
//! Important note for V3:
//! Revision V3 has been released on 2019.11, the resolution is upgraded to 800×480, from 640×384 of V1.
//! The hardware and interface of V2 are compatible with V1, however, the related software should be updated.

use embedded_hal::{
    blocking::{delay::*, spi::Write},
    digital::v2::{InputPin, OutputPin},
};

use crate::color::TriColor;
use crate::interface::DisplayInterface;
use crate::prelude::{WaveshareThreeColorDisplay};
use crate::traits::{InternalWiAdditions, RefreshLut, WaveshareDisplay};

use crate::command_v3::Command;

#[cfg(feature = "graphics")]
mod graphics;
#[cfg(feature = "graphics")]
pub use self::graphics::Display7in5bc;

/// Width of epd7in5bc_v3 in pixels
pub const WIDTH: u32 = 800;
/// Height of epd7in5bc_v3 in pixels
pub const HEIGHT: u32 = 480;
/// Default background color (white) of epd7in5bc_v3 display
pub const DEFAULT_BACKGROUND_COLOR: TriColor = TriColor::White;
const IS_BUSY_LOW: bool = true;

/// Number of bits for b/w buffer and same for chromatic buffer
const NUM_DISPLAY_BITS: u32 = WIDTH * HEIGHT / 8;


/// Epd7in5bc (V3) driver
///
pub struct Epd7in5bc<SPI, CS, BUSY, DC, RST, DELAY> {
    // Connection Interface
    interface: DisplayInterface<SPI, CS, BUSY, DC, RST, DELAY>,
    color: TriColor,
}

impl<SPI, CS, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd7in5bc<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // Reset the device
        self.interface.reset(delay, 2);

        self.command_with_data(spi, Command::PowerSetting, &[0x07, 0x07, 0x3F, 0x3F])?;
        self.command(spi, Command::PowerOn)?;

        self.wait_until_idle(spi, delay)?;

        self.command_with_data(spi, Command::PanelSetting, &[0x0F])?;
        self.command_with_data(spi, Command::TconResolution, &[0x03, 0x20, 0x01, 0xE0])?;

        self.command_with_data(spi, Command::DualSpi, &[0x00])?;

        self.command_with_data(spi, Command::VcomAndDataIntervalSetting, &[0x11, 0x07])?;

        self.command_with_data(spi, Command::TconSetting, &[0x22])?;
        self.command_with_data(spi, Command::GateSourceStart, &[0x00, 0x00, 0x00, 0x00])?;

        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> WaveshareThreeColorDisplay<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd7in5bc<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    fn update_color_frame(
        &mut self,
        spi: &mut SPI,
        black: &[u8],
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.update_achromatic_frame(spi, black)?;
        self.update_chromatic_frame(spi, chromatic)
    }

    fn update_achromatic_frame(&mut self, spi: &mut SPI, black: &[u8]) -> Result<(), SPI::Error> {
        self.command_with_data(spi, Command::DataStartTransmissionBlackWhite, black)
    }

    fn update_chromatic_frame(
        &mut self,
        spi: &mut SPI,
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.command_with_data(spi, Command::DataStartTransmissionChromatic, chromatic)
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, CS, BUSY, DC, RST, DELAY>
    for Epd7in5bc<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    type DisplayColor = TriColor;

    fn new(
        spi: &mut SPI,
        cs: CS,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
    ) -> Result<Self, SPI::Error> {
        let interface = DisplayInterface::new(cs, busy, dc, rst);
        let color = DEFAULT_BACKGROUND_COLOR;

        let mut epd =  Epd7in5bc { interface, color };

        epd.init(spi, delay)?;

        Ok(epd)
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn sleep(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // self.wait_until_idle(spi, delay)?;
        // self.command_with_data(spi, Command::VcomAndDataIntervalSetting, &[0xF7])?;

        self.command(spi, Command::PowerOff)?;
        self.wait_until_idle(spi, delay)?;

        self.command_with_data(spi, Command::DeepSleep, &[0xA5])?;
        Ok(())
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.command_with_data(spi, Command::DataStartTransmissionBlackWhite, buffer)?;

        self.command(spi, Command::DataStartTransmissionChromatic)?;
        self.interface.data_x_times(spi, 0x00, WIDTH * HEIGHT / 8)?;

        self.wait_until_idle(spi, delay)?;
        Ok(())
    }

    #[allow(unused)]
    fn update_partial_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        unimplemented!()
    }

    fn display_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.command(spi, Command::DisplayRefresh)?;
        delay.delay_ms(100); // The delay here is necessary, 200uS at least!!!
        self.wait_until_idle(spi, delay)?;
        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        self.display_frame(spi, delay)?;
        Ok(())
    }

    fn clear_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;

        self.command(spi, Command::DataStartTransmissionBlackWhite)?;
        self.interface.data_x_times(spi, 0xFF, WIDTH * HEIGHT / 8)?;

        self.command(spi, Command::DataStartTransmissionChromatic)?;
        self.interface.data_x_times(spi, 0x00, WIDTH * HEIGHT / 8)?;

        self.command(spi, Command::DisplayRefresh)?;
        Ok(())
    }

    fn set_background_color(&mut self, color: TriColor) {
        self.color = color;
    }

    fn background_color(&self) -> &TriColor {
        &self.color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn set_lut(
        &mut self,
        _spi: &mut SPI,
        _refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        unimplemented!()
    }

    fn is_busy(&self) -> bool {
        self.interface.is_busy(IS_BUSY_LOW)
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> Epd7in5bc<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    fn command(&mut self, spi: &mut SPI, command: Command) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, command)
    }

    fn send_data(&mut self, spi: &mut SPI, data: &[u8]) -> Result<(), SPI::Error> {
        self.interface.data(spi, data)
    }

    fn command_with_data(
        &mut self,
        spi: &mut SPI,
        command: Command,
        data: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd_with_data(spi, command, data)
    }

    fn wait_until_idle(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        while self.interface.is_busy(IS_BUSY_LOW) {
            self.interface.cmd(spi, Command::Revision)?;
            delay.delay_ms(20);
        }
        Ok(())
    }
}
