import css from "./Player.module.css";

import { LiveContext } from "../socket";
import { useContext, useEffect, useRef, useState } from "preact/hooks";
import PlayerControls from "./PlayerControls";
import { PlayerStatus, Queue, QueueItem, TrackId, TrackInfo } from "../types";
import { Component, Ref, RefObject, createRef } from "preact";

export default function Player() {
    const live = useContext(LiveContext);

    if (live.queue.value === null) {
        return null;
    }

    if (live.player.value == null) {
        return null;
    }

    return (<LoadedPlayer queue={live.queue.value} player={live.player.value} />);
}

type LoadedPlayerProps = { queue: Queue, player: PlayerStatus };

class LoadedPlayer extends Component<LoadedPlayerProps> {
    playerRef: RefObject<HTMLDivElement>;

    constructor(props: LoadedPlayerProps) {
        super(props);
        this.playerRef = createRef();
    }

    render() {
        let currentTrackId = this.props.player.track;
        let { history, queue } = partitionQueue(currentTrackId, this.props.queue.items)

        return (
            <>
                <QueueList items={history} scrollSnapStop={true} />
                <div class={css.player} ref={this.playerRef}>
                    <CurrentTrack />
                    <PlayerControls />
                </div>
                <QueueList items={queue} scrollSnapStop={true} />
            </>
        );
    }

    componentDidMount(): void {
        if (this.playerRef.current === null) {
            throw "playerRef.current is null in LoadedPlayer.componentDidMount";
        }

        this.playerRef.current.scrollIntoView({ behavior: "instant" });
    }
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

function QueueList(props: { items: QueueItem[], scrollSnapStop: boolean }) {
    return (
        <>
            {props.items.map(item => (
                <QueueListItem track={item.track} scrollSnapStop={props.scrollSnapStop} key={item.id} />
            ))}
        </>
    )
}

function QueueListItem(props: { track: TrackInfo, scrollSnapStop: boolean }) {
    let itemClassName = props.scrollSnapStop
        ? `${css.queueItem} ${css.scrollSnapStop}`
        : css.queueItem;

    return (
        <div class={itemClassName}>
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
