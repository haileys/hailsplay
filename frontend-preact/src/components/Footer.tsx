import feedIcon from "feathericon/build/svg/feed.svg";
import plusIcon from "feathericon/build/svg/plus.svg";
import AddUrl from "./AddUrl";
import { useContext } from "preact/hooks";
import { ModalContext } from "./App";

export default function Footer() {
    const { setModal } = useContext(ModalContext);

    return (
        <footer>
            <div class="footer-actions">
                <button>
                    <img src={plusIcon} />
                    Add
                </button>
                <button onClick={() => setModal("select-radio-station")}>
                    <img src={feedIcon} />
                    Radio
                </button>
            </div>
        </footer>
    );
}
