import { Component, JSX } from "preact";
import { ActionProps } from "./Footer";
import { useState } from "preact/hooks";

// export default class AddUrl extends Component {
//     state = { url: "" };

//     onInput = ev => {
//         let { value }= ev.target;
//         this.setState({ value });
//     }
// }

export default function AddUrl(props: ActionProps) {
    let [url, setUrl] = useState("");

    let onInput = (ev: JSX.TargetedEvent<HTMLInputElement, Event>) => {
        if (ev.target) {
            setUrl(ev.target.value);
        }
    };

    let onSubmit = () => {
        console.log(url);
    };

    return (
        <div class="add">
            <input
                value={url}
                class="url-input"
                type="text"
                placeholder="Add URL..."
                inputMode="url"
                enterkeyhint="go"
                onInput={onInput}
                onSubmit={onSubmit}
            />
        </div>
    )
}
