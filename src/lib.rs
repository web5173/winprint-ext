#![cfg(windows)]
#![warn(missing_docs)]

//! A crate for printing to a Windows printer device using Windows API.
//!
//! # Examples
//! ## Print a file
//! For a simple example presenting how to print a file:
//! - Filter for the device you want to use.
//! - Wrap the printer device with a printer.
//! - Send a file to the printer.
//!
//! First, get all printer devices via `PrinterDevice::all()` and filter for the device you want to use.
//!
//! ```rust
//! use winprint_ext::printer::PrinterDevice;
//!
//! fn get_my_device() -> PrinterDevice {
//!     let printers = PrinterDevice::all().expect("Failed to get printers");
//!     printers
//!         .into_iter()
//!         .find(|x| x.name() == "My Printer")
//!         .expect("My Printer not found")
//! }
//! ```
//!
//! Then, create a printer and send a file to it. Currently, there are two kinds of printers available:
//! - [`printer::XpsPrinter`]: For printing XPS files.
//! - [`printer::PdfiumPrinter`]: For printing PDF files via PDFium library. (Feature `pdfium` must be enabled)
//!
//! **Note**: The concept *`Printer`* here is a warpper of device for printing specific types of data,
//! not meaning the printer device.
//!
//! ```rust,no_run
//! use std::path::Path;
//! use winprint_ext::printer::FilePrinter;
//! use winprint_ext::printer::PrinterDevice;
//! use winprint_ext::printer::XpsPrinter;
//!
//! let my_device = PrinterDevice::all()
//!     .expect("Failed to get printers")
//!     .into_iter()
//!     .next()
//!     .expect("No printer found");
//! let xps = XpsPrinter::new(my_device);
//! let path = Path::new("path/to/test/document.xps");
//! xps.print(path, Default::default()).unwrap();
//! ```
//!
//! ## Specify the printing preferences
//! Print ticket is a set of options that can be to specify the printing preferences,
//! It can be used to set options such as the media size, orientation, and so on.
//! If you want to specify the printing preferences, you may use print tickets.
//!
//! See [Print Schema Specification] for technical details.
//!
//! Here is an example presenting how to use print tickets with this crate:
//! - Fetch print capabilities from the printer device.
//! - Filter the capabilities you want to use.
//! - Create a print ticket builder for your printer device.
//! - Merge the capabilities into the print ticket you are to build.
//! - Build the print ticket.
//! - Print the file with the print ticket.
//!
//! ```rust,no_run
//! use std::path::Path;
//! use winprint_ext::printer::FilePrinter;
//! use winprint_ext::printer::PrinterDevice;
//! use winprint_ext::printer::XpsPrinter;
//! use winprint_ext::ticket::FeatureOptionPackWithPredefined;
//! use winprint_ext::ticket::PredefinedMediaName;
//! use winprint_ext::ticket::PrintCapabilities;
//! use winprint_ext::ticket::PrintTicket;
//! use winprint_ext::ticket::PrintTicketBuilder;
//!
//! let my_device = PrinterDevice::all()
//!     .expect("Failed to get printers")
//!     .into_iter()
//!     .next()
//!     .expect("No printer found");
//! let capabilities = PrintCapabilities::fetch(&my_device).unwrap();
//! let a4_media = capabilities
//!     .page_media_sizes()
//!     .find(|x| x.as_predefined_name() == Some(PredefinedMediaName::ISOA4))
//!     .unwrap();
//! let mut builder = PrintTicketBuilder::new(&my_device).unwrap();
//! builder.merge(a4_media).unwrap();
//! let ticket = builder.build().unwrap();
//! let xps = XpsPrinter::new(my_device);
//! let path = Path::new("path/to/test/document.xps");
//! xps.print(path, ticket).unwrap();
//! ```
//!
//! [Print Schema Specification]: https://learn.microsoft.com/en-us/windows/win32/printdocs/printschema
//!
//! # Features
//! - `pdfium`: Enable PDFium support for printing PDF files.

mod bindings;
/// Provides a way to print various types of data to a printer device.
pub mod printer;
/// Utilities for testing
pub mod test_utils;
/// Provides a way to specify the printing preferences.
pub mod ticket;
mod utils;

