# Embedded instrument artwork

The section tabs, brand graphics and tuner background come from the supplied Neutron 1.0.3 app. The original SVGs were rasterized using Inkscape during build preparation. Cuprum Regular/Bold were reconstructed from the bundled WOFF files (`d7882aae4a49275b7865.woff` and `9f1fa472dc883e1dfd57.woff`) by reconstructing their SFNT tables.

These fonts and PNGs are embedded in the native executable. No browser, font service or external runtime asset files are required. Controls and layout are implemented in `src/instrument.rs`; resource checksums are in `docs/asset-checksums.json` and attribution is in `THIRD_PARTY.md`.
