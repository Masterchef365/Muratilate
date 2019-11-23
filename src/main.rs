mod pos58;
use escposify::printer::Printer;
use failure::Fail;
use failure::{format_err, Fallible};
use image::{DynamicImage, ImageBuffer};
use pos58::POS58USB;
use std::fs::File;
use std::io::BufRead;

fn print_usage_and_exit() -> ! {
    eprintln!("Args: <image path> <print width (ft)> Option<start row>");
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

const IN_PER_FT: f32 = 12.0;
const MM_PER_IN: f32 = 25.4;

fn main() -> Fallible<()> {
    // Parse args
    let mut args = std::env::args().skip(1);

    let image_file_path = match args.next() {
        Some(p) => p,
        None => print_usage_and_exit(),
    };

    let physical_width = match args.next() {
        Some(w) => w,
        None => print_usage_and_exit(),
    };

    let start_row = args.next();

    let physical_width: f32 = physical_width.parse()?;

    let image: DynamicImage = image::open(image_file_path)?;
    let image = image.to_luma(); // Convert to grayscale

    // Prompt if the height is ok
    let (image_width, image_height) = image.dimensions();
    println!("Image dimensions: {}x{}", image_width, image_height);
    let physical_height = (physical_width * image_height as f32) / image_width as f32;
    println!(
        "Image height is calculated to be {} ft. Is this acceptable? [Y/n]",
        physical_height
    );
    /*if stdin_char()? == 'n' {
        return Ok(());
    } */

    // The rest of the calculations are in pixels
    let virtual_width = (physical_width * IN_PER_FT * MM_PER_IN) as u32;
    for y_v in (0..virtual_height).step_by(pos58::PAPER_WIDTH_MM as usize) {
    }


    Ok(())
}

/*
let mut usb_context = libusb::Context::new().expect("Failed to create LibUSB context.");

let mut device = POS58USB::new(&mut usb_context, std::time::Duration::from_secs(90))
    .expect("Failed to connect to printer");

let mut printer = Printer::new(&mut device, None, None);
*/

/*
 * Interactive: display the expected width and height...
 */

/*
let mut bitch = vec![0u8; (DOTS_PER_WIDTH * DOTS_PER_WIDTH) as usize];
for (idx, pxl) in bitch.iter_mut().enumerate() {
    if (idx & 1) == 0 {
        *pxl = 255;
    }
}

let converted = ImageBuffer::from_raw(DOTS_PER_WIDTH, DOTS_PER_WIDTH, bitch)
    .ok_or("Failed to convert dithered image to buffer")?;
let printer_image = escposify::img::Image::from(DynamicImage::ImageLuma8(converted));

for _ in 0..2 {
    printer.bit_image(&printer_image, None)?;
}
*/
