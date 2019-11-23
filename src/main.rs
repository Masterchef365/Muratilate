mod pos58;
use dither::color::*;
use dither::ditherer::*;
use escposify::printer::Printer;
use failure::{format_err, Fallible};
use image::{imageops::resize, DynamicImage, FilterType, GenericImage, GenericImageView, RgbImage};
use pos58::POS58USB;

const IN_PER_FT: f32 = 12.0;
const MM_PER_IN: f32 = 25.4;
const PX_PER_FT: f32 = pos58::DOTS_PER_MM as f32 * MM_PER_IN * IN_PER_FT;
const PRINTABLE_WIDTH_PX: u32 = pos58::DOTS_PER_MM * pos58::PRINTABLE_WIDTH_MM;
const PAPER_WIDTH_PX: u32 = pos58::DOTS_PER_MM * pos58::PAPER_WIDTH_MM;

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
        .ok_or(format_err!("Expected input character"))
        .into()
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
    let virtual_width = (PX_PER_FT * width_ft as f32) as u32;
    let virtual_height = (virtual_width * image_height as u32) / image_width as u32;

    println!("Input dimensions: {}x{}", image_width, image_height);
    println!("Pixel dimensions: {}x{}", virtual_width, virtual_height);

    let mut usb_context = libusb::Context::new().expect("Failed to create LibUSB context.");

    let mut device = POS58USB::new(&mut usb_context, std::time::Duration::from_secs(90))
        .expect("Failed to connect to printer");

    let mut printer = Printer::new(&mut device, None, None);

    let mut print_row = |row_beginning| -> Fallible<()> {
        println!("Printing from virtual row: {}", row_beginning);

        let image_begin = image_height * row_beginning / virtual_height;
        let image_crop_height = image_height * PRINTABLE_WIDTH_PX / virtual_height;

        // Crop the last row if it doesn't quite fit.
        let image_view = if image_begin + image_crop_height <= image_height {
            image
                .view(0, image_begin, image_width, image_crop_height)
                .to_image()
        } else {
            let crop_view = image.view(0, image_begin, image_width, image_height - image_begin);
            let mut rest_of_view =
                RgbImage::from_pixel(image_width, image_crop_height, image::Rgb([255, 255, 255]));
            rest_of_view.copy_from(&crop_view, 0, 0);
            rest_of_view
        };

        let image_upscaled = resize(
            &image_view,
            virtual_width,
            PRINTABLE_WIDTH_PX,
            FilterType::Triangle,
        );

        // Convert the image to a ditherable format
        let ditherable_image = dither::prelude::Img::<RGB<u8>>::new(
            image_upscaled.pixels().map(|p| RGB::from(p.data)),
            image_upscaled.width(),
        )
        .ok_or(format_err!("Failed to convert to ditherable image"))?
        .convert_with(|rgb| rgb.convert_with(f64::from));

        // Dither the image using the SIERRA 3 algorithm
        let dithering_palette = [RGB::from((255, 255, 255)), RGB::from((0, 0, 0))];
        let dithered = SIERRA_3
            .dither(ditherable_image, palette::quantize(&dithering_palette))
            .convert_with(|rgb| rgb.convert_with(dither::prelude::clamp_f64_to_u8));

        // Switch the image back to a consumable format for the printer
        let converted_dither =
            RgbImage::from_raw(dithered.width(), dithered.height(), dithered.raw_buf())
                .ok_or(format_err!("Failed to convert dithered image to buffer"))?;

        //converted_dither.save(format!("{}.jpg", image_begin))?;

        let dyn_image = DynamicImage::ImageRgb8(converted_dither).rotate90();
        let printer_image = escposify::img::Image::from(dyn_image);

        // Print the image
        printer.align("lt")?;
        printer.bit_image(&printer_image, None)?;
        printer.flush()?;

        Ok(())
    };


    let mut row_beginning = 0u32;
    loop {
        println!("Row: {}", row_beginning);
        println!("'r' = repeat last row, 's' = skip row, 'q' = quit, else print row");
        match stdin_char()? {
            'r' => print_row(row_beginning)?,
            's' => {
                row_beginning += PAPER_WIDTH_PX;
            }
            'q' => break,
            _ => {
                print_row(row_beginning)?;
                row_beginning += PAPER_WIDTH_PX;
            },
        }

        if row_beginning > virtual_height {
            break;
        }
    }

    Ok(())
}
