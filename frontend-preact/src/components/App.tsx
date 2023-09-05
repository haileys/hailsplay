import { createContext } from "preact";
import { StateUpdater, useContext, useState } from "preact/hooks";

import Footer from "./Footer";
import Modal from "./Modal";
import SelectRadioStation from "./SelectRadioStation";

export type Route = "index";
const defaultRoute: Route = "index";

export type Modal = null | "select-radio-station";

export const RouteContext = createContext<{ route: Route, setRoute: StateUpdater<Route> }>({
    route: defaultRoute,
    setRoute() {}
});

export const ModalContext = createContext<{ modal: Modal, setModal: StateUpdater<Modal> }>({
    modal: null,
    setModal() {}
});

function renderModal(modal: Modal) {
    switch (modal) {
        case null:
            return null;
        case "select-radio-station":
            return (
                <Modal title="Radio">
                    <SelectRadioStation />
                </Modal>
            );
    }
}

export function App() {
    const [ route, setRoute ] = useState<Route>(defaultRoute);
    const [ modal, setModal ] = useState<Modal>(null);

	return (
		<RouteContext.Provider value={{ route, setRoute }}>
            <ModalContext.Provider value={{ modal, setModal }}>
                <header>
                    <div class="app-name">{"hailsPlay"}</div>
                </header>
                <main>
                    <div class="main-border main-border-top"></div>
                    <div class="main-content">
                    </div>
                    <div class="main-border main-border-bottom"></div>
                </main>
                <Footer />

                {renderModal(modal)}
            </ModalContext.Provider>
		</RouteContext.Provider>
	);
}
