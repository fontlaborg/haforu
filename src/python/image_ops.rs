// this_file: src/python/image_ops.rs
//! Python bindings for image processing operations.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use numpy::PyReadonlyArray2;

use crate::image_ops::{align_and_compare as rust_align_and_compare, resize_bilinear as rust_resize_bilinear, AlignMethod};

/// Python wrapper for AlignCompareResult
#[pyclass]
pub struct AlignCompareResult {
    /// Aligned image A as bytes
    #[pyo3(get)]
    pub aligned_a: Py<PyBytes>,

    /// Aligned image B as bytes
    #[pyo3(get)]
    pub aligned_b: Py<PyBytes>,

    /// Width of aligned images
    #[pyo3(get)]
    pub width: u32,

    /// Height of aligned images
    #[pyo3(get)]
    pub height: u32,

    /// Mean absolute pixel difference
    #[pyo3(get)]
    pub pixel_delta: f32,

    /// Center-weighted pixel delta
    #[pyo3(get)]
    pub center_weighted_delta: f32,

    /// Ink density of image A
    #[pyo3(get)]
    pub density_a: f32,

    /// Ink density of image B
    #[pyo3(get)]
    pub density_b: f32,

    /// Aspect ratio of image A
    #[pyo3(get)]
    pub aspect_a: f32,

    /// Aspect ratio of image B
    #[pyo3(get)]
    pub aspect_b: f32,
}

/// Align two grayscale images and compute comparison metrics.
///
/// This function provides a high-performance Rust implementation of image
/// alignment and comparison, eliminating Python overhead and intermediate
/// array allocations.
///
/// # Arguments
/// * `image_a` - Grayscale image A as 2D numpy array (H x W), dtype=uint8
/// * `image_b` - Grayscale image B as 2D numpy array (H x W), dtype=uint8
/// * `method` - Alignment method: "center" or "corner" (default: "center")
///
/// # Returns
/// `AlignCompareResult` with aligned images and all comparison metrics
///
/// # Performance
/// Target: <1ms per call (5-10x faster than Python+numpy)
/// Called 30-180 times per font pair in deep matching optimization
///
/// # Examples
/// ```python
/// import numpy as np
/// import haforu
///
/// # Create two test images
/// img_a = np.zeros((100, 200), dtype=np.uint8)
/// img_b = np.ones((120, 180), dtype=np.uint8) * 255
///
/// # Align and compare
/// result = haforu.align_and_compare(img_a, img_b, method="center")
///
/// print(f"Pixel delta: {result.pixel_delta:.2f}")
/// print(f"Density A: {result.density_a:.3f}")
/// print(f"Aligned size: {result.width}x{result.height}")
/// ```
#[pyfunction]
#[pyo3(signature = (image_a, image_b, method="center"))]
pub fn align_and_compare(
    py: Python<'_>,
    image_a: PyReadonlyArray2<u8>,
    image_b: PyReadonlyArray2<u8>,
    method: &str,
) -> PyResult<AlignCompareResult> {
    // Parse alignment method
    let align_method = match method {
        "center" => AlignMethod::Center,
        "corner" => AlignMethod::CornerTopLeft,
        _ => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                format!("Invalid alignment method: '{}'. Use 'center' or 'corner'", method)
            ));
        }
    };

    // Get array views
    let arr_a = image_a.as_array();
    let arr_b = image_b.as_array();

    // Extract dimensions
    let (height_a, width_a) = (arr_a.shape()[0] as u32, arr_a.shape()[1] as u32);
    let (height_b, width_b) = (arr_b.shape()[0] as u32, arr_b.shape()[1] as u32);

    // Convert to flat slices (row-major order)
    let data_a: Vec<u8> = arr_a.iter().copied().collect();
    let data_b: Vec<u8> = arr_b.iter().copied().collect();

    // Call Rust implementation
    let result = rust_align_and_compare(
        &data_a,
        width_a,
        height_a,
        &data_b,
        width_b,
        height_b,
        align_method,
    );

    // Convert result to Python
    Ok(AlignCompareResult {
        aligned_a: PyBytes::new_bound(py, &result.aligned_a).into(),
        aligned_b: PyBytes::new_bound(py, &result.aligned_b).into(),
        width: result.width,
        height: result.height,
        pixel_delta: result.pixel_delta,
        center_weighted_delta: result.center_weighted_delta,
        density_a: result.density_a,
        density_b: result.density_b,
        aspect_a: result.aspect_a,
        aspect_b: result.aspect_b,
    })
}

/// Python wrapper for ResizeResult
#[pyclass]
pub struct ResizeResult {
    /// Resized image as bytes
    #[pyo3(get)]
    pub image: Py<PyBytes>,

    /// Width of resized image
    #[pyo3(get)]
    pub width: u32,

    /// Height of resized image
    #[pyo3(get)]
    pub height: u32,
}

/// Resize a grayscale image using bilinear interpolation.
///
/// This function provides a high-performance Rust implementation of image
/// resizing, eliminating Python+OpenCV wrapper overhead.
///
/// # Arguments
/// * `image` - Grayscale image as 2D numpy array (H x W), dtype=uint8
/// * `multiplier` - Scaling factor (1.0 = no change, >1.0 = enlarge, <1.0 = shrink)
///
/// # Returns
/// `ResizeResult` with resized image data, new width, and new height
///
/// # Performance
/// Target: <2ms per call (2-3x faster than OpenCV cv2.resize)
/// Called 1-2 times per optimization iteration = 25-160 calls per font pair
///
/// # Raises
/// * `ValueError` - If multiplier is out of safe range [0.01, 100.0]
///
/// # Examples
/// ```python
/// import numpy as np
/// import haforu
///
/// # Create test image
/// img = np.zeros((100, 200), dtype=np.uint8)
///
/// # Resize by 2x
/// result = haforu.resize_bilinear(img, 2.0)
/// print(f"New size: {result.width}x{result.height}")
///
/// # Reconstruct numpy array
/// resized = np.frombuffer(result.image, dtype=np.uint8).reshape(result.height, result.width)
/// ```
#[pyfunction]
pub fn resize_bilinear(
    py: Python<'_>,
    image: PyReadonlyArray2<u8>,
    multiplier: f32,
) -> PyResult<ResizeResult> {
    // Validate multiplier
    if !(0.01..=100.0).contains(&multiplier) {
        return Err(pyo3::exceptions::PyValueError::new_err(
            format!("Scaling multiplier {} is out of safe range (0.01-100.0)", multiplier)
        ));
    }

    // Get array view
    let arr = image.as_array();

    // Extract dimensions
    let (height, width) = (arr.shape()[0] as u32, arr.shape()[1] as u32);

    // Convert to flat slice (row-major order)
    let data: Vec<u8> = arr.iter().copied().collect();

    // Call Rust implementation
    let (resized_data, new_width, new_height) = rust_resize_bilinear(
        &data,
        width,
        height,
        multiplier,
    );

    // Convert result to Python
    Ok(ResizeResult {
        image: PyBytes::new_bound(py, &resized_data).into(),
        width: new_width,
        height: new_height,
    })
}
