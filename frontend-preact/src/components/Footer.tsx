import { useContext } from "preact/hooks";

/// <reference types="vite-plugin-svgr/client" />
import { ReactComponent as PlusIcon } from "feather-icons/dist/icons/plus.svg";

/// <reference types="vite-plugin-svgr/client" />
import { ReactComponent as RadioIcon } from "feather-icons/dist/icons/radio.svg";

import { ModalContext } from "../routes";
import css from "./Footer.module.css";

export default function Footer() {
    const { setModal } = useContext(ModalContext);

    return (
        <div className={css.footer}>
            <button onClick={() => setModal("add-url")}>
                <PlusIcon />
                <span>Add</span>
            </button>
            <button onClick={() => setModal("select-radio-station")}>
                <RadioIcon />
                <span>Radio</span>
            </button>
        </div>
    );
}
