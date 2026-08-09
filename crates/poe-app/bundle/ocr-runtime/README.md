This directory is populated with the platform OCR runtime during release packaging.

The Windows job copies the Tesseract executable, its runtime libraries, and English data here.
The AppImage job stages the English model here and packages `/usr/bin/tesseract` through cargo-packager.
