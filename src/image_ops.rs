// this_file: haforu-src/src/image_ops.rs
//! Image processing operations for font matching optimization.
//!
//! This module provides high-performance image alignment and comparison
//! functions to accelerate fontsimi's deep matching pipeline.

use std::cmp::{max, min};

/// Alignment method for image comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignMethod {
    /// Center-align both images in a common canvas
    Center,
    /// Align images at top-left corner
    CornerTopLeft,
}

/// Result of image alignment and comparison
#[derive(Debug, Clone)]
pub struct AlignCompareResult {
    /// Aligned image A (may be padded)
    pub aligned_a: Vec<u8>,
    /// Aligned image B (may be padded)
    pub aligned_b: Vec<u8>,
    /// Width of aligned images
    pub width: u32,
    /// Height of aligned images
    pub height: u32,
    /// Mean absolute pixel difference (0.0 = identical, 255.0 = completely different)
    pub pixel_delta: f32,
    /// Center-weighted pixel delta using Gaussian kernel
    pub center_weighted_delta: f32,
    /// Ink density of image A (0.0 = white, 1.0 = black)
    pub density_a: f32,
    /// Ink density of image B (0.0 = white, 1.0 = black)
    pub density_b: f32,
    /// Aspect ratio of image A (width/height of cropped bounding box)
    pub aspect_a: f32,
    /// Aspect ratio of image B (width/height of cropped bounding box)
    pub aspect_b: f32,
}

/// Find bounding box of dark pixels in a grayscale image
///
/// Returns (min_x, min_y, max_x, max_y) or None if image is all white
fn find_dark_bounding_box(data: &[u8], width: u32, height: u32, threshold: u8) -> Option<(u32, u32, u32, u32)> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found_dark = false;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            if data[idx] < threshold {
                // Dark pixel found
                found_dark = true;
                min_x = min(min_x, x);
                min_y = min(min_y, y);
                max_x = max(max_x, x);
                max_y = max(max_y, y);
            }
        }
    }

    if found_dark {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// Compute ink density (fraction of dark pixels)
///
/// Returns value in [0.0, 1.0] where 0.0 = all white, 1.0 = all black
fn compute_density(data: &[u8], threshold: u8) -> f32 {
    if data.is_empty() {
        return 0.0;
    }

    let dark_count = data.iter().filter(|&&pixel| pixel < threshold).count();
    dark_count as f32 / data.len() as f32
}

/// Compute aspect ratio from bounding box
fn compute_aspect_ratio(bbox: Option<(u32, u32, u32, u32)>) -> f32 {
    if let Some((min_x, min_y, max_x, max_y)) = bbox {
        let width = (max_x - min_x + 1) as f32;
        let height = (max_y - min_y + 1) as f32;
        if height > 0.0 {
            width / height
        } else {
            1.0
        }
    } else {
        1.0 // Default for empty image
    }
}

/// Align two grayscale images and compute comparison metrics
///
/// # Arguments
/// * `data_a` - Grayscale image A (row-major, 0=black, 255=white)
/// * `width_a` - Width of image A
/// * `height_a` - Height of image A
/// * `data_b` - Grayscale image B (row-major, 0=black, 255=white)
/// * `width_b` - Width of image B
/// * `height_b` - Height of image B
/// * `method` - Alignment method (Center or CornerTopLeft)
///
/// # Returns
/// `AlignCompareResult` with aligned images and comparison metrics
///
/// # Performance
/// Target: <1ms per call (called 30-180 times per font pair)
pub fn align_and_compare(
    data_a: &[u8],
    width_a: u32,
    height_a: u32,
    data_b: &[u8],
    width_b: u32,
    height_b: u32,
    method: AlignMethod,
) -> AlignCompareResult {
    // Compute metrics before alignment
    let bbox_a = find_dark_bounding_box(data_a, width_a, height_a, 128);
    let bbox_b = find_dark_bounding_box(data_b, width_b, height_b, 128);

    let density_a = compute_density(data_a, 128);
    let density_b = compute_density(data_b, 128);

    let aspect_a = compute_aspect_ratio(bbox_a);
    let aspect_b = compute_aspect_ratio(bbox_b);

    // Determine target canvas size
    let target_width = max(width_a, width_b);
    let target_height = max(height_a, height_b);

    // Allocate aligned images (fill with white = 255)
    let mut aligned_a = vec![255u8; (target_width * target_height) as usize];
    let mut aligned_b = vec![255u8; (target_width * target_height) as usize];

    // Compute offsets based on alignment method
    let (offset_ax, offset_ay) = match method {
        AlignMethod::Center => {
            let ox = (target_width.saturating_sub(width_a)) / 2;
            let oy = (target_height.saturating_sub(height_a)) / 2;
            (ox, oy)
        }
        AlignMethod::CornerTopLeft => (0, 0),
    };

    let (offset_bx, offset_by) = match method {
        AlignMethod::Center => {
            let ox = (target_width.saturating_sub(width_b)) / 2;
            let oy = (target_height.saturating_sub(height_b)) / 2;
            (ox, oy)
        }
        AlignMethod::CornerTopLeft => (0, 0),
    };

    // Copy image A into aligned canvas
    for y in 0..height_a {
        let src_offset = (y * width_a) as usize;
        let dst_offset = ((offset_ay + y) * target_width + offset_ax) as usize;
        let src_slice = &data_a[src_offset..src_offset + width_a as usize];
        aligned_a[dst_offset..dst_offset + width_a as usize].copy_from_slice(src_slice);
    }

    // Copy image B into aligned canvas
    for y in 0..height_b {
        let src_offset = (y * width_b) as usize;
        let dst_offset = ((offset_by + y) * target_width + offset_bx) as usize;
        let src_slice = &data_b[src_offset..src_offset + width_b as usize];
        aligned_b[dst_offset..dst_offset + width_b as usize].copy_from_slice(src_slice);
    }

    // Compute pixel delta (mean absolute difference)
    let pixel_delta = compute_pixel_delta(&aligned_a, &aligned_b);

    // Compute center-weighted delta
    let center_weighted_delta = compute_center_weighted_delta(
        &aligned_a,
        &aligned_b,
        target_width,
        target_height,
    );

    AlignCompareResult {
        aligned_a,
        aligned_b,
        width: target_width,
        height: target_height,
        pixel_delta,
        center_weighted_delta,
        density_a,
        density_b,
        aspect_a,
        aspect_b,
    }
}

/// Compute mean absolute pixel difference
fn compute_pixel_delta(data_a: &[u8], data_b: &[u8]) -> f32 {
    if data_a.len() != data_b.len() || data_a.is_empty() {
        return 0.0;
    }

    let sum: u32 = data_a
        .iter()
        .zip(data_b.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).abs() as u32)
        .sum();

    sum as f32 / data_a.len() as f32
}

/// Compute center-weighted pixel delta using Gaussian kernel
///
/// Gives more weight to differences in the center of the image
fn compute_center_weighted_delta(
    data_a: &[u8],
    data_b: &[u8],
    width: u32,
    height: u32,
) -> f32 {
    if data_a.len() != data_b.len() || data_a.is_empty() {
        return 0.0;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let sigma = (width.min(height) as f32 / 4.0).max(1.0);

    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let diff = (data_a[idx] as i32 - data_b[idx] as i32).abs() as f32;

            // Gaussian weight
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist_sq = dx * dx + dy * dy;
            let weight = (-dist_sq / (2.0 * sigma * sigma)).exp();

            weighted_sum += diff * weight;
            weight_total += weight;
        }
    }

    if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        0.0
    }
}

/// Resize a grayscale image using bilinear interpolation
///
/// # Arguments
/// * `data` - Input grayscale image (row-major, 0=black, 255=white)
/// * `width` - Width of input image
/// * `height` - Height of input image
/// * `multiplier` - Scaling factor (1.0 = no scaling, >1.0 = enlarge, <1.0 = shrink)
///
/// # Returns
/// Tuple of (resized_data, new_width, new_height)
///
/// # Performance
/// Target: <2ms per call (replaces OpenCV cv2.resize)
/// Expected speedup: 2-3x over Python+OpenCV wrapper
///
/// # Panics
/// Panics if multiplier is not in range [0.01, 100.0]
pub fn resize_bilinear(
    data: &[u8],
    width: u32,
    height: u32,
    multiplier: f32,
) -> (Vec<u8>, u32, u32) {
    // Validate multiplier range
    if !(0.01..=100.0).contains(&multiplier) {
        panic!(
            "Scaling multiplier {} is out of safe range (0.01-100.0)",
            multiplier
        );
    }

    // Fast path: no scaling
    if (multiplier - 1.0).abs() < 0.001 {
        return (data.to_vec(), width, height);
    }

    // Fast path: empty image
    if data.is_empty() || width == 0 || height == 0 {
        return (Vec::new(), 0, 0);
    }

    // Compute new dimensions
    let new_width = ((width as f32 * multiplier).round() as u32).max(1);
    let new_height = ((height as f32 * multiplier).round() as u32).max(1);

    let mut output = vec![255u8; (new_width * new_height) as usize];

    // Bilinear interpolation
    let x_ratio = (width - 1) as f32 / new_width.max(1) as f32;
    let y_ratio = (height - 1) as f32 / new_height.max(1) as f32;

    for y in 0..new_height {
        for x in 0..new_width {
            // Map output pixel to input coordinates
            let src_x = x as f32 * x_ratio;
            let src_y = y as f32 * y_ratio;

            // Get integer and fractional parts
            let x0 = src_x.floor() as u32;
            let y0 = src_y.floor() as u32;
            let x1 = (x0 + 1).min(width - 1);
            let y1 = (y0 + 1).min(height - 1);

            let dx = src_x - x0 as f32;
            let dy = src_y - y0 as f32;

            // Get four neighboring pixels
            let idx00 = (y0 * width + x0) as usize;
            let idx01 = (y0 * width + x1) as usize;
            let idx10 = (y1 * width + x0) as usize;
            let idx11 = (y1 * width + x1) as usize;

            let p00 = data[idx00] as f32;
            let p01 = data[idx01] as f32;
            let p10 = data[idx10] as f32;
            let p11 = data[idx11] as f32;

            // Bilinear interpolation
            let top = p00 * (1.0 - dx) + p01 * dx;
            let bottom = p10 * (1.0 - dx) + p11 * dx;
            let value = top * (1.0 - dy) + bottom * dy;

            let out_idx = (y * new_width + x) as usize;
            output[out_idx] = value.round().clamp(0.0, 255.0) as u8;
        }
    }

    (output, new_width, new_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_identical_images() {
        // Two identical 2x2 images
        let data_a = vec![0, 255, 255, 0]; // Black corners
        let data_b = data_a.clone();

        let result = align_and_compare(&data_a, 2, 2, &data_b, 2, 2, AlignMethod::Center);

        assert_eq!(result.width, 2);
        assert_eq!(result.height, 2);
        assert!(result.pixel_delta < 0.1, "Identical images should have near-zero delta");
    }

    #[test]
    fn test_align_different_sizes() {
        // 2x2 image
        let data_a = vec![0, 0, 0, 0]; // All black

        // 3x3 image
        let data_b = vec![255; 9]; // All white

        let result = align_and_compare(&data_a, 2, 2, &data_b, 3, 3, AlignMethod::Center);

        assert_eq!(result.width, 3);
        assert_eq!(result.height, 3);
        assert!(result.pixel_delta > 0.0, "Different images should have non-zero delta");
    }

    #[test]
    fn test_density_all_white() {
        let data = vec![255; 100];
        let density = compute_density(&data, 128);
        assert!(density < 0.01, "All-white image should have near-zero density");
    }

    #[test]
    fn test_density_all_black() {
        let data = vec![0; 100];
        let density = compute_density(&data, 128);
        assert!(density > 0.99, "All-black image should have near-1.0 density");
    }

    #[test]
    fn test_bounding_box() {
        // 3x3 image with dark center pixel
        let data = vec![
            255, 255, 255,
            255, 0,   255,
            255, 255, 255,
        ];

        let bbox = find_dark_bounding_box(&data, 3, 3, 128);
        assert_eq!(bbox, Some((1, 1, 1, 1)), "Bounding box should be center pixel");
    }

    #[test]
    fn test_resize_no_scaling() {
        let data = vec![0, 128, 255, 64];
        let (resized, w, h) = resize_bilinear(&data, 2, 2, 1.0);

        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(resized, data, "No scaling should return identical data");
    }

    #[test]
    fn test_resize_upscale_2x() {
        // 2x2 checkerboard
        let data = vec![
            0, 255,
            255, 0,
        ];

        let (resized, w, h) = resize_bilinear(&data, 2, 2, 2.0);

        assert_eq!(w, 4);
        assert_eq!(h, 4);
        assert_eq!(resized.len(), 16);

        // Check corners are preserved
        assert_eq!(resized[0], 0);  // Top-left
        assert_eq!(resized[3], 255);  // Top-right
        assert_eq!(resized[12], 255);  // Bottom-left
        assert_eq!(resized[15], 0);  // Bottom-right
    }

    #[test]
    fn test_resize_downscale() {
        // 4x4 all black
        let data = vec![0u8; 16];

        let (resized, w, h) = resize_bilinear(&data, 4, 4, 0.5);

        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(resized.len(), 4);

        // All pixels should be black
        assert!(resized.iter().all(|&p| p == 0), "Downscaled all-black image should remain all-black");
    }

    #[test]
    fn test_resize_empty_image() {
        let data: Vec<u8> = Vec::new();
        let (resized, w, h) = resize_bilinear(&data, 0, 0, 2.0);

        assert_eq!(w, 0);
        assert_eq!(h, 0);
        assert!(resized.is_empty());
    }

    #[test]
    #[should_panic(expected = "out of safe range")]
    fn test_resize_invalid_multiplier() {
        let data = vec![0; 4];
        resize_bilinear(&data, 2, 2, 150.0);  // Out of range
    }
}
