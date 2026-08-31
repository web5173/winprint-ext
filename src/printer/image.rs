use super::DxgiPrintContext;
use super::DxgiPrintContextError;
use crate::printer::FilePrinter;
use crate::printer::PrinterDevice;
use crate::ticket::PrintTicket;
use crate::ticket::ToDevModeError;
use crate::utils::wchar;
use std::path::Path;
use thiserror::Error;
use windows::core::PCWSTR;
use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_F;
use windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC;
use windows::Win32::Graphics::Imaging::GUID_WICPixelFormat32bppPBGRA;
use windows::Win32::Graphics::Imaging::WICBitmapDitherTypeNone;
use windows::Win32::Graphics::Imaging::WICBitmapPaletteTypeMedianCut;
use windows::Win32::Graphics::Imaging::WICDecodeMetadataCacheOnDemand;
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, GetDeviceCaps, LOGPIXELSX,
    PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH,
};
use windows_numerics::Matrix3x2;

#[derive(Error, Debug)]
/// Represents an error from [`ImagePrinter`].
pub enum ImagePrinterError {
    /// DXGI print context error.
    #[error("DXGI print context error")]
    DxgiPrintContextError(#[from] DxgiPrintContextError),
    /// Print ticket error.
    #[error("Print ticket error")]
    PrintTicketError(#[from] ToDevModeError),
    /// Invalid path.
    #[error("Invalid path")]
    InvalidPath(#[source] std::io::Error),
    /// Failed to open the document.
    #[error("Failed to open the document")]
    FailedToOpenDocument(#[source] windows::core::Error),
    /// Render error.
    #[error("Render error")]
    RenderError(#[source] windows::core::Error),
}

/// A printer that prints images. Multiple frames in a single image file will be printed as separate pages.
pub struct ImagePrinter {
    printer: PrinterDevice,
}

impl ImagePrinter {
    /// Create a new [`ImagePrinter`] for the given printer device.
    pub fn new(printer: PrinterDevice) -> Self {
        Self { printer }
    }

    /// Print an image with additional options.
    ///
    /// When `auto_rotate` is true, the image will be automatically rotated 90°
    /// to best fit the paper orientation (only when the image and paper aspect
    /// ratios mismatch). This is useful when no explicit orientation parameter
    /// is provided by the caller.
    pub fn print_with_options(
        &self,
        path: &Path,
        options: PrintTicket,
        auto_rotate: bool,
    ) -> std::result::Result<(), ImagePrinterError> {
        let context = DxgiPrintContext::new(
            &self.printer,
            &options,
            path.file_name().unwrap_or(path.as_ref()),
        )?;
        let wic_factory = &context.wic_factory;
        let print_control = &context.print_control;
        let d2d_context = &context.d2d_context;
        unsafe {
            let absolute_path =
                std::path::absolute(path).map_err(ImagePrinterError::InvalidPath)?;
            let image_decoder = wic_factory
                .CreateDecoderFromFilename(
                    PCWSTR(wchar::to_wide_chars(absolute_path.as_os_str()).as_ptr()),
                    None,
                    GENERIC_READ,
                    WICDecodeMetadataCacheOnDemand,
                )
                .map_err(ImagePrinterError::FailedToOpenDocument)?;

            let page_count = image_decoder
                .GetFrameCount()
                .map_err(ImagePrinterError::RenderError)?;
            for i in 0..page_count {
                let frame = image_decoder
                    .GetFrame(i)
                    .map_err(ImagePrinterError::RenderError)?;

                let mut image_width = 0;
                let mut image_height = 0;
                frame
                    .GetSize(&mut image_width, &mut image_height)
                    .map_err(ImagePrinterError::RenderError)?;
                let mut image_dpi_x = 0.0;
                let mut image_dpi_y = 0.0;
                frame
                    .GetResolution(&mut image_dpi_x, &mut image_dpi_y)
                    .map_err(ImagePrinterError::RenderError)?;

                let natural_page_size = D2D_SIZE_F {
                    width: (image_width as f64 * 96.0 / image_dpi_x) as f32,
                    height: (image_height as f64 * 96.0 / image_dpi_y) as f32,
                };

                // Determine if auto-rotation is needed (before scaling)
                let need_rotate = auto_rotate
                    && natural_page_size.width > natural_page_size.height;

                // Query physical paper dimensions via GDI for fit-to-page scaling
                let dev_mode_for_dc = options
                    .to_dev_mode(&self.printer)
                    .map_err(ImagePrinterError::PrintTicketError)?;
                let print_driver = PCWSTR(
                    ['W' as u16, 'I' as u16, 'N' as u16, 'S' as u16, 'P' as u16,
                     'O' as u16, 'O' as u16, 'L' as u16, 0].as_ptr(),
                );
                let hdc = CreateDCW(
                    print_driver,
                    PCWSTR(wchar::to_wide_chars(self.printer.os_name()).as_ptr()),
                    None,
                    Some(dev_mode_for_dc.as_ptr() as *const _),
                );
                let (page_size, translate_x, translate_y, dest_w, dest_h, scale, actual_rotate) =
                    if hdc.is_invalid() {
                        (natural_page_size, 0.0f64, 0.0f64,
                         natural_page_size.width as f64, natural_page_size.height as f64,
                         1.0f64, false)
                    } else {
                        let paper_w = GetDeviceCaps(Some(hdc), PHYSICALWIDTH);
                        let paper_h = GetDeviceCaps(Some(hdc), PHYSICALHEIGHT);
                        let offset_x = GetDeviceCaps(Some(hdc), PHYSICALOFFSETX);
                        let offset_y = GetDeviceCaps(Some(hdc), PHYSICALOFFSETY);
                        let dpi_x = GetDeviceCaps(Some(hdc), LOGPIXELSX);
                        let _ = DeleteDC(hdc);

                        // Convert device units to DIPs (96 DPI)
                        let paper_w_dips = paper_w as f64 * 96.0 / dpi_x as f64;
                        let paper_h_dips = paper_h as f64 * 96.0 / dpi_x as f64;
                        let offset_x_dips = offset_x as f64 * 96.0 / dpi_x as f64;
                        let offset_y_dips = offset_y as f64 * 96.0 / dpi_x as f64;

                        // Check if rotation gives a better fit
                        let paper_landscape = paper_w_dips > paper_h_dips;
                        let should_rotate = need_rotate && paper_landscape != (natural_page_size.width > natural_page_size.height);

                        let (eff_w, eff_h) = if should_rotate {
                            (natural_page_size.height as f64, natural_page_size.width as f64)
                        } else {
                            (natural_page_size.width as f64, natural_page_size.height as f64)
                        };

                        let scale = f64::min(paper_w_dips / eff_w, paper_h_dips / eff_h);
                        let scaled_w = eff_w * scale;
                        let scaled_h = eff_h * scale;

                        // Center on printable area (compensate for physical offset)
                        let tx = -offset_x_dips + (paper_w_dips - scaled_w) / 2.0;
                        let ty = -offset_y_dips + (paper_h_dips - scaled_h) / 2.0;

                        let page_size = D2D_SIZE_F {
                            width: paper_w_dips as f32,
                            height: paper_h_dips as f32,
                        };
                        // dest rect uses ORIGINAL bitmap dimensions;
                        // the transform matrix handles scale + rotate + translate
                        (page_size, tx, ty,
                         natural_page_size.width as f64, natural_page_size.height as f64,
                         scale, should_rotate)
                    };

                let format_converter = wic_factory
                    .CreateFormatConverter()
                    .map_err(ImagePrinterError::RenderError)?;
                format_converter
                    .Initialize(
                        &frame,
                        &GUID_WICPixelFormat32bppPBGRA,
                        WICBitmapDitherTypeNone,
                        None,
                        0.0,
                        WICBitmapPaletteTypeMedianCut,
                    )
                    .map_err(ImagePrinterError::RenderError)?;
                let bitmap = d2d_context
                    .CreateBitmapFromWicBitmap(&format_converter, None)
                    .map_err(ImagePrinterError::RenderError)?;

                let command_list = d2d_context
                    .CreateCommandList()
                    .map_err(ImagePrinterError::RenderError)?;
                d2d_context.SetTarget(&command_list);

                d2d_context.BeginDraw();

                // Apply combined transform: scale + optional 90° CW rotation + translate
                // For no rotation: standard scale then translate
                // For 90° CW rotation: scale, rotate, then translate
                //   Rotated bitmap: x-axis → (0, scale), y-axis → (scale, 0)
                //   i.e., image width maps to vertical, image height maps to horizontal
                let s = scale as f32;
                let matrix = if actual_rotate {
                    Matrix3x2 {
                        M11: 0.0, M12: s,
                        M21: s,   M22: 0.0,
                        M31: translate_x as f32, M32: translate_y as f32,
                    }
                } else {
                    Matrix3x2 {
                        M11: s,   M12: 0.0,
                        M21: 0.0, M22: s,
                        M31: translate_x as f32, M32: translate_y as f32,
                    }
                };
                d2d_context.SetTransform(&matrix);

                d2d_context.DrawBitmap(
                    &bitmap,
                    Some(&D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: dest_w as f32,
                        bottom: dest_h as f32,
                    }),
                    1.0,
                    D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                    None,
                    None,
                );
                d2d_context
                    .EndDraw(None, None)
                    .map_err(ImagePrinterError::RenderError)?;
                command_list
                    .Close()
                    .map_err(ImagePrinterError::RenderError)?;
                print_control
                    .AddPage(&command_list, page_size, None, None, None)
                    .map_err(ImagePrinterError::RenderError)?;
            }
        }
        context.close_and_wait()?;
        Ok(())
    }
}

impl FilePrinter for ImagePrinter {
    type Options = PrintTicket;
    type Error = ImagePrinterError;
    fn print(
        &self,
        path: &Path,
        options: PrintTicket,
    ) -> std::result::Result<(), ImagePrinterError> {
        self.print_with_options(path, options, false)
    }
}

#[cfg(test)]
mod tests {
    use super::ImagePrinter;
    use crate::{printer::FilePrinter, test_utils::null_device};
    use std::path::Path;

    #[test]
    fn print_simple_tiff_document() {
        let device = null_device::thread_local();
        let image = ImagePrinter::new(device);
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/test_document.tiff");
        image.print(path.as_path(), Default::default()).unwrap();
    }
}
