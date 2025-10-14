

Install prerequisites

Rust stable and wasm32-unknown-unknown target:

rustup target add wasm32-unknown-unknown

Trunk (zero-config web bundler for Rust Wasm):

cargo install trunk

Run dev server

trunk serve --open

Trunk builds to Wasm, spins a local web server, and opens your browser.

Release build

trunk build --release

Outputs to dist/ (ready to host as static files).

Note on versions: If wgpu API changes, adjust minor versions in Cargo.toml (comments provided). The scaffold keeps APIs conventional (State/init/resize/update/render pattern) so upgrades are trivial.