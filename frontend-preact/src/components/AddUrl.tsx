import { JSX, createRef } from "preact";
import { useEffect, useState } from "preact/hooks";
import { queueAdd } from "../api";

import css from "./AddUrl.module.css";
import spinnerCss from "../spinner.module.css";

import { Metadata, metadata } from "../api";

type PreviewState
    = { state: "none" }
    | { state: "loading", url: string, controller: AbortController }
    | { state: "ready", url: string, metadata: Metadata }
    ;

function fetchMetadata(url: string, signal: AbortSignal): Promise<Metadata | null> {
    return metadata(url, signal)
        .catch((error) => {
            // silence abort errors
            if (error instanceof DOMException) {
                if (error.name === "AbortError") {
                    return null;
                }
            }

            throw error;
        });
}

function renderPreview(preview: PreviewState) {
    switch (preview.state) {
        case "none":
            return null;

        case "loading":
            return (
                <div className={css.loadingSpinner}>
                    <div className={spinnerCss.spinner}><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div></div>
                </div>
            );

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

export default function AddUrl() {
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
        fetchMetadata(url, controller.signal).then((metadata) => {
            if (metadata !== null) {
                setPreview({ state: "ready", url, metadata });
            }
        });

        // finally set the preview state to loading
        setPreview({ state: "loading", url, controller });
    }

    let onSubmit = async () => {
        if (url !== "") {
            await queueAdd(url);
            // TODO error handling
        }
    };

    return (
        <div className={css.addUrl}>
            {renderPreview(preview)}
            <input
                ref={input}
                value={url}
                onInput={onInput}
                onSubmit={onSubmit}
                className={css.urlInput}
                type="text"
                placeholder="Media URL"
                inputMode="url"
                enterkeyhint="go"
                autoFocus={true}
            />
        </div>
    )
}
