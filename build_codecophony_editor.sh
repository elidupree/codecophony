cargo build -p codecophony -p codecophony_editor_shared -p codecophony_editor_backend && \
cargo web build -p codecophony_editor_frontend --target=asmjs-unknown-emscripten && \
cd ./codecophony_editor && npm install
