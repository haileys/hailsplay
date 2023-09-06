import type { ComponentChildren } from "preact";
import { useContext } from "preact/hooks";
import { ReactComponent as CloseIcon } from "feather-icons/dist/icons/x.svg";
import { ModalContext } from "../routes";
import css from "./Modal.module.css";

type Props = {
    title: string,
    children: ComponentChildren,
};

export default function Modal(props: Props) {
    const { setModal } = useContext(ModalContext);

    return (
        <div class={css.container}>
            <div class={css.backdrop} onClick={() => setModal(null)}></div>
            <div class={css.sheet}>
                <div class={css.titleBar}>
                    <div class={css.title}>
                        {props.title}
                    </div>
                    <button class={css.closeButton} onClick={() => setModal(null)}>
                        <CloseIcon />
                    </button>
                </div>
                <div class={css.content}>
                    {props.children}
                </div>
            </div>
        </div>
    )
}
