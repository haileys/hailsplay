import { ComponentChildren, createContext } from "preact";
import { StateUpdater, useState } from "preact/hooks";

export type RouteId = "index";

export type ModalId = null
    | { t: "error", message: string }
    | { t: "select-radio-station" }
    | { t: "add-url" }
    ;

export const defaultRoute: RouteId = "index";

export const RouteContext = createContext<{ route: RouteId, setRoute: StateUpdater<RouteId> }>({
    route: defaultRoute,
    setRoute() {}
});

export const ModalContext = createContext<{ modal: ModalId, setModal: StateUpdater<ModalId> }>({
    modal: null,
    setModal() {}
});

export function Router(props: { children: ComponentChildren }) {
    const [ route, setRoute ] = useState<RouteId>(defaultRoute);
    const [ modal, setModal ] = useState<ModalId>(null);

	return (
		<RouteContext.Provider value={{ route, setRoute }}>
            <ModalContext.Provider value={{ modal, setModal }}>
                {props.children}
            </ModalContext.Provider>
        </RouteContext.Provider>
    );
}
