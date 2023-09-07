import { ModalContext, ModalId } from "../routes";
import { useContext, useEffect, useErrorBoundary } from "preact/hooks";

import Footer from "./Footer";
import Modal from "./Modal";
import SelectRadioStation from "./SelectRadioStation";
import AddUrl from "./AddUrl";
import { ApiError } from "../api";
import Player from "./Player";
import css from "./App.module.css";
import { ReactComponent as RoundedCorner } from "../assets/rounded-corner.svg";

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
    const { modal, setModal } = useContext(ModalContext);
    useErrorHandling(setModal);

    // const live = useContext(LiveContext);

    return (
        <>
            <div class={css.header}>
                <div class={css.appName}>{"hailsPlay"}</div>
            </div>
            <div class={css.insetContainer}>
                <RoundedCorner class={`${css.roundedCorner} ${css.roundedCornerTopLeft}`} />
                <RoundedCorner class={`${css.roundedCorner} ${css.roundedCornerTopRight}`} />
                <RoundedCorner class={`${css.roundedCorner} ${css.roundedCornerBottomLeft}`} />
                <RoundedCorner class={`${css.roundedCorner} ${css.roundedCornerBottomRight}`} />
                <div class={css.scrollContainer}>
                    <div class={css.clientArea}>
                        <Player />
                    </div>
                </div>
            </div>
            <Footer />

            {renderModal(modal)}
        </>
	);
}

function useErrorHandling(setModal: (_: ModalId) => void) {
    let handleError = (error: any) => {
        let message = error.toString();
        setModal({ t: "error", message });
    };

    useErrorBoundary((error, _errorInfo) => { handleError(error); });

    useGlobalUnhandledRejectionHandler((ev: PromiseRejectionEvent) => {
        if (ev.reason instanceof ApiError) {
            handleError(ev.reason.message);
        } else {
            handleError(ev.reason);
        }
    });
}

function useGlobalUnhandledRejectionHandler(handler: (_: PromiseRejectionEvent) => void) {
    useEffect(() => {
        window.addEventListener("unhandledrejection", handler);
        return () => {
            window.removeEventListener("unhandledrejection", handler);
        };
    })
}
