import css from "./Player.module.css";
import iconUrl from "../assets/radio-soma-groovesalad.png";

import { ReactComponent as PlayIcon } from "feather-icons/dist/icons/play.svg";
import { ReactComponent as StopIcon } from "feather-icons/dist/icons/square.svg";
import { ReactComponent as PauseIcon } from "feather-icons/dist/icons/pause.svg";
import { ReactComponent as SkipBackIcon } from "feather-icons/dist/icons/skip-back.svg";
import { ReactComponent as SkipForwardIcon } from "feather-icons/dist/icons/skip-forward.svg";
import { LiveContext } from "../socket";
import { PlayerStatus } from "../types";
import { useContext } from "preact/hooks";

function CurrentTrack() {
    const live = useContext(LiveContext);

    let track = live.currentTrack.value;
    if (track === null) {
        return null;
    }

    return (
        <>
            <div class={css.coverArtContainer}>
                <div class={css.coverArtInset}>
                    <img src={track.imageUrl} />
                </div>
            </div>
            <div class={css.trackInfo}>
                <div class={css.trackPrimaryLabel}>
                    {track.primaryLabel}
                </div>
                <div class={css.trackSecondaryLabel}>
                    {track.secondaryLabel}
                </div>
            </div>
        </>
    )
}

function PlayerControls() {
    const live = useContext(LiveContext);

    let player = live.player.value;
    if (player === null) {
        return null;
    }

    return (
        <>
            <div class={css.playerControls}>
                <button class={css.playerSecondaryControl}>
                    <SkipBackIcon />
                </button>
                <PlayPauseButton player={player} />
                <button class={css.playerSecondaryControl}>
                    <SkipForwardIcon />
                </button>
            </div>
        </>
    )
}

function PlayPauseButton(props: { player: PlayerStatus }) {
    if (props.player.playing) {
        switch (props.player.position.t) {
            case "streaming":
                return (
                    <button class={`${css.playerPrimaryControl}`}>
                        <StopIcon />
                    </button>
                );

            case "elapsed":
                return (
                    <button class={`${css.playerPrimaryControl}`}>
                        <PauseIcon />
                    </button>
                );
        }
    } else {
        return (
            <button class={`${css.playerPrimaryControl} ${css.playButton}`}>
                <PlayIcon />
            </button>
        );
    }
}

export default function() {
    return (
        <div class={css.player}>
            <CurrentTrack />
            <PlayerControls />
        </div>
    );
}
