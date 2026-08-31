# winprint-ext

A fork of [winprint](https://github.com/ArcticLampyrid/winprint.rs/) (BSD-3-Clause) that adds **enhanced image printing**: fit-to-page scaling, centering and automatic rotation (`ImagePrinter::print_with_options`).

> Windows only.

## Differences from upstream winprint

- `ImagePrinter::print_with_options(path, options, auto_rotate)`: prints an image fitted and centered on the physical paper; when `auto_rotate` is true the image is rotated 90° if that fits the paper better (used when no explicit orientation is given).

Everything else is identical to winprint 0.2.1.

## License

BSD-3-Clause. See `LICENSE.md`. Upstream: ArcticLampyrid/winprint.rs.
