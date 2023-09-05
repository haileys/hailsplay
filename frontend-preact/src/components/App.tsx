import { RouteContext, ModalContext, RouteId, ModalId, defaultRoute } from "../routes";
import { useErrorBoundary, useState } from "preact/hooks";

import Footer from "./Footer";
import Modal from "./Modal";
import SelectRadioStation from "./SelectRadioStation";
import AddUrl from "./AddUrl";

function renderModal(modal: ModalId) {
    if (modal === null) {
        return null;
    }

    switch (modal.t) {
        case null:
            return null;
        case "select-radio-station":
            return (
                <Modal title="Radio">
                    <SelectRadioStation />
                </Modal>
            );
        case "add-url":
            return (
                <Modal title="Add online media">
                    <AddUrl />
                </Modal>
            );
        case "error":
            return (
                <Modal title="Application error">
                    {modal.message}
                </Modal>
            )
    }
}

export function App() {
    const [ route, setRoute ] = useState<RouteId>(defaultRoute);
    const [ modal, setModal ] = useState<ModalId>(null);

    // useErrorBoundary((error, errorInfo) => {
    //     setModal({ t: "error", message: error.toString() });
    // });

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
