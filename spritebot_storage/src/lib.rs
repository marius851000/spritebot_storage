use std::io::{BufReader, Cursor};

use animdata_xml::AnimsXML;
use image::{GenericImage, GenericImageView, ImageBuffer, ImageFormat, ImageOutputFormat, Rgba};

mod error;
pub use error::SpriteBotStorageError;

mod animdata_xml;
pub use animdata_xml::AnimDataXML;

use crate::animdata_xml::{AnimXML, DurationsXML};

type RgbaU8 = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
pub struct Sprite {
    pub shadow_size: u8,
    pub animations: Vec<Animation>,
}

fn get_number_of_component_on_axis(
    size: u32,
    divider: u32,
    side_name: &str,
    anim_name: &str,
    image_kind: &str,
) -> Result<u32, SpriteBotStorageError> {
    if size.checked_rem(divider as u32).ok_or_else(|| {
        SpriteBotStorageError::SpriteSizeZero(
            anim_name.to_string(),
            image_kind.to_string(),
            side_name.into(),
        )
    })? != 0
    {
        return Err(SpriteBotStorageError::SpriteSizeNotMultiple(
            anim_name.to_string(),
            image_kind.to_string(),
            side_name.into(),
        ));
    }
    return Ok(size / divider);
}

/**
 * Read the image for the animation of said type (Anim, Offsets or Shadow) in the VFS, and split it up based on the height and width.
 *
 * The first Vec is for the column, the second is for the row. The row Vecs will always have the same size between them.
 */
fn get_image<T: vfs::FileSystem>(
    vfs: &T,
    name: &str,
    kind: &str,
    frame_height: u32,
    frame_width: u32,
) -> Result<Vec<Vec<RgbaU8>>, SpriteBotStorageError> {
    let path = format!("/{}-{}.png", name, kind);
    let image_file = vfs
        .open_file(&path)
        .map_err(|err| SpriteBotStorageError::VfsError(err, path.to_string()))?;

    let img = image::io::Reader::with_format(BufReader::new(image_file), ImageFormat::Png)
        .decode()
        .map_err(|err| {
            SpriteBotStorageError::ErrorImageRead(kind.to_string(), name.to_string(), err)
        })?
        .to_rgba8();

    let nb_on_width =
        get_number_of_component_on_axis(img.width(), frame_width, "width", name, kind)?;
    let nb_on_height =
        get_number_of_component_on_axis(img.height(), frame_height, "height", name, kind)?;

    let mut result = Vec::new();
    for column in 0..nb_on_height {
        let mut column_result = Vec::new();
        let column_start = column * frame_height;
        for row in 0..nb_on_width {
            let row_start = row * frame_width;
            column_result.push(
                img.view(row_start, column_start, frame_width, frame_height)
                    .to_image(),
            );
        }
        result.push(column_result);
    }
    Ok(result)
}

impl Sprite {
    pub fn new_empty(shadow_size: u8) -> Self {
        Self {
            shadow_size,
            animations: Vec::new(),
        }
    }

    /// Read the sprite contained at the root of the given virtual file system
    pub fn new<T: vfs::FileSystem>(vfs: &T) -> Result<Self, SpriteBotStorageError> {
        let animdata_xml_file = vfs
            .open_file("/AnimData.xml")
            .map_err(|err| SpriteBotStorageError::VfsError(err, "/AnimData.xml".to_string()))?;
        let animdata_xml: AnimDataXML =
            quick_xml::de::from_reader(BufReader::new(animdata_xml_file))?;
        let mut animations = Vec::new();

        for anim_source in &animdata_xml.anims.anim {
            let mut images = Vec::new();

            let anim_image = get_image(
                vfs,
                &anim_source.name,
                "Anim",
                anim_source.frame_height,
                anim_source.frame_width,
            )?;
            let shadow_image = get_image(
                vfs,
                &anim_source.name,
                "Shadow",
                anim_source.frame_height,
                anim_source.frame_width,
            )?;
            let offset_image = get_image(
                vfs,
                &anim_source.name,
                "Offsets",
                anim_source.frame_height,
                anim_source.frame_width,
            )?;

            if anim_image.len() != shadow_image.len() || shadow_image.len() != offset_image.len() {
                return Err(SpriteBotStorageError::SpriteSizeNotIdentical(
                    anim_source.name.clone(),
                ));
            }

            for (column_nb, ((column_anims, column_shadows), column_offsets)) in anim_image
                .into_iter()
                .zip(&shadow_image)
                .zip(&offset_image)
                .enumerate()
            {
                if column_anims.len() != column_shadows.len()
                    || column_shadows.len() != column_offsets.len()
                {
                    return Err(SpriteBotStorageError::SpriteSizeNotIdentical(
                        anim_source.name.clone(),
                    ));
                };

                if column_anims.len() != anim_source.durations.duration.len() {
                    return Err(SpriteBotStorageError::InconsistantDuration(
                        anim_source.name.clone(),
                        column_anims.len(),
                        anim_source.durations.duration.len(),
                    ));
                }

                let mut frames = Vec::new();
                for (
                    row_nb,
                    (((anim_local_image, shadow_local_image), offset_local_image), duration),
                ) in column_anims
                    .into_iter()
                    .zip(column_shadows)
                    .zip(column_offsets)
                    .zip(anim_source.durations.duration.iter().copied())
                    .enumerate()
                {
                    let offsets = FrameOffset::from_images(
                        &offset_local_image,
                        &shadow_local_image,
                        column_nb,
                        row_nb,
                        &anim_source.name,
                    )?;

                    frames.push(Frame {
                        image: anim_local_image,
                        offsets,
                        duration: duration
                            .try_into()
                            .map_err(SpriteBotStorageError::TooLargeDuration)?,
                    })
                }

                images.push(frames);
            }

            animations.push(Animation {
                name: anim_source.name.clone(),
                index: anim_source.index,
                rush_frame: anim_source.rush_frame,
                hit_frame: anim_source.hit_frame,
                return_frame: anim_source.return_frame,
                images,
            });
        }

        Ok(Self {
            shadow_size: animdata_xml.shadow_size,
            animations,
        })
    }

    pub fn write_to_folder<T: vfs::FileSystem>(
        &self,
        vfs: &mut T,
    ) -> Result<(), SpriteBotStorageError> {
        let mut animdata = AnimDataXML {
            shadow_size: self.shadow_size,
            anims: AnimsXML { anim: Vec::new() },
        };

        for animation in &self.animations {
            let (segment_size, anim_img, offset_img, shadow_img) = animation.generate_sheet()?;

            let write_image =
                |image: RgbaU8, file_name: String| -> Result<(), SpriteBotStorageError> {
                    let mut buffer = Vec::new();
                    image
                        .write_to(&mut Cursor::new(&mut buffer), ImageOutputFormat::Png)
                        .map_err(|e| {
                            SpriteBotStorageError::WriteImageError(e, file_name.to_string())
                        })?;
                    let mut anim_file = vfs
                        .create_file(&file_name)
                        .map_err(|e| SpriteBotStorageError::VfsError(e, file_name.to_string()))?;
                    anim_file
                        .write_all(&buffer)
                        .map_err(|e| SpriteBotStorageError::WriteFileError(e, file_name))?;
                    Ok(())
                };

            write_image(anim_img, format!("{}-Anim.png", animation.name))?;
            write_image(offset_img, format!("{}-Offsets.png", animation.name))?;
            write_image(shadow_img, format!("{}-Shadow.png", animation.name))?;

            animdata.anims.anim.push(AnimXML {
                name: animation.name.clone(),
                index: animation.index,
                rush_frame: animation.rush_frame,
                hit_frame: animation.hit_frame,
                return_frame: animation.return_frame,
                frame_width: segment_size.0,
                frame_height: segment_size.1,
                durations: DurationsXML {
                    duration: animation
                        .images
                        .get(0)
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|x| x.duration as usize)
                        .collect(),
                },
            })
        }

        let animdata_str = quick_xml::se::to_string(&animdata)?;
        let mut animdata_file = vfs
            .create_file("AnimData.xml")
            .map_err(|e| SpriteBotStorageError::VfsError(e, "AnimData.xml".to_string()))?;
        animdata_file
            .write_all(animdata_str.as_bytes())
            .map_err(|e| SpriteBotStorageError::WriteFileError(e, "AnimData.xml".to_string()))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Animation {
    pub name: String,
    pub index: u32,
    pub rush_frame: Option<u32>,
    pub hit_frame: Option<u32>,
    pub return_frame: Option<u32>,
    pub images: Vec<Vec<Frame>>,
}

impl Animation {
    /// The three images are 1. Anim, 2. Offsets 3. Shadow
    pub fn generate_sheet(
        &self,
    ) -> Result<((u32, u32), RgbaU8, RgbaU8, RgbaU8), SpriteBotStorageError> {
        let mut max_size = (8, 8);
        let mut max_row = 1;
        for line in &self.images {
            max_row = max_row.max(line.len());
            for row in line {
                fn max_size_offset(first: (u32, u32), offset: (u16, u16)) -> (u32, u32) {
                    (
                        first.0.max(offset.0 as u32 + 1),
                        first.1.max(offset.1 as u32 + 1),
                    )
                }
                max_size = (
                    max_size.0.max(row.image.dimensions().0),
                    max_size.1.max(row.image.dimensions().1),
                );
                max_size = max_size_offset(max_size, row.offsets.center);
                max_size = max_size_offset(max_size, row.offsets.hand_left);
                max_size = max_size_offset(max_size, row.offsets.hand_right);
                max_size = max_size_offset(max_size, row.offsets.head);
                max_size = max_size_offset(max_size, row.offsets.shadow);
            }
        }

        let image_dimension = (
            max_size.0
                * TryInto::<u32>::try_into(max_row)
                    .map_err(SpriteBotStorageError::TooLargeGeneratedSheet)?,
            max_size.1
                * TryInto::<u32>::try_into(self.images.len())
                    .map_err(SpriteBotStorageError::TooLargeGeneratedSheet)?,
        );

        let mut anim_image = RgbaU8::new(image_dimension.0, image_dimension.1);
        let mut offset_images = RgbaU8::new(image_dimension.0, image_dimension.1);
        let mut shadow_image = RgbaU8::new(image_dimension.0, image_dimension.1);

        let mut start_x = 0;
        let mut start_y = 0;
        for line in &self.images {
            for row in line {
                anim_image.copy_from(&row.image, start_x, start_y).unwrap(); // Should never fail
                shadow_image.put_pixel(
                    start_x + row.offsets.shadow.0 as u32,
                    start_y + row.offsets.shadow.1 as u32,
                    Rgba([255, 255, 255, 255]),
                );
                if row.offsets.head == row.offsets.hand_left
                    || row.offsets.head == row.offsets.hand_right
                {
                    return Err(SpriteBotStorageError::InvalidHeadPosition);
                }
                offset_images.put_pixel(
                    start_x + row.offsets.head.0 as u32,
                    start_y + row.offsets.head.1 as u32,
                    Rgba([0, 0, 0, 255]),
                );
                {
                    let center_pixel = offset_images.get_pixel_mut(
                        start_x + row.offsets.center.0 as u32,
                        start_y + row.offsets.center.1 as u32,
                    );
                    center_pixel.0[1] = 255;
                    center_pixel.0[3] = 255;
                }
                {
                    let hand_right_pixel = offset_images.get_pixel_mut(
                        start_x + row.offsets.hand_right.0 as u32,
                        start_y + row.offsets.hand_right.1 as u32,
                    );
                    hand_right_pixel[2] = 255;
                    hand_right_pixel[3] = 255;
                }
                {
                    let hand_left_pixel = offset_images.get_pixel_mut(
                        start_x + row.offsets.hand_left.0 as u32,
                        start_y + row.offsets.hand_left.1 as u32,
                    );
                    hand_left_pixel[0] = 255;
                    hand_left_pixel[3] = 255;
                }
                start_x += max_size.0;
            }
            start_x = 0;
            start_y += max_size.1;
        }

        Ok((max_size, anim_image, offset_images, shadow_image))
    }
}

#[derive(Debug)]
pub struct Frame {
    pub duration: u8,
    pub image: RgbaU8,
    pub offsets: FrameOffset,
}

#[derive(Debug)]
pub struct FrameOffset {
    pub head: (u16, u16),
    pub hand_left: (u16, u16),
    pub hand_right: (u16, u16),
    pub center: (u16, u16),
    pub shadow: (u16, u16),
}

fn find_pixel_in_image<T: Fn(&Rgba<u8>) -> bool>(
    image: &RgbaU8,
    filter: T,
    anim_name: &str,
    column_nb: usize,
    row_nb: usize,
    image_kind: &str,
    color_text: &str,
) -> Result<(u16, u16), SpriteBotStorageError> {
    let mut result = None;
    for (x, y, pixel) in image.enumerate_pixels() {
        if filter(pixel) {
            if result.is_some() {
                return Err(SpriteBotStorageError::ColorDuplicateInPixelDate(
                    anim_name.into(),
                    column_nb,
                    row_nb,
                    image_kind.into(),
                    color_text.into(),
                ));
            }
            result = Some((x, y));
        }
    }
    if let Some(r) = result {
        Ok((
            r.0.try_into()
                .map_err(|_| SpriteBotStorageError::OffsetTooLarge)?,
            r.1.try_into()
                .map_err(|_| SpriteBotStorageError::OffsetTooLarge)?,
        ))
    } else {
        Err(SpriteBotStorageError::ColorNotFoundInPixelData(
            anim_name.into(),
            column_nb,
            row_nb,
            image_kind.into(),
            color_text.into(),
        ))
    }
}

impl FrameOffset {
    pub fn from_images(
        offset_image: &RgbaU8,
        shadow_image: &RgbaU8,
        column_nb: usize,
        row_nb: usize,
        animation_name: &str,
    ) -> Result<Self, SpriteBotStorageError> {
        let black_offset = match find_pixel_in_image(
            offset_image,
            |p| p == &Rgba::from([0, 0, 0, 255]),
            animation_name,
            column_nb,
            row_nb,
            "offsets",
            "black",
        ) {
            Ok(r) => Some(r),
            Err(SpriteBotStorageError::ColorNotFoundInPixelData(_, _, _, _, _)) => None,
            Err(x) => return Err(x),
        };
        let red_offset = find_pixel_in_image(
            offset_image,
            |p| p.0[0] == 255 && p.0[3] == 255,
            animation_name,
            column_nb,
            row_nb,
            "offsets",
            "red",
        )?;
        let green_offset = find_pixel_in_image(
            offset_image,
            |p| p.0[1] == 255 && p.0[3] == 255,
            animation_name,
            column_nb,
            row_nb,
            "offsets",
            "green",
        )?;
        let blue_offset = find_pixel_in_image(
            offset_image,
            |p| p.0[2] == 255 && p.0[3] == 255,
            animation_name,
            column_nb,
            row_nb,
            "offsets",
            "blue",
        )?;
        let shadow_center = find_pixel_in_image(
            shadow_image,
            |p| p.0 == [255, 255, 255, 255],
            animation_name,
            column_nb,
            row_nb,
            "shadows",
            "white",
        )?;
        Ok(FrameOffset {
            head: black_offset.unwrap_or(green_offset),
            center: green_offset,
            hand_left: red_offset,
            hand_right: blue_offset,
            shadow: shadow_center,
        })
    }
}
