import { render } from "preact";

import { App } from "./components/App";
import { Router } from "./routes";

import "./style.css";
import { LiveSession } from "./socket";

function root() {
    return (
        <Router>
            <LiveSession>
                <App />
            </LiveSession>
        </Router>
    );
}

render(root(), document.body);
