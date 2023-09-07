import { useContext, useEffect, useState } from "preact/hooks";

import css from "./SelectRadioStation.module.css";
import { tuneRadio, radioStations } from "../api";
import { RadioStation } from "../types";
import { LoadingSpinnerBlock } from "./LoadingSpinner";
import { ModalContext } from "../routes";

export default function SelectRadioStation() {
    const [stations, setStations] = useState<RadioStation[] | null>(null);

    useEffect(() => {
        radioStations().then((stations) => setStations(stations));
        return () => {};
    }, []);

    if (stations === null) {
        return (<LoadingSpinnerBlock />);
    } else {
        return (
            <div className={css.grid}>
                {stations.map((station, index) => <StationButton station={station} key={index} />)}
            </div>
        );
    }
}

function StationButton(props: { station: RadioStation }) {
    const { setModal } = useContext(ModalContext);

    let onPick = (ev: Event) => {
        ev.preventDefault();
        tuneRadio(props.station.stream_url);
        setModal(null);
    };

    return (
        <button className={css.item} onClick={onPick}>
            <img
                title={props.station.name}
                alt={props.station.name}
                src={props.station.icon_url}
                draggable={false}
                onTouchStart={onPick}
            />
        </button>
    )
}
