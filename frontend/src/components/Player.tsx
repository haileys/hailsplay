import css from "./Player.module.css";

import { LiveContext } from "../socket";
import { useContext } from "preact/hooks";
import PlayerControls from "./PlayerControls";
import { QueueItem, TrackId, TrackInfo } from "../types";

export default function Player() {
    const live = useContext(LiveContext);

    if (live.queue.value === null) {
        return null;
    }

    let currentTrackId = live.player.value && live.player.value.track;
    let { history, queue } = partitionQueue(currentTrackId, live.queue.value.items)

    return (
        <>
            <QueueList items={history} />
            <div class={css.player}>
                <CurrentTrack />
                <PlayerControls />
            </div>
            <QueueList items={queue} />
        </>
    );
}


function partitionQueue(current: TrackId | null, items: QueueItem[]): { history: QueueItem[], queue: QueueItem[] } {
    let i = 0;

    let history = [];
    if (current !== null) {
        for (; i < items.length; i++) {
            if (items[i].id === current) {
                break;
            }

            history.push(items[i]);
        }
    }

    // skip past current track
    i++;

    let queue = [];
    for (; i < items.length; i++) {
        queue.push(items[i]);
    }

    return { history, queue };
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
                    { track.imageUrl ? (
                        <img src={track.imageUrl} class={css.image} />
                    ) : (
                        // TODO - handle no cover art case
                        null
                    ) }
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

function QueueList(props: { items: QueueList[] }) {
    return (
        <div class={css.queueList}>
            {props.items.map(item => (
                <QueueListItem track={item.track} key={item.id} />
            ))}
        </div>
    )
}

function QueueListItem(props: { track: TrackInfo }) {
    return (
        <div class={css.queueItem}>
            <div class={css.queueItemArt}>
                { props.track.imageUrl ? (
                    <img src={props.track.imageUrl} class={css.image} />
                ) : (
                    // TODO - handle no cover art case
                    null
                ) }
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
