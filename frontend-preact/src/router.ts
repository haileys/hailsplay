import { createContext } from "preact";
import { useEffect, useState } from "preact/hooks";

export type Route
	= "index"
	| "selectRadioStation"
	;

type SetRouteFunc = (_: Route) => void;

let _setRoute: SetRouteFunc | null = null;

class Router {
    route: Route;

    constructor(route: Route) {
        this.route = route;
    }
}

export const RouteContext = createContext<Route>("index");

export function navigate

export default {
    createRouter(defaultRoute: Route) {
        return createContext(new Router(defaultRoute));
    }

    useRouter(defaultRoute: Route) {
        const [route, setRoute] = useState<Route>(defaultRoute);
        useContext()

        useEffect(() => {
            _setRoute = setRoute;
            return () => { _setRoute = null; };
        });
    }

    init(setRoute: SetRouteFunc) {
        if (__setRoute !== null) {
            throw "can't reinitialize router";
        }
    },

    deinit() {
        __setRoute = null;
    },

    navigate(route: Route) {
        if (__setRoute !== null) {
            __setRoute(route);
        }
    },
}
