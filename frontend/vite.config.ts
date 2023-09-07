import { defineConfig } from "vite";
import preact from "@preact/preset-vite";
import svgr from "vite-plugin-svgr";
// import eslint from "vite-plugin-eslint";

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [
		preact(),
		svgr(),
		// eslint(),
	],
	server: {
		proxy: {
			"/ws": {
				target: "ws://localhost:3000",
				ws: true,
			},
			"/api": {
				target: "http://localhost:3000",
			},
			// we need to use a regex here to avoid conflicting with vite's
			// own assets route:
			"^/assets/\\d+/[0-9a-f]+/.*": {
				target: "http://localhost:3000",
			},
		}
	}
});
