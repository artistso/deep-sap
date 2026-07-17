# Media assets

The source repository intentionally excludes duplicated generated PNG/JPG previews and ZIP press bundles from the stabilization archive. The application uses the tracked `static/icon.svg` for its installable shell.

Press screenshots and downloadable press bundles should be produced as release artifacts rather than committed repeatedly into the source tree. This keeps pull requests reviewable and avoids storing identical binary blobs in several paths.
