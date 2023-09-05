import { JSX, createRef } from "preact";
import { useContext, useEffect, useState } from "preact/hooks";
import { Url, queueAdd } from "../api";

import { ReactComponent as PlusIcon } from "feather-icons/dist/icons/plus.svg";
import css from "./AddUrl.module.css";
import spinnerCss from "../spinner.module.css";

import { Metadata, metadata, catchAbortErrors } from "../api";
import { ModalContext } from "../routes";

type ViewState =
    | { state: "form" }
    | { state: "adding", controller: AbortController }
    ;

export default function AddUrl() {
    const { setModal } = useContext(ModalContext);

    let [view, setView] = useState<ViewState>({ state: "form" });

    if (view.state === "form") {
        let onsubmit = (url: string) => {
            let controller = new AbortController();

            catchAbortErrors(queueAdd(url, controller.signal))
                .then((result) => {
                    setModal(null);
                });

            setView({ state: "adding", controller: controller });
        };

        return (<Form onsubmit={onsubmit} />);
    } else {
        return (
            <div class={css.loadingSpinnerFullWidth}>
                <LoadingSpinner />
            </div>
        );
    }
}

type PreviewState =
    | { state: "none" }
    | { state: "loading", url: string, controller: AbortController }
    | { state: "ready", url: string, metadata: Metadata }
    ;

function Form(props: { onsubmit(_: Url): void }) {
    let [url, setUrl] = useState("");
    let [preview, setPreview] = useState<PreviewState>({ state: "none" });

    let input = createRef<HTMLInputElement>();

    let onInput = () => {
        let url = input.current?.value || "";
        setUrl(url);

        // if url is unchanged from current, do nothing
        if (preview.state !== "none" && preview.url === url) {
            return;
        }

        // cancel current request if there is one
        if (preview.state === "loading") {
            preview.controller.abort();
        }

        // if there is no url clear the preview
        if (url === "") {
            setPreview({ state: "none" });
            return;
        }

        // send new metadata request
        let controller = new AbortController();
        catchAbortErrors(metadata(url, controller.signal)).then((metadata) => {
            if (metadata !== null) {
                setPreview({ state: "ready", url, metadata });
            }
        });

        // finally set the preview state to loading
        setPreview({ state: "loading", url, controller });
    }

    let onSubmit = async () => {
        if (url !== "") {
            props.onsubmit(url);
        }
    };

    return (
        <div class={css.addUrl}>
            {renderPreview(preview)}
            <div class={css.inputBar}>
                <input
                    ref={input}
                    value={url}
                    onInput={onInput}
                    onSubmit={onSubmit}
                    class={css.urlInput}
                    type="text"
                    placeholder="Media URL"
                    inputMode="url"
                    enterkeyhint="go"
                    autoFocus={true}
                />
                {renderButton(preview, onSubmit)}
            </div>
        </div>
    );
}

function renderPreview(preview: PreviewState) {
    switch (preview.state) {
        case "none":
        case "loading":
            return null;

        case "ready":
            return (
                <div className={css.addPreview}>
                    <div className={css.previewCoverArt}>
                        {preview.metadata.thumbnail ? (
                            <img src={preview.metadata.thumbnail} />
                        ) : null}
                    </div>
                    <div className={css.previewDetails}>
                        <div class={css.previewTitle}>{preview.metadata.title}</div>
                        {preview.metadata.artist ? (
                            <div class={css.previewArtist}>{preview.metadata.artist}</div>
                        ) : null}
                    </div>
                </div>
            );
    }
}

function renderButton(preview: PreviewState, onclick: () => void) {
    switch (preview.state) {
        case "none":
            return (
                <button class={`${css.addButton} ${css.addButtonInactive}`} onClick={onclick}>
                    <PlusIcon />
                </button>
            );

        case "ready":
            return (
                <button class={css.addButton} onClick={onclick}>
                    <PlusIcon />
                </button>
            );

        case "loading":
            return (<LoadingSpinner />);
    }
}

function LoadingSpinner() {
    return (
        <div class={css.loadingSpinner}>
            <div class={spinnerCss.spinner}><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div></div>
        </div>
    );
}
