import { JSX, createRef } from "preact";
import { useState } from "preact/hooks";
import { queueAdd } from "../api";

export default function AddUrl() {
    let [url, setUrl] = useState("");
    let input = createRef<HTMLInputElement>();

    let onSubmit = async () => {
        await queueAdd(url);
        // TODO error handling
    };

    return (
        <div class="add">
            <input
                ref={input}
                value={url}
                onInput={() => input.current && setUrl(input.current.value)}
                onSubmit={onSubmit}
                className="url-input"
                type="text"
                placeholder="Add URL..."
                inputMode="url"
                enterkeyhint="go"
            />
        </div>
    )
}
