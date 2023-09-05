import type { ComponentChildren } from "preact";
import { useContext } from "preact/hooks";
import { ModalContext } from "./App";
import closeIcon from "feather-icons/dist/icons/x.svg";
import css from "./Modal.module.css";

type Props = {
    title: string,
    children: ComponentChildren,
};

export default function Modal(props: Props) {
    const { setModal } = useContext(ModalContext);

    return (
        <div className={css.container}>
            <div className={css.backdrop} onClick={() => setModal(null)}></div>
            <div className={css.sheet}>
                <div className={css.titleBar}>
                    <div className={css.title}>
                        {props.title}
                    </div>
                    <button className={css.closeButton} onClick={() => setModal(null)}>
                        <img src={closeIcon} />
                    </button>
                </div>
                <div className={css.content}>
                    {props.children}
                </div>
            </div>
        </div>
    )
}
