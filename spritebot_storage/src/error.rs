use std::{io, num::TryFromIntError};

use image::ImageError;
use thiserror::Error;
use vfs::VfsError;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SpriteBotStorageError {
    #[error("Error with the underlying virtual file system with the file {0}")]
    VfsError(#[source] VfsError, String),
    #[error("Error while reading the AnimData.xml file")]
    XmlReadError(#[from] quick_xml::DeError),
    #[error("Error while writing the AnimData.xml file")]
    XmlWriteError(#[from] quick_xml::SeError),
    #[error("Error reading the image {0} for the animation {1}")]
    ErrorImageRead(String, String, #[source] image::ImageError),
    #[error("The {0} for the image {1}-{2} is zero")]
    SpriteSizeZero(String, String, String),
    #[error("The {0} of the image {1}-{2} isn’t a multiple of it’s set {0}")]
    SpriteSizeNotMultiple(String, String, String),
    #[error("The images for the animation {0} doesn’t have an identical size")]
    SpriteSizeNotIdentical(String),
    #[error("The image for the animation {0} has size for {1} frame per column, but the frame duration data in the XML account for {2} frames.")]
    InconsistantDuration(String, usize, usize),
    #[error("For the animation {0}, column {1} row {2}: In the image for {3}, the pixel with the color {4} can’t be found")]
    ColorNotFoundInPixelData(String, usize, usize, String, String),
    #[error("For the animation {0}, column {1} row {2}: In the image for {3}, there are multiple {4} pixels")]
    ColorDuplicateInPixelDate(String, usize, usize, String, String),
    #[error("There is a too large duration (max is about 255 frames)")]
    TooLargeDuration(#[source] TryFromIntError),
    #[error("The dimension of a sheet is too large (more than 2^32). You probably have an insanly large amount of image.")]
    TooLargeGeneratedSheet(#[source] TryFromIntError),
    #[error(
        "The position of the head offset in file is invalid, as it would overwrite an hand offset"
    )]
    InvalidHeadPosition,
    #[error("Error writing the image at {1}")]
    WriteImageError(#[source] ImageError, String),
    #[error("Error writing to {1}")]
    WriteFileError(#[source] io::Error, String),
    #[error("An offset is placed somewhere too far from the original (should only happen on sheet greater than 2^16 in width or height)")]
    OffsetTooLarge,
}
