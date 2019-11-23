/* TODO:
 * Refactor this whole shebang. It's gross af
 * Rotate the rows efficiently (or not at all!)
 * Dither more efficiently (Converting to RGB and back sucks)
 * Mosaic mode? (Like a bunch of different lengths at random 90 degree angles)
 * Add a CLI
 * Improve UX
 */
mod crop_bounds;
use crop_bounds::{CropBounds, CropInfo};

const IN_PER_FT: f32 = 12.0;
const MM_PER_IN: f32 = 25.4;
const PX_PER_FT: f32 = pos58_usb::DOTS_PER_MM as f32 * MM_PER_IN * IN_PER_FT;
const PRINTABLE_WIDTH_PX: u32 = pos58_usb::DOTS_PER_MM * pos58_usb::PRINTABLE_WIDTH_MM;
const PAPER_WIDTH_PX: u32 = pos58_usb::DOTS_PER_MM * pos58_usb::PAPER_WIDTH_MM;

use dither::color::*;
use dither::ditherer::*;
use escposify::printer::Printer;
use failure::{format_err, Fallible};
use image::{imageops::resize, DynamicImage, FilterType, GenericImage, GenericImageView, RgbImage};
use pos58_usb::POS58USB;

fn print_usage_and_exit() -> ! {
    eprintln!("Args: <image path> <actual width>");
    std::process::exit(-1);
}

fn stdin_char() -> Fallible<char> {
    let mut string = String::new();
    std::io::stdin().read_line(&mut string)?;
    string
        .chars()
        .nth(0)
        .ok_or_else(|| format_err!("Expected input character"))
}

fn main() -> Fallible<()> {
    let mut args = std::env::args().skip(1);

    let image_file_path = match args.next() {
        Some(p) => p,
        None => print_usage_and_exit(),
    };

    let width_ft: f32 = match args.next() {
        Some(p) => p.parse()?,
        None => print_usage_and_exit(),
    };

    let image: DynamicImage = image::open(image_file_path)?;
    //let image = image.to_luma();
    let image = image.to_rgb();

    let (image_width, image_height) = image.dimensions();

    let bounds = CropBounds::new(
        image_width,
        image_height,
        width_ft,
        PX_PER_FT,
        PAPER_WIDTH_PX,
        PRINTABLE_WIDTH_PX,
        500,
    );

    let mut usb_context = libusb::Context::new()?;

    let mut device = POS58USB::new(&mut usb_context, std::time::Duration::from_secs(90))?;

    let mut printer = Printer::new(&mut device, None, None);

    let mut print_crop = |crop: CropInfo| -> Fallible<()> {
        // Crop the last row if it doesn't quite fit.
        let image_view = if let Some(whitespace) = crop.whitespace_height {
            let crop_view = image.view(crop.x, crop.y, crop.width, crop.height);
            let mut blank_space =
                RgbImage::from_pixel(crop.width, crop.height + whitespace, image::Rgb([255, 255, 255]));
            blank_space.copy_from(&crop_view, 0, 0);
            blank_space
        } else {
            image.view(crop.x, crop.y, crop.width, crop.height).to_image()
        };

        let image_upscaled = resize(
            &image_view,
            crop.upscale_width,
            crop.upscale_height,
            FilterType::Triangle,
        );

        //image_upscaled.save(format!("{}-{}.jpg", crop.x, crop.y))?;

        // Convert the image to a ditherable format
        let ditherable_image = dither::prelude::Img::<RGB<u8>>::new(
            image_upscaled.pixels().map(|p| RGB::from(p.data)),
            image_upscaled.width(),
        )
        .ok_or_else(|| format_err!("Failed to convert to ditherable image"))?
        .convert_with(|rgb| rgb.convert_with(f64::from));

        // Dither the image using the SIERRA 3 algorithm
        let dithering_palette = [RGB::from((255, 255, 255)), RGB::from((0, 0, 0))];
        let dithered = SIERRA_3
            .dither(ditherable_image, palette::quantize(&dithering_palette))
            .convert_with(|rgb| rgb.convert_with(dither::prelude::clamp_f64_to_u8));

        // Switch the image back to a consumable format for the printer
        let converted_dither =
            RgbImage::from_raw(dithered.width(), dithered.height(), dithered.raw_buf())
                .ok_or_else(|| format_err!("Failed to convert dithered image to buffer"))?;

        //converted_dither.save(format!("{}.jpg", crop_top))?;

        let dyn_image = DynamicImage::ImageRgb8(converted_dither).rotate90();
        let printer_image = escposify::img::Image::from(dyn_image);

        // Print the image
        printer.align("lt")?;
        printer.bit_image(&printer_image, None)?;
        printer.flush()?;

        if crop.is_strip_end {
            println!("Cut!");
            let _ = stdin_char()?;
        }

        Ok(())
    };

    for bound in bounds {
        print_crop(bound)?;
    }

    /*
    let mut virtual_row_beginning = 0u32;
    loop {
        println!("Row: {}", virtual_row_beginning);
        println!("'r' = repeat last row, 's' = skip row, 'q' = quit, else print row");
        match stdin_char()? {
            'r' => print_row(virtual_row_beginning)?,
            's' => {
                virtual_row_beginning += PAPER_WIDTH_PX;
            }
            'q' => break,
            _ => {
                print_row(virtual_row_beginning)?;
                virtual_row_beginning += PAPER_WIDTH_PX;
            }
        }

        if virtual_row_beginning > virtual_height {
            break;
        }
    }
    */

    Ok(())
}
