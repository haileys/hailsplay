import { createContext } from "preact";
import { StateUpdater, useState } from "preact/hooks";

export type RouteId = "index";

export type ModalId = null
    | "select-radio-station"
    | "add-url"
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
