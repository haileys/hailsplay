import { ReactComponent as PlayIcon } from "feather-icons/dist/icons/play.svg";
import { ReactComponent as StopIcon } from "feather-icons/dist/icons/square.svg";
import { ReactComponent as PauseIcon } from "feather-icons/dist/icons/pause.svg";
import { ReactComponent as SkipBackIcon } from "feather-icons/dist/icons/skip-back.svg";
import { ReactComponent as SkipForwardIcon } from "feather-icons/dist/icons/skip-forward.svg";
import { useContext } from "preact/hooks";

import { LiveContext } from "../socket";
import { PlayState, PlayerStatus } from "../types";
import { post } from "../api";
import { getNextTrack, getPreviousTrack } from "../player";

import css from "./PlayerControls.module.css";
import { LoadingSpinner } from "./LoadingSpinner";

export default function PlayerControls(props: { onChangeTrack: () => void }) {
    const live = useContext(LiveContext);

    let player = live.player.value;
    if (player === null) {
        return null;
    }

    let setOptimisticState = (state: PlayState) => {
        if (live.player.value) {
            live.player.value = { ...live.player.value, state };
        }
    };

    let setOptimisticTrack = (transition: "next" | "previous", getter: typeof getNextTrack) => {
        if (live.player.value === null) {
            return;
        }

        if (live.queue.value === null) {
            return;
        }

        let track = getter(live.player.value, live.queue.value);

        if(track === null) {
            return;
        }

        live.optimisticTrack.value = { transition, track };

        props.onChangeTrack();
    };

    let onPlayAction = async (action: PlayAction) => {
        switch (action) {
            case "play":
                setOptimisticState("loading");
                await post("/api/player/play").response();
                break;
            case "pause":
                setOptimisticState("stopped");
                await post("/api/player/pause").response();
                break;
            case "stop":
                setOptimisticState("stopped");
                await post("/api/player/stop").response();
                break;
        }
    };

    let onSkipNext = async () => {
        setOptimisticState("loading");
        setOptimisticTrack("next", getNextTrack);
        await post("/api/player/skip-next").response();
    };

    let onSkipBack = async() => {
        setOptimisticState("loading");
        setOptimisticTrack("previous", getPreviousTrack);
        await post("/api/player/skip-back").response();
    };

    return (
        <>
            <div class={css.playerControls}>
                <button
                    class={css.playerSecondaryControl}
                    onClick={onSkipBack}
                >
                    <SkipBackIcon />
                </button>

                <PlayPauseButton player={player} onaction={onPlayAction} />

                <button
                    class={css.playerSecondaryControl}
                    onClick={onSkipNext}
                >
                    <SkipForwardIcon />
                </button>
            </div>
        </>
    )
}

export type PlayAction = "play" | "pause" | "stop";

function PlayPauseButton(props: { player: PlayerStatus, onaction: (_: PlayAction) => void }) {
    switch (props.player.state) {
        case "stopped":
            return (
                <button
                    class={`${css.playerPrimaryControl} ${css.playButton}`}
                    onClick={() => props.onaction("play")}
                >
                    <PlayIcon />
                </button>
            );

        case "playing":
            if (props.player.position === null || props.player.position.t === "streaming") {
                return (
                    <button
                        class={`${css.playerPrimaryControl}`}
                        onClick={() => props.onaction("stop")}
                    >
                        <StopIcon />
                    </button>
                );
            } else {
                return (
                    <button
                        class={`${css.playerPrimaryControl}`}
                        onClick={() => props.onaction("pause")}
                    >
                        <PauseIcon />
                    </button>
                );
            }

        case "loading":
            return (
                <button
                    class={`${css.playerPrimaryControl}`}
                    onClick={() => props.onaction("stop")}
                >
                    <LoadingSpinner />
                </button>
            );
    }
}
