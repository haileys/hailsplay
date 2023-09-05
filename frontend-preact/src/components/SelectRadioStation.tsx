import css from "./SelectRadioStation.module.css";
import pbsIcon from "../assets/radio-pbs.png";
import rrrIcon from "../assets/radio-rrr.png";
import grooveSaladIcon from "../assets/radio-soma-groovesalad.png";
import reggaeIcon from "../assets/radio-soma-reggae.png";

function radioStation(iconUrl: string) {
    return (
        <button className={css.item}>
            <img src={iconUrl} draggable={false} onTouchStart={(ev) => ev.preventDefault()} />
        </button>
    )
}

export default function SelectRadioStation() {
    return (
        <div className={css.grid}>
            {radioStation(pbsIcon)}
            {radioStation(rrrIcon)}
            {radioStation(grooveSaladIcon)}
            {radioStation(reggaeIcon)}
        </div>
    )
}
