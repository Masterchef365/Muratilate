pub struct CropBounds {
    image_width: u32,
    image_height: u32,
    virtual_width: u32,
    virtual_height: u32,
    paper_width_px: u32,
    printable_width_px: u32,
    max_strip_len_px: u32,
}

impl CropBounds {
    /// # Arguments
    ///
    /// * `image_width` - Input image width
    /// * `image_height` - Input image height
    /// * `physical_width_ft` - Physical width of output in feet
    /// * `pixels_per_ft` - Pixels per foot of the printer
    /// * `paper_width_px` - Width of paper, in px
    /// * `printable_width_px` - Printable width of paper, in px
    /// * `max_strip_len_px` - Maximal length for any strip
    ///
    pub fn new(
        image_width: u32,
        image_height: u32,
        physical_width_ft: f32,
        pixels_per_ft: f32,
        paper_width_px: u32,
        printable_width_px: u32,
        max_strip_len_px: u32, // TODO: Make this an Option<>?
    ) -> Self {
        let virtual_width = (pixels_per_ft * physical_width_ft as f32) as u32;
        let virtual_height = (virtual_width * image_height as u32) / image_width as u32;
        Self {
            image_width,
            image_height,
            virtual_width,
            virtual_height,
            paper_width_px,
            printable_width_px,
            max_strip_len_px,
        }
    }

    /*
    pub fn used_paper_len_ft() -> f32 {
        unimplemented!()
    }

    pub fn physical_height_ft() -> f32 {
        unimplemented!()
    }
    */
}

#[derive(Debug, Clone, Copy)]
pub struct CropInfo {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub upscale_width: u32,
    pub upscale_height: u32,
    pub whitespace_height: Option<u32>,
    pub is_strip_end: bool,
}

impl IntoIterator for CropBounds {
    type Item = CropInfo;
    type IntoIter = CropBoundsIterator;
    fn into_iter(self) -> Self::IntoIter {
        CropBoundsIterator {
            bounds: self,
            virtual_x: 0,
            virtual_y: 0,
            finished: false,
        }
    }
}

pub struct CropBoundsIterator {
    bounds: CropBounds,
    virtual_x: u32,
    virtual_y: u32,
    finished: bool,
}

impl Iterator for CropBoundsIterator {
    type Item = CropInfo;
    fn next(&mut self) -> Option<Self::Item> {
        let b = &self.bounds;

        if self.finished {
            return None;
        }

        let crop_x = b.image_width * self.virtual_x / b.virtual_width;
        let crop_y = b.image_height * self.virtual_y / b.virtual_height;
        let crop_width = b.image_width * b.max_strip_len_px / b.virtual_width;
        let crop_height = b.image_height * b.printable_width_px / b.virtual_height;

        let crop_width = if crop_x + crop_width > b.image_width {
            b.image_width - crop_x
        } else {
            crop_width
        };

        let (whitespace_height, crop_height) = if crop_y + crop_height > b.image_height {
            (Some(crop_height), b.image_height - crop_y)
        } else {
            (None, crop_height)
        };

        let mut crop_info = CropInfo {
            x: crop_x,
            y: crop_y,
            width: crop_width,
            height: crop_height,
            upscale_width: if self.virtual_x + b.max_strip_len_px > b.virtual_width {
                b.virtual_width - self.virtual_x
            } else {
                b.max_strip_len_px
            },
            whitespace_height,
            upscale_height: b.printable_width_px,
            is_strip_end: false,
        };

        self.virtual_x += b.max_strip_len_px;

        if self.virtual_x >= b.virtual_width {
            self.virtual_x = 0;
            self.virtual_y += b.paper_width_px;
            crop_info.is_strip_end = true;
            if self.virtual_y >= b.virtual_height + crop_height {
                self.finished = true;
            }
        }

        Some(crop_info)
    }
}
