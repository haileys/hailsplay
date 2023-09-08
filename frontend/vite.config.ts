import { defineConfig } from "vite";
import preact from "@preact/preset-vite";
import svgr from "vite-plugin-svgr";

const PROXY_BACKEND = "localhost:3000";

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
				target: "ws://" + PROXY_BACKEND,
				ws: true,
			},
			"/api": {
				target: "http://" + PROXY_BACKEND,
			},
			// we need to use a regex here to avoid conflicting with vite's
			// own assets route:
			"^/assets/\\d+/[0-9a-f]+/.*": {
				target: "http://" + PROXY_BACKEND,
			},
		}
	}
});
