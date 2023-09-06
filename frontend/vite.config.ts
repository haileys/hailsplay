import { defineConfig } from "vite";
import preact from "@preact/preset-vite";
import svgr from "vite-plugin-svgr";

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [preact(), svgr()],
	server: {
		proxy: {
			"/ws": {
				target: "ws://localhost:3000",
				ws: true,
			},
			"/api": {
				target: "http://localhost:3000",
			},
		}
	}
});
