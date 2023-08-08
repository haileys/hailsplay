# hailsplay

## Running in development

1. Install tool dependencies:
   ```sh-session
   $ cargo install trunk
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
   $ cd frontend
   frontend$ trunk serve --address 0.0.0.0
   ```
