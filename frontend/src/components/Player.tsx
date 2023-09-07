import css from "./Player.module.css";

import { LiveContext } from "../socket";
import { useContext } from "preact/hooks";
import PlayerControls from "./PlayerControls";
import { TrackInfo } from "src/types";

export default function Player() {
    return (
        <>
            <div class={css.player}>
                <CurrentTrack />
                <PlayerControls />
            </div>
            <Queue />
        </>
    );
}

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

function Queue() {
    const live = useContext(LiveContext);

    console.log("Queue");

    let queue = live.queue.value;
    if (queue === null) {
        return null;
    }

    console.log("queue:", queue);

    return (
        <div class={css.queueList}>
            {queue.items.map(item => (
                <QueueItem track={item.track} key={item.id} />
            ))}
        </div>
    )
}

function QueueItem(props: { track: TrackInfo }) {
    return (
        <div class={css.queueItem}>
            <div class={css.queueItemArt}>
                <img src={props.track.imageUrl} />
            </div>
            <div class={css.queueItemDetails}>
                <div class={css.queueItemPrimaryLabel}>
                    {props.track.primaryLabel}
                </div>
                <div class={css.queueItemSecondaryLabel}>
                    {props.track.secondaryLabel}
                </div>
            </div>
        </div>
    );
}
