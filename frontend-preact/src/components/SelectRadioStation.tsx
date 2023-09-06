import css from "./SelectRadioStation.module.css";
import pbsIcon from "../assets/radio-pbs.png";
import rrrIcon from "../assets/radio-rrr.png";
import grooveSaladIcon from "../assets/radio-soma-groovesalad.png";
import reggaeIcon from "../assets/radio-soma-reggae.png";
import { tuneRadio } from "../api";

type Station = { iconUrl: string, streamUrl: string };

const RadioStations = [
    { iconUrl: pbsIcon, streamUrl: "https://playerservices.streamtheworld.com/api/livestream-redirect/3PBS_FMAAC128.aac" },
    { iconUrl: rrrIcon, streamUrl: "http://realtime.rrr.org.au/p1h" },
    { iconUrl: grooveSaladIcon, streamUrl: "https://ice4.somafm.com/groovesalad-256-mp3" },
    { iconUrl: reggaeIcon, streamUrl: "https://ice4.somafm.com/reggae-256-mp3" },
]

function renderRadioStation(station: Station) {
    let onClick = () => {
        alert("click!");
        tuneRadio(station.streamUrl);
    };

    let onTouchStart = (ev: Event) => {
        ev.preventDefault();
        tuneRadio(station.streamUrl);
    };

    return (
        <button className={css.item} onClick={onClick}>
            <img src={station.iconUrl} draggable={false} onTouchStart={onTouchStart} />
        </button>
    )
}

export default function SelectRadioStation() {
    return (
        <div className={css.grid}>
            {RadioStations.map(renderRadioStation)}
        </div>
    )
}
