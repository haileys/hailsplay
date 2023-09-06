import css from "./SelectRadioStation.module.css";
import { tuneRadio, RadioStation, radioStations } from "../api";
import { useEffect, useState } from "preact/hooks";
import { LoadingSpinnerBlock } from "./LoadingSpinner";

function renderRadioStation(station: RadioStation) {
    let onPick = (ev: Event) => {
        ev.preventDefault();
        tuneRadio(station.stream_url);
    };

    return (
        <button className={css.item} onClick={onPick}>
            <img
                title={station.name}
                alt={station.name}
                src={station.icon_url}
                draggable={false}
                onTouchStart={onPick}
            />
        </button>
    )
}

export default function SelectRadioStation() {
    const [stations, setStations] = useState<RadioStation[] | null>(null);

    useEffect(() => {
        radioStations().then((stations) => setStations(stations));
        return () => {};
    })

    if (stations === null) {
        return (<LoadingSpinnerBlock />);
    } else {
        return (
            <div className={css.grid}>
                {stations.map(renderRadioStation)}
            </div>
        );
    }
}
