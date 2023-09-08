# hailsplay

## Running in development

1. Install tool dependencies:
   ```sh-session
   $ cargo install wasm-bindgen-cli
   ```

2. Start mpd:
   ```sh-session
   $ script/start-mpd
   ```

3. Start hailsplay server:
   ```sh-session
   $ cargo run --bin hailsplay
   ```

4. Run frontend dev server:
   ```sh-session
   $ script/dev-frontend
   ```

5. Access the app via the URL shown by the frontend dev server. The dev server proxies back to the running instance of the hailsplay server, you can configure the proxy backend url in `frontend/vite.config.ts`
